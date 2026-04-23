use serenity::all::{
    ComponentInteraction, Context, CreateEmbed, CreateEmbedFooter,
    CreateInteractionResponse, CreateInteractionResponseMessage,
};

use sentinel_shared::discord_helpers::component_reply_ephemeral as respond_ephemeral;

use crate::modules::coude::GameApiKey;
use crate::modules::coude::load_guild_config;

pub const CANCEL_PREFIX: &str = "coude_cancel:";

pub async fn handle(ctx: &Context, component: &ComponentInteraction) {
    let combat_id = component.data.custom_id.trim_start_matches(CANCEL_PREFIX).to_string();

    let data = ctx.data.read().await;
    let api = data.get::<GameApiKey>().unwrap();

    // Recuperer le combat
    let combat = match api.get_combat(&combat_id).await {
        Ok(Some(c)) => c,
        Ok(None) => {
            respond_ephemeral(ctx, component, "Combat introuvable.").await;
            return;
        }
        Err(e) => {
            respond_ephemeral(ctx, component, &format!("Erreur API : {e}")).await;
            return;
        }
    };

    // Seul l'attaquant peut annuler
    if combat.attacker_id != component.user.id.to_string() {
        respond_ephemeral(ctx, component, "Seul l'initiateur du defi peut annuler.").await;
        return;
    }

    // Le combat doit etre en attente
    if combat.status != "pending" {
        respond_ephemeral(ctx, component, "Ce combat n'est plus en attente.").await;
        return;
    }

    let guild_id = combat.guild_id.clone();

    drop(data);
    let config = load_guild_config(ctx, &guild_id).await;
    let penalty_pct = config.cancel_penalty();

    let data = ctx.data.read().await;
    let api = data.get::<GameApiKey>().unwrap();

    let attacker = match api.get_or_create_player(&guild_id, &combat.attacker_id, &combat.attacker_name).await {
        Ok(p) => p,
        Err(e) => {
            respond_ephemeral(ctx, component, &format!("Erreur API : {e}")).await;
            return;
        }
    };

    let penalty = (attacker.coins as f64 * penalty_pct).max(1.0) as i64;
    let penalty_display = (penalty_pct * 100.0) as i32;

    // Annuler le combat — utilise Cancel (gate status='pending') plutot
    // qu Expire (ecrase tout), pour eviter d annuler un combat qui vient
    // d etre accepte par le defenseur en parallele.
    if let Err(e) = api.cancel_combat(&combat_id).await {
        respond_ephemeral(ctx, component, &format!("Erreur annulation : {e}")).await;
        return;
    }

    // Retirer la penalite + comptabiliser dans total_lost
    if let Err(e) = api.record_coins_lost(&guild_id, &combat.attacker_id, penalty).await {
        respond_ephemeral(ctx, component, &format!("Erreur penalite : {e}")).await;
        return;
    }

    // Rembourser les paris
    if let Err(e) = api.refund_bets(&combat_id).await {
        tracing::warn!(error = %e, "Echec API refund_bets");
    }

    let embed = CreateEmbed::new()
        .title("\u{274c} Combat annule !")
        .description(format!(
            "<@{}> a annule son defi contre <@{}>.\n\n\
            **Penalite** : -{} coins ({}%)\n\
            Solde restant : {} coins",
            combat.attacker_id,
            combat.defender_id,
            penalty,
            penalty_display,
            attacker.coins - penalty,
        ))
        .color(0x95A5A6)
        .footer(CreateEmbedFooter::new("Coup de Coude | Sentinel"))
        .timestamp(serenity::model::Timestamp::now());

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

