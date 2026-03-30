use rand::Rng;
use serenity::builder::{CreateActionRow, CreateButton, CreateMessage};
use serenity::model::id::UserId;
use serenity::prelude::*;
use tracing::{error, info, warn};

/// Identifiant du bouton de vérification captcha.
pub const CAPTCHA_BUTTON_ID: &str = "sentinel_captcha_verify";

/// Génère un code captcha simple (6 caractères alphanumériques).
#[allow(dead_code)]
pub fn generate_code() -> String {
    let mut rng = rand::thread_rng();
    (0..6)
        .map(|_| {
            let idx = rng.gen_range(0..36);
            if idx < 10 {
                (b'0' + idx) as char
            } else {
                (b'A' + idx - 10) as char
            }
        })
        .collect()
}

/// Envoie un message de vérification en DM avec un bouton.
/// Le code captcha est encodé dans le custom_id du bouton.
pub async fn send_challenge(ctx: &Context, user_id: UserId, guild_name: &str) -> bool {
    let user = match user_id.to_user(&ctx.http).await {
        Ok(u) => u,
        Err(e) => {
            warn!(error = %e, user_id = %user_id, "Impossible de récupérer l'utilisateur pour captcha");
            return false;
        }
    };

    let dm_channel = match user.create_dm_channel(&ctx.http).await {
        Ok(c) => c,
        Err(e) => {
            warn!(error = %e, user_id = %user_id, "Impossible de créer le DM pour captcha");
            return false;
        }
    };

    let button = CreateButton::new(CAPTCHA_BUTTON_ID)
        .label("✅ Je suis humain — Vérifier")
        .style(serenity::all::ButtonStyle::Success);

    let row = CreateActionRow::Buttons(vec![button]);

    let message = CreateMessage::new()
        .content(format!(
            "**Vérification de sécurité — {}**\n\n\
             Votre compte a été détecté comme potentiellement suspect.\n\
             Cliquez sur le bouton ci-dessous pour confirmer que vous êtes humain.\n\n\
             ⏱️ Vous avez **5 minutes** pour vous vérifier, sinon vous serez expulsé.",
            guild_name
        ))
        .components(vec![row]);

    match dm_channel.send_message(&ctx.http, message).await {
        Ok(_) => {
            info!(user_id = %user_id, "Challenge captcha envoyé en DM");
            true
        }
        Err(e) => {
            error!(error = %e, user_id = %user_id, "Impossible d'envoyer le captcha en DM");
            false
        }
    }
}
