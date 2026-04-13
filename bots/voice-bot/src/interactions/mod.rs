pub mod access_control;
pub mod channel_management;
pub mod co_admin;
pub mod queue;
pub mod transfer;
pub mod vote_kick;

use serenity::model::application::ComponentInteraction;
use serenity::model::application::ModalInteraction;
use serenity::model::id::ChannelId;
use serenity::prelude::*;
use tracing::{info, warn};

use crate::api_client::{ApiClient, VoiceChannelResponse};
use crate::handler::{TextToVoiceMapKey, MembersToVoiceMapKey};

// ── Helpers ──

/// Find the voice channel ID associated with a text panel channel.
pub async fn find_voice_from_text(ctx: &Context, text_channel_id: ChannelId) -> Option<ChannelId> {
    let data = ctx.data.read().await;
    let map = data.get::<TextToVoiceMapKey>()?;
    map.get(&text_channel_id).map(|e| *e.value())
}

/// Find the voice channel ID associated with a members panel channel.
pub async fn find_voice_from_members(ctx: &Context, members_channel_id: ChannelId) -> Option<ChannelId> {
    let data = ctx.data.read().await;
    let map = data.get::<MembersToVoiceMapKey>()?;
    map.get(&members_channel_id).map(|e| *e.value())
}

/// Helper that checks admin + returns (voice_channel_id, channel_response).
/// Sends an ephemeral error if the user is not admin or channel is not found.
/// Returns None if the check fails (response already sent).
pub async fn require_admin(
    ctx: &Context,
    component: &ComponentInteraction,
) -> Option<(ChannelId, VoiceChannelResponse)> {
    let text_channel_id = component.channel_id;
    let user_id = component.user.id;

    // Try finding voice channel from text panel first, then from members panel
    let voice_channel_id = if let Some(vc) = find_voice_from_text(ctx, text_channel_id).await {
        vc
    } else if let Some(vc) = find_voice_from_members(ctx, text_channel_id).await {
        vc
    } else {
        respond_ephemeral(ctx, component, "Ce salon n'est pas lie a un salon vocal temporaire.").await;
        return None;
    };

    // Fetch channel info from API
    let channel_resp = {
        let data = ctx.data.read().await;
        let Some(api) = ApiClient::from_data(&data) else {
            warn!("ApiClient ou GrpcClient manquants dans TypeMap");
            respond_ephemeral(ctx, component, "Erreur interne (client API indisponible).").await;
            return None;
        };
        api.get_channel(&voice_channel_id.get().to_string()).await
    };

    let ch = match channel_resp {
        Ok(Some(ch)) => ch,
        Ok(None) => {
            respond_ephemeral(ctx, component, "Ce salon vocal n'existe plus dans la base.").await;
            return None;
        }
        Err(e) => {
            warn!(error = %e, "Erreur API get_channel dans require_admin");
            respond_ephemeral(ctx, component, "Erreur lors de la verification des droits.").await;
            return None;
        }
    };

    if ch.owner_id != user_id.get().to_string() {
        respond_ephemeral(ctx, component, "Seul le proprietaire du salon peut effectuer cette action.").await;
        return None;
    }

    Some((voice_channel_id, ch))
}

/// Send an ephemeral response to a component interaction.
pub async fn respond_ephemeral(
    ctx: &Context,
    component: &ComponentInteraction,
    content: &str,
) {
    use serenity::builder::CreateInteractionResponse;
    use serenity::builder::CreateInteractionResponseMessage;

    let msg = CreateInteractionResponseMessage::new()
        .content(content)
        .ephemeral(true);

    let response = CreateInteractionResponse::Message(msg);

    if let Err(e) = component.create_response(&ctx.http, response).await {
        warn!(error = %e, "Erreur envoi reponse ephemere");
    }
}

/// Send an ephemeral response to a modal interaction.
pub async fn respond_ephemeral_modal(
    ctx: &Context,
    modal: &ModalInteraction,
    content: &str,
) {
    use serenity::builder::CreateInteractionResponse;
    use serenity::builder::CreateInteractionResponseMessage;

    let msg = CreateInteractionResponseMessage::new()
        .content(content)
        .ephemeral(true);

    let response = CreateInteractionResponse::Message(msg);

    if let Err(e) = modal.create_response(&ctx.http, response).await {
        warn!(error = %e, "Erreur envoi reponse ephemere modal");
    }
}

// ── Dispatch ──

/// Dispatch a component interaction to the appropriate sub-handler.
pub async fn handle_component(ctx: &Context, component: &ComponentInteraction) {
    let custom_id = component.data.custom_id.as_str();

    info!(custom_id = %custom_id, user = %component.user.id, "Component interaction");

    match custom_id {
        // Channel management
        "btn_hide" | "btn_lock" | "btn_limit" | "btn_rename" | "btn_status" => {
            channel_management::handle(ctx, component).await;
        }

        // Access control
        "select_invite" | "btn_kick" | "select_kick" | "btn_ban" | "select_ban" => {
            access_control::handle(ctx, component).await;
        }

        // Co-admin
        "btn_coadmin" | "select_coadmin" => {
            co_admin::handle(ctx, component).await;
        }

        // Transfer
        "btn_transfer" | "select_transfer" => {
            transfer::handle(ctx, component).await;
        }

        // Queue
        "btn_queue" => {
            queue::handle(ctx, component).await;
        }

        // Vote kick
        "select_votekick" | "votekick_yes" | "votekick_no" => {
            vote_kick::handle(ctx, component).await;
        }

        other => {
            // Check prefix-based routing
            if other.starts_with("limit_") {
                channel_management::handle(ctx, component).await;
            } else if other.starts_with("ban_duration_") {
                access_control::handle(ctx, component).await;
            } else if other.starts_with("queue_accept_") || other.starts_with("queue_refuse_") {
                queue::handle(ctx, component).await;
            } else {
                warn!(custom_id = %other, "Interaction inconnue");
            }
        }
    }
}

/// Dispatch a modal interaction to the appropriate sub-handler.
pub async fn handle_modal(ctx: &Context, modal: &ModalInteraction) {
    let custom_id = modal.data.custom_id.as_str();

    info!(custom_id = %custom_id, user = %modal.user.id, "Modal interaction");

    match custom_id {
        "modal_rename" | "modal_status" => {
            channel_management::handle_modal(ctx, modal).await;
        }
        other => {
            warn!(custom_id = %other, "Modal inconnue");
        }
    }
}
