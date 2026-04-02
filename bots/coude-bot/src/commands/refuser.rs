use serenity::all::{
    ComponentInteraction, Context, CreateEmbed, CreateEmbedFooter,
    CreateInteractionResponse, CreateInteractionResponseMessage,
};
use uuid::Uuid;

use crate::handler::GameDbKey;

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
    let combat_id_str = match component.data.custom_id.strip_prefix(REFUSE_PREFIX) {
        Some(id) => id,
        None => return,
    };

    let combat_id = match Uuid::parse_str(combat_id_str) {
        Ok(id) => id,
        Err(_) => {
            reply_ephemeral(ctx, component, "ID de combat invalide.").await;
            return;
        }
    };

    let data = ctx.data.read().await;
    let db = data.get::<GameDbKey>().unwrap();

    let combat_record = match db.get_combat(combat_id).await {
        Ok(Some(c)) => c,
        Ok(None) => {
            reply_ephemeral(ctx, component, "Combat introuvable.").await;
            return;
        }
        Err(e) => {
            reply_ephemeral(ctx, component, &format!("Erreur DB : {e}")).await;
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
    let elapsed = chrono::Utc::now()
        .signed_duration_since(combat_record.created_at)
        .num_seconds();
    if elapsed > 86400 {
        let _ = db.expire_combat(combat_id).await;
        reply_ephemeral(ctx, component, "Ce defi a expire ! (24h)").await;
        return;
    }

    // Penalite : 20% de la mise
    let penalty = (combat_record.mise as f64 * 0.20).max(1.0) as i64;

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

    if let Err(e) = db
        .resolve_combat(
            combat_id,
            "refused",
            None,
            None,
            None,
            None,
            &refuse_msg,
            penalty,
        )
        .await
    {
        reply_ephemeral(ctx, component, &format!("Erreur DB : {e}")).await;
        return;
    }

    // Retirer les coins et incrementer la lachete
    let _ = db
        .update_player_coins(&combat_record.guild_id, &combat_record.defender_id, -penalty)
        .await;
    let cowardice = db
        .increment_cowardice(&combat_record.guild_id, &combat_record.defender_id)
        .await
        .unwrap_or(0);

    let mut description = refuse_msg;

    if cowardice >= 5 {
        description.push_str(&format!(
            "\n\n\u{1f414} **<@{}> est un lache notoire !** ({} refus)\nLes laches gagnent 20% de moins en combat.",
            combat_record.defender_id, cowardice
        ));
    }

    let embed = CreateEmbed::new()
        .title("\u{1f414} Defi refuse !")
        .description(description)
        .color(0xED4245)
        .footer(CreateEmbedFooter::new("Coup de Coude | Sentinel"))
        .timestamp(serenity::model::Timestamp::now());

    // Remplacer la card de defi par la card de refus (supprime les boutons)
    component
        .create_response(
            &ctx.http,
            CreateInteractionResponse::UpdateMessage(
                CreateInteractionResponseMessage::new()
                    .embed(embed)
                    .components(vec![]),
            ),
        )
        .await
        .ok();
}

async fn reply_ephemeral(ctx: &Context, component: &ComponentInteraction, content: &str) {
    component
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content(content)
                    .ephemeral(true),
            ),
        )
        .await
        .ok();
}
