//! Commande /braquage (Phase 10).
//!
//! Permet de tenter un gros coup sur la caisse communautaire, 1 fois
//! par semaine. Base 5 % de chance, +5 % par item consommable present
//! dans l'inventaire (cap 50 %). Succes : gain 30-75 % de la caisse.
//! Echec : prison 24 h (blocage total du gameplay).
//!
//! Toute la logique metier vit cote API. Cette commande defere
//! l'interaction, appelle AttemptHeist, et affiche le resultat.

use serenity::all::{
    CommandInteraction, Context, CreateCommand, CreateEmbed, CreateEmbedFooter,
};

use crate::shared::discord_helpers::require_guild_id;

use crate::modules::coude::load_guild_config;
use crate::modules::coude::GameApiKey;

// Templates HEIST_SUCCESS / HEIST_FAIL migres dans `coude_flavor_templates`
// (Phase 3 #9 audit). Le bot consomme via `api.random_flavor`. Pas de
// fallback local — si l'API est indispo on affiche un message d'erreur.

fn format_heist(template: &str, user: &str, montant: i64, chance: u32) -> String {
    template
        .replace("{user}", user)
        .replace("{montant}", &montant.to_string())
        .replace("{chance}", &chance.to_string())
}

pub fn register() -> CreateCommand {
    CreateCommand::new("braquage")
        .description("Tente de braquer la caisse communautaire (1x par semaine, gros risque !)")
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    let Some(guild_id) = require_guild_id(ctx, command).await else { return; };

    let config = load_guild_config(ctx, &guild_id).await;
    if !config.enabled() {
        simple_reply_ephemeral(
            ctx,
            command,
            "Le jeu Coup de Coude est desactive sur ce serveur.",
        )
        .await;
        return;
    }
    if !crate::modules::coude::channel_check::check_channel(ctx, command, config.channel_activites()).await {
        return;
    }

    // Defer : attempt_heist cote API enchaine list_inventory + get_cashbox
    // + withdraw + credit + record_attempt + (eventuellement) send_to_prison.
    // Defer public parce que le resultat est visible a tous (c'est un evt
    // du jeu, pas secret comme /protection).
    if !crate::modules::coude::interaction_helper::defer_response(ctx, command).await {
        return;
    }

    let user_id = command.user.id.to_string();

    let data = ctx.data.read().await;
    let api = match data.get::<GameApiKey>() {
        Some(a) => a,
        None => return,
    };

    match api.attempt_heist(&guild_id, &user_id).await {
        Ok(result) => {
            let flavor_key = if result.success { "heist_success" } else { "heist_fail" };
            let flavor_template = match api.random_flavor(flavor_key, "fr").await {
                Ok(Some(s)) => s,
                Ok(None) | Err(_) => {
                    crate::modules::coude::interaction_helper::followup_text(
                        ctx,
                        command,
                        "API indispo, veuillez reessayer plus tard.",
                    )
                    .await;
                    return;
                }
            };
            let embed = build_result_embed(
                &command.user.id.to_string(),
                &result,
                &flavor_template,
            );
            crate::modules::coude::interaction_helper::followup_embed(ctx, command, embed).await;
        }
        Err(e) => {
            // L'API rejette avec DomainError::Forbidden si :
            // - en prison
            // - cooldown non ecoule
            // - caisse vide
            crate::modules::coude::interaction_helper::followup_text(ctx, command, &format!("\u{1f6ab} {e}")).await;
        }
    }
}

fn build_result_embed(
    user_id: &str,
    r: &crate::modules::coude::api_client::HeistResult,
    flavor_template: &str,
) -> CreateEmbed {
    let user_mention = format!("<@{}>", user_id);
    if r.success {
        let tools_line = if r.tools_consumed.is_empty() {
            "Aucun outil utilise".to_string()
        } else {
            format!("{} outils consommes", r.tools_consumed.len())
        };
        let flavor = format_heist(
            flavor_template,
            &user_mention,
            r.amount_stolen,
            r.chance_percent,
        );
        CreateEmbed::new()
            .title("\u{1f4b0} BRAQUAGE REUSSI !")
            .description(format!(
                "{}\n\n\
                 \u{1fa99} **+{} coins** empoches\n\
                 \u{1f3b2} Chance : **{} %**\n\
                 \u{1f6e0}\u{fe0f} {}\n\n\
                 _La caisse etait a {} coins avant le braquage._",
                flavor,
                r.amount_stolen,
                r.chance_percent,
                tools_line,
                r.cashbox_total_before
            ))
            .color(0xFFD700)
            .footer(CreateEmbedFooter::new(format!(
                "{} — Braquage hebdomadaire",
                crate::shared::branding::COUDE_TAGLINE_SHORT,
            )))
            .timestamp(serenity::model::Timestamp::now())
    } else {
        let prison_msg = r
            .prison_released_at
            .as_deref()
            .and_then(|ts| ts.split(&[' ', 'T'][..]).next())
            .map(|d| format!("\n\u{26d3}\u{fe0f} **EN PRISON** jusqu'au **{}** — aucune action de jeu possible !", d))
            .unwrap_or_default();

        let tools_line = if r.tools_consumed.is_empty() {
            "Aucun outil utilise".to_string()
        } else {
            format!("{} outils perdus", r.tools_consumed.len())
        };

        let flavor = format_heist(
            flavor_template,
            &user_mention,
            r.amount_stolen,
            r.chance_percent,
        );
        CreateEmbed::new()
            .title("\u{1f6a8} BRAQUAGE RATE !")
            .description(format!(
                "{}\n\n\
                 \u{1f3b2} Chance : **{} %**\n\
                 \u{1f6e0}\u{fe0f} {}\
                 {}",
                flavor, r.chance_percent, tools_line, prison_msg
            ))
            .color(0xE74C3C)
            .footer(CreateEmbedFooter::new(format!(
                "{} — Retour dans 1 semaine minimum",
                crate::shared::branding::COUDE_TAGLINE_SHORT,
            )))
            .timestamp(serenity::model::Timestamp::now())
    }
}

/// Reply ephemeral simple avant defer, pour les early returns (wrong
/// channel, disabled, etc.). Utilise `create_response` classique.
async fn simple_reply_ephemeral(ctx: &Context, command: &CommandInteraction, content: &str) {
    use serenity::all::{CreateInteractionResponse, CreateInteractionResponseMessage};
    if let Err(e) = command
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
        tracing::warn!(error = %e, "Echec response Discord braquage");
    }
}
