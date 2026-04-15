use serenity::all::{
    ComponentInteraction, Context, CreateEmbed, CreateEmbedFooter, CreateInteractionResponse,
    CreateInteractionResponseMessage, CreateMessage,
};
use serenity::model::id::ChannelId;

use crate::api_client::Combat;
use crate::GameApiKey;
use crate::handler::load_guild_config;

pub const ACCEPT_PREFIX: &str = "coude_accept:";

pub async fn handle(ctx: &Context, component: &ComponentInteraction) {
    let combat_id = match component.data.custom_id.strip_prefix(ACCEPT_PREFIX) {
        Some(id) => id.to_string(),
        None => return,
    };

    let data = ctx.data.read().await;
    let api = data.get::<GameApiKey>().unwrap();

    let combat_record = match api.get_combat(&combat_id).await {
        Ok(Some(c)) => c,
        Ok(None) => {
            reply_ephemeral(ctx, component, "Combat introuvable.").await;
            return;
        }
        Err(e) => {
            reply_ephemeral(ctx, component, &format!("Erreur API : {e}")).await;
            return;
        }
    };

    // Verifier que le clic vient bien de la guild ou le combat a ete cree.
    // Garde-fou cross-guild si un combat_id fuit et qu'un user de guild B a
    // par hasard le meme Discord id que le defenseur de guild A.
    if let Some(gid) = component.guild_id {
        if gid.to_string() != combat_record.guild_id {
            reply_ephemeral(ctx, component, "Ce combat n'appartient pas a cette guild.").await;
            return;
        }
    }

    // Verifier que c'est bien le defenseur qui clique
    if component.user.id.to_string() != combat_record.defender_id {
        reply_ephemeral(ctx, component, "Seul le defenseur peut accepter le defi !").await;
        return;
    }

    // Verifier le statut
    if combat_record.status != "pending" {
        reply_ephemeral(ctx, component, "Ce combat n'est plus en attente.").await;
        return;
    }

    // Charger la config
    drop(data);
    let config = load_guild_config(ctx, &combat_record.guild_id).await;

    // Verifier l'expiration (configurable, defaut 24h)
    let expire_secs = config.combat_expire_secs() as i64;
    let created = chrono::DateTime::parse_from_rfc3339(&combat_record.created_at)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .unwrap_or_else(|_| chrono::Utc::now());
    let elapsed = chrono::Utc::now()
        .signed_duration_since(created)
        .num_seconds();
    if elapsed > expire_secs {
        let data = ctx.data.read().await;
        let api = data.get::<GameApiKey>().unwrap();
        if let Err(e) = api.expire_combat(&combat_id).await {
            tracing::warn!(error = %e, "Echec API expire_combat");
        }
        let expire_label = if expire_secs >= 3600 { format!("{}h", expire_secs / 3600) } else { format!("{}min", expire_secs / 60) };
        reply_ephemeral(ctx, component, &format!("Ce defi a expire ! ({})", expire_label)).await;
        return;
    }

    let delay_min = config.bet_delay_secs() / 60;

    // Passer le combat en phase "betting" avec le message_id
    let data = ctx.data.read().await;
    let api = data.get::<GameApiKey>().unwrap();
    let message_id = component.message.id.to_string();

    match api.set_combat_betting(&combat_id, &message_id).await {
        Ok(false) => {
            reply_ephemeral(ctx, component, "Ce combat n'est plus en attente.").await;
            return;
        }
        Err(e) => {
            reply_ephemeral(ctx, component, &format!("Erreur API : {e}")).await;
            return;
        }
        Ok(true) => {}
    }

    // Remplacer le message de defi par "Combat accepte, paris ouverts"
    let waiting_embed = CreateEmbed::new()
        .title("\u{270a} Combat accepte !")
        .description(format!(
            "<@{}> a accepte le defi de <@{}> !\n\n\
            \u{1f3b2} **Les paris sont ouverts pendant {} minute(s) !**\n\
            Utilisez `/pari` pour miser sur le vainqueur.\n\n\
            \u{23f3} Le combat sera resolu automatiquement par le serveur...",
            combat_record.defender_id,
            combat_record.attacker_id,
            delay_min,
        ))
        .field("Mise", format!("{} coins", combat_record.mise), true)
        .color(0x3498DB)
        .footer(CreateEmbedFooter::new("Coup de Coude | Sentinel"))
        .timestamp(serenity::model::Timestamp::now());

    if let Err(e) = component
        .create_response(
            &ctx.http,
            CreateInteractionResponse::UpdateMessage(
                CreateInteractionResponseMessage::new()
                    .embed(waiting_embed)
                    .components(vec![]),
            ),
        )
        .await
    {
        tracing::warn!(error = %e, "Echec response Discord");
    }

    // Notification dans le salon notifications
    if let Some(notif_ch) = config.channel_notifications() {
        if let Ok(ch_id) = notif_ch.parse::<u64>() {
            let combat_channel = config.channel_combats().unwrap_or_default();
            let notif_embed = CreateEmbed::new()
                .title("\u{1f3b0} Paris ouverts !")
                .description(format!(
                    "**{}** vs **{}** pour **{} coins** !\n\n\
                    \u{23f3} Paris ouverts pendant **{} minute(s)** !\n\
                    Utilisez `/pari` dans <#{}> pour miser.",
                    combat_record.attacker_name, combat_record.defender_name,
                    combat_record.mise, delay_min, combat_channel,
                ))
                .color(0x57F287)
                .footer(CreateEmbedFooter::new("Coup de Coude | Sentinel"))
                .timestamp(serenity::model::Timestamp::now());

            if let Err(e) = serenity::model::id::ChannelId::new(ch_id)
                .send_message(&ctx.http, CreateMessage::new().embed(notif_embed))
                .await
            {
                tracing::warn!(error = %e, "Echec send_message salon notifications");
            }
        }
    }
}

/// Resoud un combat instantanement et retourne l'embed pret a poster.
///
/// Phase 7 refacto : 450 lignes -> ~15. Toute la logique metier (combat
/// engine, wallet, stats, XP, primes, paris, assurance, chaos) vit
/// maintenant dans l'API via le use case `ResolveCombatNowUseCase` /
/// RPC `CoudeCombatsService.ResolveCombatNow`.
pub async fn resolve_combat_internal(
    ctx: &Context,
    combat_record: &Combat,
    _channel_id: ChannelId,
) -> Option<CreateEmbed> {
    let data = ctx.data.read().await;
    let api = data.get::<GameApiKey>().unwrap();

    let resp = match api.resolve_combat_now(&combat_record.id).await {
        Ok(r) => r,
        Err(e) => {
            tracing::error!(error = %e, combat_id = %combat_record.id, "Echec API resolve_combat_now");
            return None;
        }
    };

    let mut embed = CreateEmbed::new()
        .title(&resp.title)
        .description(&resp.description)
        .color(resp.color)
        .footer(CreateEmbedFooter::new("Coup de Coude | Sentinel"))
        .timestamp(serenity::model::Timestamp::now());

    for f in resp.fields {
        embed = embed.field(f.name, f.value, f.inline);
    }

    Some(embed)
}

async fn reply_ephemeral(ctx: &Context, component: &ComponentInteraction, content: &str) {
    if let Err(e) = component
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content(content)
                    .ephemeral(true),
            ),
        )
        .await
    {
        tracing::warn!(error = %e, "Echec response Discord");
    }
}
