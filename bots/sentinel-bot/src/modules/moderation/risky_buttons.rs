//! Handlers des boutons de confirmation / annulation pour les actions risquees
//! (ban ou mute necessitant une double-validation moderateur).

use serenity::all::{
    ComponentInteraction, Context, CreateInteractionResponse, CreateInteractionResponseMessage,
};
use serenity::model::id::UserId;
use tracing::warn;

use super::{commands, risk_check};

pub(super) async fn handle_risky_confirm(ctx: &Context, component: &ComponentInteraction) {
    let pending_id = match component
        .data
        .custom_id
        .strip_prefix(risk_check::CONFIRM_PREFIX)
    {
        Some(id) => id.to_string(),
        None => return,
    };

    let pending = {
        let data = ctx.data.read().await;
        let store = match data.get::<risk_check::RiskyPendingKey>() {
            Some(s) => s,
            None => return,
        };
        risk_check::purge_expired(store);
        store.remove(&pending_id).map(|(_, p)| p)
    };

    let pending = match pending {
        Some(p) => p,
        None => {
            let response = CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content("Cette confirmation a expire ou n'est plus disponible.")
                    .ephemeral(true),
            );
            if let Err(e) = component.create_response(&ctx.http, response).await {
                warn!(error = %e, "Failed to send risky expired response");
            }
            return;
        }
    };

    let ack = CreateInteractionResponse::UpdateMessage(
        CreateInteractionResponseMessage::new()
            .content(format!(
                "\u{2705} Execution confirmee pour `{}`.",
                pending.target_name
            ))
            .embeds(vec![])
            .components(vec![]),
    );
    if let Err(e) = component.create_response(&ctx.http, ack).await {
        warn!(error = %e, "Failed to ACK risky confirm");
    }

    let guild_id = match pending.guild_id.parse::<u64>() {
        Ok(id) => serenity::model::id::GuildId::new(id),
        Err(_) => return,
    };
    let target_uid = match pending.target_id.parse::<u64>() {
        Ok(id) => id,
        Err(_) => return,
    };
    let target_user = match UserId::new(target_uid).to_user(&ctx.http).await {
        Ok(u) => u,
        Err(e) => {
            warn!(error = %e, "risky confirm: user fetch failed");
            return;
        }
    };

    match pending.kind {
        risk_check::PendingKind::Ban {
            delete_message_days,
            is_permanent,
        } => {
            commands::ban::execute_ban(
                ctx,
                pending.channel_id.clone(),
                pending.moderator_id.clone(),
                pending.moderator_name.clone(),
                guild_id,
                &target_user,
                &pending.reason,
                pending.duration_secs,
                &pending.duration_label,
                is_permanent,
                delete_message_days,
                None,
            )
            .await;
        }
        risk_check::PendingKind::Mute { timeout_secs } => {
            let is_permanent = pending.duration_secs.is_none();
            commands::mute::execute_mute(
                ctx,
                pending.channel_id.clone(),
                pending.moderator_id.clone(),
                pending.moderator_name.clone(),
                guild_id,
                &target_user,
                &pending.reason,
                pending.duration_secs,
                &pending.duration_label,
                is_permanent,
                timeout_secs,
                None,
            )
            .await;
        }
    }
}

pub(super) async fn handle_risky_cancel(ctx: &Context, component: &ComponentInteraction) {
    let pending_id = match component
        .data
        .custom_id
        .strip_prefix(risk_check::CANCEL_PREFIX)
    {
        Some(id) => id.to_string(),
        None => return,
    };

    let removed = {
        let data = ctx.data.read().await;
        if let Some(store) = data.get::<risk_check::RiskyPendingKey>() {
            store.remove(&pending_id).is_some()
        } else {
            false
        }
    };

    let content = if removed {
        "\u{274c} Action annulee."
    } else {
        "Cette confirmation a deja expire."
    };

    let response = CreateInteractionResponse::UpdateMessage(
        CreateInteractionResponseMessage::new()
            .content(content)
            .embeds(vec![])
            .components(vec![]),
    );
    if let Err(e) = component.create_response(&ctx.http, response).await {
        warn!(error = %e, "Failed to send risky cancel response");
    }
}
