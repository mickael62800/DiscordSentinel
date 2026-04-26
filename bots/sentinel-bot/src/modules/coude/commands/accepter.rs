use serenity::all::{
    ComponentInteraction, Context, CreateEmbed, CreateEmbedFooter, CreateInteractionResponse,
    CreateInteractionResponseFollowup, CreateMessage, EditMessage,
};
use serenity::model::id::ChannelId;

use crate::modules::coude::api_client::Combat;
use crate::modules::coude::GameApiKey;
use crate::modules::coude::load_guild_config;

pub const ACCEPT_PREFIX: &str = "coude_accept:";

pub async fn handle(ctx: &Context, component: &ComponentInteraction) {
    let combat_id = match component.data.custom_id.strip_prefix(ACCEPT_PREFIX) {
        Some(id) => id.to_string(),
        None => return,
    };

    // Defer update message : acknowledge le bouton et garde le message
    // tel quel pendant qu'on fait les API calls (get_combat, expire_combat
    // si besoin, set_combat_betting). Sans ca, Discord coupait apres 3s
    // et affichait "L'interaction a echoue".
    if let Err(e) = component
        .create_response(&ctx.http, CreateInteractionResponse::Acknowledge)
        .await
    {
        tracing::warn!(error = %e, "Echec defer accepter button");
        return;
    }

    let data = ctx.data.read().await;
    let api = data.get::<GameApiKey>().unwrap();

    let combat_record = match api.get_combat(&combat_id).await {
        Ok(Some(c)) => c,
        Ok(None) => {
            followup_ephemeral(ctx, component, "Combat introuvable.").await;
            return;
        }
        Err(e) => {
            followup_ephemeral(ctx, component, &format!("Erreur API : {e}")).await;
            return;
        }
    };

    // Verifier que le clic vient bien de la guild ou le combat a ete cree.
    // Garde-fou cross-guild si un combat_id fuit et qu'un user de guild B a
    // par hasard le meme Discord id que le defenseur de guild A.
    if let Some(gid) = component.guild_id {
        if gid.to_string() != combat_record.guild_id {
            followup_ephemeral(ctx, component, "Ce combat n'appartient pas a cette guild.").await;
            return;
        }
    }

    // Verifier que c'est bien le defenseur qui clique
    if component.user.id.to_string() != combat_record.defender_id {
        followup_ephemeral(ctx, component, "Seul le defenseur peut accepter le defi !").await;
        return;
    }

    // Verifier le statut
    if combat_record.status != "pending" {
        followup_ephemeral(ctx, component, "Ce combat n'est plus en attente.").await;
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
        followup_ephemeral(ctx, component, &format!("Ce defi a expire ! ({})", expire_label)).await;
        return;
    }

    let delay_min = config.bet_delay_secs() / 60;

    // Passer le combat en phase "betting" avec le message_id
    let data = ctx.data.read().await;
    let api = data.get::<GameApiKey>().unwrap();
    let message_id = component.message.id.to_string();

    match api.set_combat_betting(&combat_id, &message_id).await {
        Ok(false) => {
            followup_ephemeral(ctx, component, "Ce combat n'est plus en attente.").await;
            return;
        }
        Err(e) => {
            followup_ephemeral(ctx, component, &format!("Erreur API : {e}")).await;
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
        .footer(CreateEmbedFooter::new(sentinel_shared::branding::COUDE_TAGLINE_SHORT))
        .timestamp(serenity::model::Timestamp::now());

    // Edit du message original : on enleve les boutons et on met le nouveau
    // embed. Comme on a Ack au debut, le message reste intact jusqu'ici.
    // On edit directement via http le message avec son id.
    let edit_msg = serenity::all::EditMessage::new()
        .embed(waiting_embed)
        .components(vec![]);
    if let Err(e) = component
        .channel_id
        .edit_message(&ctx.http, component.message.id, edit_msg)
        .await
    {
        tracing::warn!(error = %e, "Echec edit message defi accepte");
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
                .footer(CreateEmbedFooter::new(sentinel_shared::branding::COUDE_TAGLINE_SHORT))
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
/// Resultat d'une tentative de resolution instantanee cote API.
///
/// Distingue le succes (embed pret a poster) du cas special ou l'API
/// refuse l'auto-resolve parce que le defenseur possede un item de contre
/// (Explosion) et que la regle `surprise_allow_defender_counter` est active.
/// Dans ce dernier cas, l'appelant doit basculer sur le flow de defi normal.
pub enum ResolveOutcome {
    Resolved(CreateEmbed),
    DefenderCanCounter,
    Failed,
}

/// Phase 7 refacto : 450 lignes -> ~15. Toute la logique metier (combat
/// engine, wallet, stats, XP, primes, paris, assurance, chaos) vit
/// maintenant dans l'API via le use case `ResolveCombatNowUseCase` /
/// RPC `CoudeCombatsService.ResolveCombatNow`.
pub async fn resolve_combat_internal(
    ctx: &Context,
    combat_record: &Combat,
    _channel_id: ChannelId,
) -> Option<CreateEmbed> {
    match resolve_combat_internal_ex(ctx, combat_record, _channel_id).await {
        ResolveOutcome::Resolved(e) => Some(e),
        _ => None,
    }
}

/// Variante etendue qui distingue l'echec general du cas "defender_can_counter"
/// (Phase 132+). Utilisee par le flow d'attaque surprise pour basculer sur
/// le flow de defi normal quand l'API refuse l'auto-resolve.
pub async fn resolve_combat_internal_ex(
    ctx: &Context,
    combat_record: &Combat,
    _channel_id: ChannelId,
) -> ResolveOutcome {
    let data = ctx.data.read().await;
    let api = data.get::<GameApiKey>().unwrap();

    let resp = match api.resolve_combat_now(&combat_record.id).await {
        Ok(r) => r,
        Err(e) => {
            // Sentinel : le defenseur possede un item de contre (Explosion),
            // l'API demande au bot de basculer sur le flow normal.
            if e.contains("surprise_defender_can_counter") {
                return ResolveOutcome::DefenderCanCounter;
            }
            tracing::error!(error = %e, combat_id = %combat_record.id, "Echec API resolve_combat_now");
            return ResolveOutcome::Failed;
        }
    };

    let mut embed = CreateEmbed::new()
        .title(&resp.title)
        .description(&resp.description)
        .color(resp.color)
        .footer(CreateEmbedFooter::new(sentinel_shared::branding::COUDE_TAGLINE_SHORT))
        .timestamp(serenity::model::Timestamp::now());

    for f in resp.fields {
        embed = embed.field(f.name, f.value, f.inline);
    }

    // Malediction "Lenteur" (cf. COUPE_AMELIORATIONS 5.1) : si un des deux
    // combattants est sous Slowness, on retarde la diffusion du resultat
    // de 10s. Best-effort : si l API curses casse, on n attend pas.
    let attacker_slow = matches!(
        api.get_active_curse(&combat_record.guild_id, &combat_record.attacker_id).await,
        Ok(Some(c)) if c.kind == "slowness"
    );
    let defender_slow = matches!(
        api.get_active_curse(&combat_record.guild_id, &combat_record.defender_id).await,
        Ok(Some(c)) if c.kind == "slowness"
    );
    if attacker_slow || defender_slow {
        tokio::time::sleep(std::time::Duration::from_secs(10)).await;
    }

    // Phase 9 Part D : dispatch les taunt events (100% IO, zero logique).
    if !resp.taunt_events.is_empty() {
        if let Ok(guild_id) = combat_record.guild_id.parse::<u64>() {
            let gid = serenity::all::GuildId::new(guild_id);
            crate::modules::coude::taunts_dispatch::dispatch_all(ctx, gid, &resp.taunt_events).await;
        }
    }

    // Vendetta echouee (cf. COUPE_AMELIORATIONS 5.3) : rename le gagnant
    // "le Bourreau de @<challenger>" pendant 7 jours. Best-effort.
    if let Some(humil) = &resp.vendetta_humiliation {
        if let (Ok(guild_id), Ok(target_id), Ok(challenger_id)) = (
            combat_record.guild_id.parse::<u64>(),
            humil.target_user_id.parse::<u64>(),
            humil.challenger_user_id.parse::<u64>(),
        ) {
            let gid = serenity::all::GuildId::new(guild_id);
            let target = serenity::all::UserId::new(target_id);
            // Resout le pseudo du challenger pour le suffixe.
            let challenger_name = serenity::all::UserId::new(challenger_id)
                .to_user(&ctx.http)
                .await
                .map(|u| u.name)
                .unwrap_or_else(|_| "challenger".into());
            let suffix = format!(" le Bourreau de {}", challenger_name);
            crate::modules::coude::taunts_dispatch::apply_suffix_to_user(
                ctx, gid, target, &suffix,
            )
            .await;
        }
    }

    ResolveOutcome::Resolved(embed)
}

/// Seuil de mise au-dessus duquel on anime le combat round par round
/// (cf. COUPE_AMELIORATIONS 2.6). En dessous, on poste le resultat
/// directement pour ne pas ralentir les petits combats.
pub const ANIMATED_COMBAT_MISE_THRESHOLD: i64 = 500;

/// Poste un embed de combat avec ou sans animation selon la mise.
/// Pour les grosses mises (>= seuil), affiche d abord 3 phases
/// "buildup" avec 2s de pause chacune avant le resultat final.
///
/// Best-effort : si une etape d edit foire, on log et on continue. Le
/// resultat final est toujours poste si possible.
pub async fn post_combat_embed_animated(
    ctx: &Context,
    channel: ChannelId,
    final_embed: CreateEmbed,
    mise: i64,
) {
    if mise < ANIMATED_COMBAT_MISE_THRESHOLD {
        if let Err(e) = channel
            .send_message(&ctx.http, CreateMessage::new().embed(final_embed))
            .await
        {
            tracing::warn!(error = %e, "Echec post embed combat (direct)");
        }
        return;
    }

    // Phase 1 : placeholder "le combat commence".
    let phase1 = CreateEmbed::new()
        .title("\u{1f941} Le combat commence...")
        .description("Les deux combattants entrent dans l arene. Les paris sont fermes. Le silence se fait.")
        .color(0xF1C40F);
    let msg = match channel
        .send_message(&ctx.http, CreateMessage::new().embed(phase1))
        .await
    {
        Ok(m) => m,
        Err(e) => {
            tracing::warn!(error = %e, "Echec post embed combat (animation)");
            return;
        }
    };

    // Phase 2.
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    let phase2 = CreateEmbed::new()
        .title("\u{2694}\u{fe0f} Round 1 — les coups pleuvent...")
        .description("Premiers echanges. Les d20 roulent dans le bruit.")
        .color(0xE67E22);
    let _ = channel
        .edit_message(&ctx.http, msg.id, EditMessage::new().embed(phase2))
        .await;

    // Phase 3.
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    let phase3 = CreateEmbed::new()
        .title("\u{1f525} Round suivant — ca chauffe...")
        .description("Le tempo s emballe. La foule retient son souffle.")
        .color(0xE74C3C);
    let _ = channel
        .edit_message(&ctx.http, msg.id, EditMessage::new().embed(phase3))
        .await;

    // Phase 4 : verdict final.
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    if let Err(e) = channel
        .edit_message(&ctx.http, msg.id, EditMessage::new().embed(final_embed))
        .await
    {
        tracing::warn!(error = %e, "Echec edit final combat anime");
    }
}

/// Followup ephemeral apres un Acknowledge (on a defer au debut).
async fn followup_ephemeral(ctx: &Context, component: &ComponentInteraction, content: &str) {
    if let Err(e) = component
        .create_followup(
            &ctx.http,
            CreateInteractionResponseFollowup::new()
                .content(content)
                .ephemeral(true),
        )
        .await
    {
        tracing::warn!(error = %e, "Echec followup Discord accepter");
    }
}
