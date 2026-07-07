use serenity::all::{
    ComponentInteraction, Context, CreateEmbed, CreateEmbedFooter, CreateInteractionResponse,
    CreateInteractionResponseMessage,
};

use crate::shared::discord_helpers::component_reply_ephemeral as respond_ephemeral;

use crate::modules::coude::GameApiKey;

pub const CANCEL_PREFIX: &str = "coude_cancel:";

pub async fn handle(ctx: &Context, component: &ComponentInteraction) {
    let combat_id = component
        .data
        .custom_id
        .trim_start_matches(CANCEL_PREFIX)
        .to_string();

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
            respond_ephemeral(ctx, component, &e).await;
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

    // Annuler le combat — utilise Cancel (gate status='pending') plutot
    // qu Expire (ecrase tout), pour eviter d annuler un combat qui vient
    // d etre accepte par le defenseur en parallele.
    if let Err(e) = api.cancel_combat(&combat_id).await {
        respond_ephemeral(ctx, component, &format!("Erreur annulation : {e}")).await;
        return;
    }

    // Penalite calculee ET debitee server-side de facon atomique (le bot ne
    // calcule plus le barème ni ne pilote le debit ; corrige aussi le bug ou
    // `record_coins_lost` ne debitait pas reellement le wallet).
    let penalty_outcome = match api.apply_cancel_penalty(&guild_id, &combat.attacker_id).await {
        Ok(o) => o,
        Err(e) => {
            respond_ephemeral(ctx, component, &format!("Erreur penalite : {e}")).await;
            return;
        }
    };

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
            penalty_outcome.penalty,
            penalty_outcome.penalty_percent,
            penalty_outcome.new_balance,
        ))
        .color(0x95A5A6)
        .footer(CreateEmbedFooter::new(
            crate::shared::branding::COUDE_TAGLINE_SHORT,
        ))
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
