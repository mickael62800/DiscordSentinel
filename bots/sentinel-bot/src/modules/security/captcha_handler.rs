//! Handler des interactions captcha (bouton classique + math).

use serenity::all::{ComponentInteraction, Context};
use serenity::model::id::RoleId;
use tracing::{error, warn};

use sentinel_shared::embeds::{danger_embed, success_embed, warn_embed};
use sentinel_shared::heartbeat::ApiClientKey;

use super::api_client::SecurityEvent;
use super::detectors::captcha;
use super::{CaptchaPendingKey, QuarantineKey, SecurityApiKey, SecurityConfigKey};

/// Gere les interactions captcha (bouton + math).
pub(super) async fn on_component(ctx: &Context, component: &ComponentInteraction) {
    let custom_id = &component.data.custom_id;
    let is_button_captcha = custom_id == captcha::CAPTCHA_BUTTON_ID;
    let is_math_captcha = custom_id.starts_with(captcha::CAPTCHA_MATH_PREFIX);

    if !is_button_captcha && !is_math_captcha {
        return;
    }

    let user_id = component.user.id;

    let data = ctx.data.read().await;
    let (quarantine, env_config, base, sec_api) = match (
        data.get::<QuarantineKey>(),
        data.get::<SecurityConfigKey>(),
        data.get::<ApiClientKey>(),
        data.get::<SecurityApiKey>(),
    ) {
        (Some(q), Some(c), Some(a), Some(s)) => (q, c, a, s),
        _ => {
            error!("TypeMap incomplete pour interaction captcha");
            return;
        }
    };

    // ── Captcha math : verifier la reponse ──
    if is_math_captcha {
        let captcha_pending = match data.get::<CaptchaPendingKey>() {
            Some(p) => p,
            None => {
                error!("CaptchaPendingKey manquant");
                return;
            }
        };

        // Extraire et valider l'index du bouton presse
        let pressed_str = custom_id
            .strip_prefix(captcha::CAPTCHA_MATH_PREFIX)
            .unwrap_or("");
        let pressed_index: usize = match pressed_str.parse::<usize>() {
            Ok(i) if i < 4 => i,
            _ => {
                tracing::warn!(user=%user_id, index=%pressed_str, "Index captcha invalide");
                return;
            }
        };

        // Trouver le guild_id de l'utilisateur en quarantaine
        let mut target_guild = None;
        for gid in ctx.cache.guilds() {
            if quarantine.is_quarantined(gid, user_id) {
                target_guild = Some(gid);
                break;
            }
        }

        let guild_id = match target_guild {
            Some(g) => g,
            None => {
                let embed =
                    warn_embed("\u{26a0}\u{fe0f} Deja verifie").description("Vous n'etes pas en quarantaine.");
                let response = serenity::builder::CreateInteractionResponse::Message(
                    serenity::builder::CreateInteractionResponseMessage::new()
                        .embed(embed)
                        .ephemeral(true),
                );
                if let Err(e) = component.create_response(&ctx.http, response).await {
                    warn!(error = %e, "Failed to send already-verified response");
                }
                return;
            }
        };

        match captcha_pending.verify(guild_id, user_id, pressed_index) {
            Some(true) => {
                // Bonne reponse — liberer
                captcha_pending.remove(guild_id, user_id);

                let guild_config = match base.get_guild_config_for(&guild_id.to_string(), crate::modules::security::MODULE_BOT_NAME).await {
                    Ok(cfg) => cfg,
                    Err(e) => {
                        tracing::warn!(error = %e, guild_id = %guild_id, "Echec chargement config guild");
                        std::collections::HashMap::new()
                    }
                };
                let role_id = guild_config
                    .get("quarantine_role_id")
                    .and_then(|v| v.parse::<u64>().ok())
                    .or(env_config.quarantine_role_id);

                if let Some(role_id) = role_id {
                    quarantine
                        .release_user(ctx, guild_id, user_id, RoleId::new(role_id))
                        .await;
                }

                let event = SecurityEvent {
                    guild_id: guild_id.to_string(),
                    event_type: "captcha_verified".to_string(),
                    severity: "info".to_string(),
                    description: format!(
                        "Utilisateur {} a passe le captcha math",
                        component.user.name
                    ),
                    user_ids: vec![user_id.to_string()],
                };
                if let Err(e) = sec_api.report_event(&event).await {
                    warn!(error = %e, "Failed to report captcha_verified event");
                }

                let embed = success_embed("\u{2705} Verification reussie")
                    .description("Bonne reponse ! Vous avez maintenant acces au serveur.");
                let response = serenity::builder::CreateInteractionResponse::Message(
                    serenity::builder::CreateInteractionResponseMessage::new()
                        .embed(embed)
                        .ephemeral(true),
                );
                if let Err(e) = component.create_response(&ctx.http, response).await {
                    warn!(error = %e, "Failed to send captcha success response");
                }
            }
            Some(false) => {
                // Mauvaise reponse — log pour detection brute-force
                tracing::warn!(guild=%guild_id, user=%user_id, index=%pressed_index, "Echec captcha math");
                let embed = danger_embed("\u{274c} Mauvaise reponse")
                    .description("Ce n'est pas la bonne reponse. Reessayez.");
                let response = serenity::builder::CreateInteractionResponse::Message(
                    serenity::builder::CreateInteractionResponseMessage::new()
                        .embed(embed)
                        .ephemeral(true),
                );
                if let Err(e) = component.create_response(&ctx.http, response).await {
                    warn!(error = %e, "Failed to send captcha failure response");
                }
            }
            None => {
                let embed = warn_embed("\u{26a0}\u{fe0f} Captcha expire")
                    .description("Ce captcha n'est plus valide.");
                let response = serenity::builder::CreateInteractionResponse::Message(
                    serenity::builder::CreateInteractionResponseMessage::new()
                        .embed(embed)
                        .ephemeral(true),
                );
                if let Err(e) = component.create_response(&ctx.http, response).await {
                    warn!(error = %e, "Failed to send captcha expired response");
                }
            }
        }
        return;
    }

    // ── Captcha bouton classique ──
    let mut released = false;

    {
        let cache = &ctx.cache;
        for guild_id in cache.guilds() {
            if !quarantine.is_quarantined(guild_id, user_id) {
                continue;
            }

            let guild_config = match base.get_guild_config_for(&guild_id.to_string(), crate::modules::security::MODULE_BOT_NAME).await {
                Ok(cfg) => cfg,
                Err(e) => {
                    tracing::warn!(error = %e, guild_id = %guild_id, "Echec chargement config guild (captcha bouton)");
                    std::collections::HashMap::new()
                }
            };
            let role_id = guild_config
                .get("quarantine_role_id")
                .and_then(|v| v.parse::<u64>().ok())
                .or(env_config.quarantine_role_id);

            if let Some(role_id) = role_id {
                quarantine
                    .release_user(ctx, guild_id, user_id, RoleId::new(role_id))
                    .await;

                let event = SecurityEvent {
                    guild_id: guild_id.to_string(),
                    event_type: "captcha_verified".to_string(),
                    severity: "info".to_string(),
                    description: format!(
                        "Utilisateur {} a passe le captcha",
                        component.user.name
                    ),
                    user_ids: vec![user_id.to_string()],
                };
                if let Err(e) = sec_api.report_event(&event).await {
                    warn!(error = %e, "Failed to report captcha_verified event");
                }

                released = true;
            }
        }
    }

    let embed = if released {
        success_embed("\u{2705} Verification reussie")
            .description("Vous avez maintenant acces au serveur.")
    } else {
        warn_embed("\u{26a0}\u{fe0f} Deja verifie")
            .description("Vous n'etes pas en quarantaine ou la verification a deja ete effectuee.")
    };

    let response = serenity::builder::CreateInteractionResponse::Message(
        serenity::builder::CreateInteractionResponseMessage::new()
            .embed(embed)
            .ephemeral(true),
    );

    if let Err(e) = component.create_response(&ctx.http, response).await {
        warn!(error = %e, "Failed to send captcha final response");
    }
}
