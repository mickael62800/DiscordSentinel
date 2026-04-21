pub mod access_control;
pub mod channel_management;
pub mod claim_ownership;
pub mod co_admin;
pub mod queue;
pub mod transfer;

// Re-exports pour les enfants de interactions/ (evite les super::super::)
pub(super) use super::api_client;
pub(super) use super::handlers;
pub(super) use super::{VoiceOwnerMapKey, TextToVoiceMapKey};

use serenity::model::application::ComponentInteraction;
use serenity::model::application::ModalInteraction;
use serenity::model::id::ChannelId;
use serenity::prelude::*;
use tracing::{info, warn};

use super::api_client::{ApiClient, VoiceChannelResponse};

// ── Helpers ──

/// Resout le voice channel associe a une interaction. Depuis la refonte, le
/// panneau admin est poste dans le chat integre du vocal, donc
/// `component.channel_id` = le vocal lui-meme dans la majorite des cas.
///
/// Ordre de resolution :
/// 1. Si `channel_id` est directement un voice owner connu → return it.
/// 2. Sinon, lookup legacy `TextToVoiceMap` (salons pre-refonte).
pub async fn find_voice_from_text(ctx: &Context, channel_id: ChannelId) -> Option<ChannelId> {
    let data = ctx.data.read().await;
    if let Some(owners) = data.get::<VoiceOwnerMapKey>() {
        if owners.contains_key(&channel_id) {
            return Some(channel_id);
        }
    }
    let map = data.get::<TextToVoiceMapKey>()?;
    map.get(&channel_id).map(|e| *e.value())
}

/// Helper that checks admin + returns (voice_channel_id, channel_response).
pub async fn require_admin(
    ctx: &Context,
    component: &ComponentInteraction,
) -> Option<(ChannelId, VoiceChannelResponse)> {
    let channel_id = component.channel_id;
    let user_id = component.user.id;

    let voice_channel_id = match find_voice_from_text(ctx, channel_id).await {
        Some(vc) => vc,
        None => {
            respond_ephemeral(ctx, component, "Ce salon n'est pas lie a un salon vocal temporaire.").await;
            return None;
        }
    };

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
        "btn_hide" | "btn_lock" | "btn_limit" | "btn_rename" | "btn_status" => {
            channel_management::handle(ctx, component).await;
        }
        "select_invite" | "btn_kick" | "select_kick" | "btn_ban" | "select_ban" => {
            access_control::handle(ctx, component).await;
        }
        "btn_coadmin" | "select_coadmin" => {
            co_admin::handle(ctx, component).await;
        }
        "btn_transfer" | "select_transfer" => {
            transfer::handle(ctx, component).await;
        }
        "btn_queue" => {
            queue::handle(ctx, component).await;
        }
        other => {
            if other.starts_with("limit_") {
                channel_management::handle(ctx, component).await;
            } else if other.starts_with("ban_duration_") {
                access_control::handle(ctx, component).await;
            } else if other.starts_with("queue_accept_") || other.starts_with("queue_refuse_") {
                queue::handle(ctx, component).await;
            } else if other.starts_with("btn_claim_ownership_") {
                claim_ownership::handle(ctx, component).await;
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
        "modal_rename" | "modal_status" | "modal_limit" => {
            channel_management::handle_modal(ctx, modal).await;
        }
        other => {
            warn!(custom_id = %other, "Modal inconnue");
        }
    }
}
