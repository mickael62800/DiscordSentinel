use serenity::all::{
    ComponentInteraction, Context, CreateEmbed, CreateEmbedFooter,
    CreateInteractionResponse, CreateInteractionResponseMessage,
};

use crate::shared::discord_helpers::component_reply_ephemeral as reply_ephemeral;

use crate::modules::coude::GameApiKey;
use crate::modules::coude::load_guild_config;

pub const REFUSE_PREFIX: &str = "coude_refuse:";

// Messages humiliants migres dans `coude_flavor_templates` (key
// `combat_refused`) — Phase 3 #9 audit. Voir migration 173.

pub async fn handle(ctx: &Context, component: &ComponentInteraction) {
    let combat_id = match component.data.custom_id.strip_prefix(REFUSE_PREFIX) {
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
            reply_ephemeral(ctx, component, &e).await;
            return;
        }
    };

    // Verifier que c'est le defenseur
    if component.user.id.to_string() != combat_record.defender_id {
        reply_ephemeral(ctx, component, "Seul le defenseur peut refuser le defi !").await;
        return;
    }

    if combat_record.status != "pending" {
        reply_ephemeral(ctx, component, "Ce combat n'est plus en attente.").await;
        return;
    }

    // Charger la config guild pour les penalites + le seuil d'expiration.
    let config = load_guild_config(ctx, &combat_record.guild_id).await;

    // Expiration : seuil migre dans `combat_expire_secs` (default 24h).
    let created = chrono::DateTime::parse_from_rfc3339(&combat_record.created_at)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .unwrap_or_else(|_| chrono::Utc::now());
    let elapsed = chrono::Utc::now()
        .signed_duration_since(created)
        .num_seconds();
    if elapsed > config.combat_expire_secs() as i64 {
        // Expire : on annule le combat. Utilise cancel_combat (gate
        // status='pending') pour ne pas ecraser un combat qu un clic
        // concurrent viendrait de faire passer en betting.
        if let Err(e) = api.cancel_combat(&combat_id).await {
            tracing::warn!(error = %e, "Echec API cancel_combat (expire)");
        }
        let hours = config.combat_expire_secs() / 3600;
        reply_ephemeral(ctx, component, &format!("Ce defi a expire ! ({}h)", hours)).await;
        return;
    }

    // Penalite depuis la config
    let penalty = (combat_record.mise as f64 * config.refusal_penalty()).max(1.0) as i64;

    // Tirage du shame_msg cote API (catalogue editable runtime).
    let shame_msg = match api.random_flavor("combat_refused", "fr").await {
        Ok(Some(s)) => s,
        Ok(None) | Err(_) => {
            reply_ephemeral(
                ctx,
                component,
                "API indispo, veuillez reessayer plus tard.",
            )
            .await;
            return;
        }
    };

    let refuse_msg = format!(
        "\u{1f414} <@{}> {} Perte de **{} coins** !",
        combat_record.defender_id, shame_msg, penalty
    );

    if let Err(e) = api
        .resolve_combat(
            &combat_id,
            "refused",
            None,
            None,
            None,
            None,
            Some(&refuse_msg),
            penalty,
        )
        .await
    {
        reply_ephemeral(ctx, component, &e).await;
        return;
    }

    // Retirer les coins et incrementer la lachete
    if let Err(e) = api
        .update_player_coins(&combat_record.guild_id, &combat_record.defender_id, -penalty)
        .await
    {
        tracing::warn!(error = %e, "Echec API update_player_coins refus");
    }
    let cowardice = api
        .increment_cowardice(&combat_record.guild_id, &combat_record.defender_id)
        .await
        .unwrap_or(0);

    // Dette d honneur (cf. COUPE_AMELIORATIONS 5.3) : incremente le
    // compteur par paire (requester=attaquant, refuser=defenseur).
    // Quand il atteint 3, l attaquant pourra invoquer /honneur.
    api.increment_refusal(
        &combat_record.guild_id,
        &combat_record.attacker_id,
        &combat_record.defender_id,
    )
    .await;

    let mut description = refuse_msg;

    if cowardice >= config.cowardice_threshold() {
        let penalty_pct = (config.cowardice_penalty() * 100.0) as i32;
        description.push_str(&format!(
            "\n\n\u{1f414} **<@{}> est un lache notoire !** ({} refus)\nLes laches gagnent {}% de moins en combat.",
            combat_record.defender_id, cowardice, penalty_pct
        ));
    }

    let embed = CreateEmbed::new()
        .title("\u{1f414} Defi refuse !")
        .description(description)
        .color(0xED4245)
        .footer(CreateEmbedFooter::new(crate::shared::branding::COUDE_TAGLINE_SHORT))
        .timestamp(serenity::model::Timestamp::now());

    // Remplacer la card de defi par la card de refus (supprime les boutons)
    if let Err(e) = component
        .create_response(
            &ctx.http,
            CreateInteractionResponse::UpdateMessage(
                CreateInteractionResponseMessage::new()
                    .embed(embed)
                    .components(vec![]),
            ),
        )
        .await
    {
        tracing::warn!(error = %e, "Echec response Discord");
    }
}

