//! Handlers des boutons approve / reject pour les actions proposees par un
//! moderateur junior (a valider par un moderateur senior).

use std::time::Instant;

use serenity::all::{
    ComponentInteraction, Context, CreateInteractionResponse, CreateInteractionResponseMessage,
};
use tracing::{error, info, warn};

use crate::shared::heartbeat::ApiClientKey;

use super::{ModerationApiKey, PendingActionsKey, APPROVE_PREFIX, REJECT_PREFIX};

pub(super) async fn handle_approve(ctx: &Context, component: &ComponentInteraction) {
    let pending_id = match component.data.custom_id.strip_prefix(APPROVE_PREFIX) {
        Some(id) => id.to_string(),
        None => return,
    };

    let data = ctx.data.read().await;
    let pending_actions = match data.get::<PendingActionsKey>() {
        Some(p) => p,
        None => return,
    };

    if let Some(guild_id) = component.guild_id {
        if let Ok(member) = guild_id.member(&ctx.http, component.user.id).await {
            #[allow(deprecated)]
            if let Ok(perms) = member.permissions(&ctx.cache) {
                if !perms.moderate_members() {
                    let response = CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new()
                            .content("Tu n'as pas la permission d'approuver des actions.")
                            .ephemeral(true),
                    );
                    let _ = component.create_response(&ctx.http, response).await;
                    return;
                }
            }
        }
    }

    let pending = match pending_actions.remove(&pending_id) {
        Some((_, p)) => p,
        None => {
            let response = CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content("Cette action n'est plus en attente.")
                    .ephemeral(true),
            );
            if let Err(e) = component.create_response(&ctx.http, response).await {
                warn!(error = %e, "Failed to send pending-action-not-found response");
            }
            return;
        }
    };

    let now = Instant::now();
    pending_actions.retain(|_, p| now.duration_since(p.proposed_at).as_secs() < 86400);

    let api = match data.get::<ModerationApiKey>() {
        Some(a) => a,
        None => return,
    };

    match api.log_action(&pending.action).await {
        Ok(_) => {
            api.resolve_pending_action(&pending_id, "approved", &component.user.id.to_string())
                .await;

            if let Some(base) = data.get::<ApiClientKey>() {
                base.publish_event(
                    "pending_action_resolved",
                    serde_json::json!({
                        "version": "1",
                        "action_id": pending_id,
                        "status": "approved",
                        "reviewed_by": component.user.id.to_string(),
                        "action_type": pending.action.action_type,
                        "target_id": pending.action.target_id,
                    }),
                );
            }

            let response = CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new().content(format!(
                    "Action approuvee par <@{}>. {} executee sur <@{}>.",
                    component.user.id, pending.action.action_type, pending.action.target_id
                )),
            );
            if let Err(e) = component.create_response(&ctx.http, response).await {
                warn!(error = %e, "Failed to send approve response");
            }
            info!(
                approver = %component.user.name,
                action = %pending.action.action_type,
                target = %pending.action.target_name,
                "Action apprenti approuvee"
            );
        }
        Err(e) => {
            error!(error = %e, "Erreur execution action approuvee");
        }
    }
}

pub(super) async fn handle_reject(ctx: &Context, component: &ComponentInteraction) {
    let pending_id = match component.data.custom_id.strip_prefix(REJECT_PREFIX) {
        Some(id) => id.to_string(),
        None => return,
    };

    let data = ctx.data.read().await;
    let pending_actions = match data.get::<PendingActionsKey>() {
        Some(p) => p,
        None => return,
    };

    if let Some((_, pending)) = pending_actions.remove(&pending_id) {
        if let Some(api) = data.get::<ModerationApiKey>() {
            api.resolve_pending_action(&pending_id, "rejected", &component.user.id.to_string())
                .await;
        }

        if let Some(base) = data.get::<ApiClientKey>() {
            base.publish_event(
                "pending_action_resolved",
                serde_json::json!({
                    "version": "1",
                    "action_id": pending_id,
                    "status": "rejected",
                    "reviewed_by": component.user.id.to_string(),
                    "action_type": pending.action.action_type,
                    "target_id": pending.action.target_id,
                }),
            );
        }

        let response = CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new().content(format!(
                "Action rejetee par <@{}>. {} sur <@{}> annulee.",
                component.user.id, pending.action.action_type, pending.action.target_id
            )),
        );
        if let Err(e) = component.create_response(&ctx.http, response).await {
            warn!(error = %e, "Failed to send reject response");
        }
        info!(
            rejector = %component.user.name,
            action = %pending.action.action_type,
            target = %pending.action.target_name,
            "Action apprenti rejetee"
        );
    }
}
