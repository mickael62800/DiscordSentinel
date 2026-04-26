//! Handler du bouton Tirer la Roue.
//!
//! Flow :
//!   1. Click -> defer ephemeral
//!   2. Call API spin (qui valide le daily + commit la tx + retourne la case)
//!   3. Post message PUBLIC dans le salon courant : embed "spinning"
//!   4. Wait 3-4 secondes (suspense)
//!   5. Edit le message avec le resultat final
//!   6. Acquitte le clic ephemeral

use std::sync::Arc;
use std::time::Duration;

use serenity::all::{
    ComponentInteraction, Context, CreateInteractionResponse, CreateInteractionResponseMessage,
    CreateMessage,
};
use serenity::builder::EditMessage;
use tracing::warn;

use sentinel_shared::api_client::BaseApiClient;
use sentinel_shared::heartbeat::ApiClientKey;

use super::api_client::{self, WheelSpinRequest};
use super::embeds;

/// Duree de l animation suspense en ms.
pub const SPIN_ANIMATION_MS: u64 = 4000;

pub async fn handle_spin(ctx: &Context, component: &ComponentInteraction) {
    let guild_id = match component.guild_id {
        Some(g) => g.to_string(),
        None => return,
    };
    let user_id = component.user.id.to_string();
    let username = component.user.name.clone();

    // Defer ephemeral.
    if let Err(e) = component
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Defer(
                CreateInteractionResponseMessage::new().ephemeral(true),
            ),
        )
        .await
    {
        warn!(error = %e, "Echec defer wheel spin");
        return;
    }

    let base = match get_api_client(ctx).await {
        Some(c) => c,
        None => {
            edit_ephemeral_error(ctx, component, "API indisponible").await;
            return;
        }
    };

    let req = WheelSpinRequest { user_id, username: username.clone() };

    let response = match api_client::spin(&base, &guild_id, &req).await {
        Ok(r) => r,
        Err(e) => {
            let msg = humanize_api_error(&e);
            edit_ephemeral_error(ctx, component, &msg).await;
            return;
        }
    };

    // Post public spinning message.
    let mut sent = match component
        .channel_id
        .send_message(
            &ctx.http,
            CreateMessage::new().embed(embeds::build_spinning_embed(&username)),
        )
        .await
    {
        Ok(m) => m,
        Err(e) => {
            warn!(error = %e, "Echec post spinning wheel");
            return;
        }
    };

    tokio::time::sleep(Duration::from_millis(SPIN_ANIMATION_MS)).await;

    let final_embed = embeds::build_result_embed(&response, &username);
    if let Err(e) = sent
        .edit(&ctx.http, EditMessage::new().embed(final_embed))
        .await
    {
        warn!(error = %e, "Echec edit final wheel");
    }

    // Acquitte le clic ephemeral.
    let edit = serenity::builder::EditInteractionResponse::new()
        .content(format!("\u{1f300} Tu as tire la roue : {} ({})",
            response.case_label, format_payout(response.payout)));
    let _ = component.edit_response(&ctx.http, edit).await;
}

fn format_payout(p: i64) -> String {
    if p > 0 { format!("+{p}c") } else if p < 0 { format!("{p}c") } else { "0c".into() }
}

async fn get_api_client(ctx: &Context) -> Option<Arc<BaseApiClient>> {
    let data = ctx.data.read().await;
    data.get::<ApiClientKey>().map(Arc::clone)
}

async fn edit_ephemeral_error(ctx: &Context, component: &ComponentInteraction, message: &str) {
    let edit = serenity::builder::EditInteractionResponse::new()
        .embed(embeds::build_error_embed(message));
    if let Err(e) = component.edit_response(&ctx.http, edit).await {
        warn!(error = %e, "Echec edit error wheel");
    }
}

pub(crate) fn humanize_api_error(raw: &str) -> String {
    if raw.contains("deja tire") {
        return "Tu as deja tire la Roue aujourd hui. Reviens demain !".to_string();
    }
    if raw.contains("desactive") {
        return "Le module Roue du Destin est desactive sur ce serveur.".to_string();
    }
    "Erreur lors du spin. Reessaie dans quelques instants.".to_string()
}

#[cfg(test)]
#[path = "tests/buttons.rs"]
mod tests;
