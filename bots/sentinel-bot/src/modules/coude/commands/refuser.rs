use serenity::all::{
    ComponentInteraction, Context, CreateEmbed, CreateEmbedFooter,
    CreateInteractionResponse, CreateInteractionResponseMessage,
};

use crate::modules::coude::GameApiKey;
use crate::modules::coude::load_guild_config;

pub const REFUSE_PREFIX: &str = "coude_refuse:";

/// Messages humiliants aleatoires pour les laches.
const SHAME_MESSAGES: &[&str] = &[
    "a fui comme un poulet sans tete !",
    "a prefere se cacher sous la table...",
    "a tremble de peur et s'est enfui !",
    "a fait pipi dans son pantalon !",
    "a pleure en appelant sa maman !",
    "a couru si vite qu'il a perdu ses chaussures !",
    "a fait semblant d'avoir un rendez-vous urgent...",
    "s'est cache derriere un buisson !",
    "a invente une excuse bidon pour fuir !",
    "a declare forfait avant meme de commencer !",
];

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
            reply_ephemeral(ctx, component, &format!("Erreur API : {e}")).await;
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

    // Expiration (24 heures)
    let created = chrono::DateTime::parse_from_rfc3339(&combat_record.created_at)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .unwrap_or_else(|_| chrono::Utc::now());
    let elapsed = chrono::Utc::now()
        .signed_duration_since(created)
        .num_seconds();
    if elapsed > 86400 {
        if let Err(e) = api.expire_combat(&combat_id).await {
            tracing::warn!(error = %e, "Echec API expire_combat");
        }
        reply_ephemeral(ctx, component, "Ce defi a expire ! (24h)").await;
        return;
    }

    // Charger la config guild pour les penalites
    let config = load_guild_config(ctx, &combat_record.guild_id).await;

    // Penalite depuis la config
    let penalty = (combat_record.mise as f64 * config.refusal_penalty()).max(1.0) as i64;

    // Mettre a jour le combat
    let shame_msg = {
        use rand::Rng;
        let idx = rand::thread_rng().gen_range(0..SHAME_MESSAGES.len());
        SHAME_MESSAGES[idx]
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
        reply_ephemeral(ctx, component, &format!("Erreur API : {e}")).await;
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
        .footer(CreateEmbedFooter::new("Coup de Coude | Sentinel"))
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
