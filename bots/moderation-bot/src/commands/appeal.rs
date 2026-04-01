use serenity::all::{
    CommandInteraction, ComponentInteraction, Context, CreateCommand,
    CreateInteractionResponse, CreateInteractionResponseMessage,
};
use tracing::{error, info};

use sentinel_shared::discord_helpers::reply_ephemeral;
use sentinel_shared::heartbeat::ApiClientKey;

pub const APPEAL_PREFIX: &str = "sentinel_mod_appeal_";

pub fn register() -> CreateCommand {
    CreateCommand::new("appeal")
        .description("Contester une sanction recue (cree un ticket automatiquement)")
}

/// /appeal — cree un ticket d'appel de sanction depuis un salon.
pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    let guild_id = match command.guild_id {
        Some(id) => id,
        None => { reply_ephemeral(ctx, command, "Commande serveur uniquement.").await; return; }
    };

    let data = ctx.data.read().await;
    let base = match data.get::<ApiClientKey>() {
        Some(b) => b,
        None => { reply_ephemeral(ctx, command, "Erreur interne.").await; return; }
    };

    // Creer un ticket via l'API
    let req = base
        .client()
        .post(format!("{}/api/tickets", base.base_url()))
        .json(&serde_json::json!({
            "title": format!("Appel de sanction — {}", command.user.name),
            "priority": "medium",
            "author_id": command.user.id.to_string(),
            "author_name": command.user.name,
            "server": guild_id.to_string(),
            "category": "appel_sanction",
            "ticket_type": "appel_sanction",
        }));

    match base.auth(req).send().await {
        Ok(resp) if resp.status().is_success() => {
            reply_ephemeral(
                ctx,
                command,
                "Votre appel de sanction a ete enregistre. Un ticket a ete cree et un moderateur senior va l'examiner.",
            ).await;
            info!(user = %command.user.name, "Appel de sanction cree via /appeal");
        }
        Ok(resp) => {
            let status = resp.status();
            error!(status = %status, "Erreur creation ticket appel");
            reply_ephemeral(ctx, command, "Erreur lors de la creation de l'appel. Reessayez plus tard.").await;
        }
        Err(e) => {
            error!(error = %e, "Erreur reseau creation ticket appel");
            reply_ephemeral(ctx, command, "Erreur reseau. Reessayez plus tard.").await;
        }
    }
}

/// Gere le clic sur le bouton "Contester cette sanction" dans un DM.
pub async fn handle_appeal_button(ctx: &Context, component: &ComponentInteraction) {
    let action_id = match component.data.custom_id.strip_prefix(APPEAL_PREFIX) {
        Some(id) => id,
        None => return,
    };

    let data = ctx.data.read().await;
    let base = match data.get::<ApiClientKey>() {
        Some(b) => b,
        None => return,
    };

    // Trouver le guild_id — chercher dans les guilds du cache
    let mut found_guild = String::new();
    for guild_id in ctx.cache.guilds() {
        if let Ok(member) = guild_id.member(&ctx.http, component.user.id).await {
            let _ = member;
            found_guild = guild_id.to_string();
            break;
        }
    }

    if found_guild.is_empty() {
        let response = CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new()
                .content("Impossible de determiner le serveur. Utilisez `/appeal` dans un salon du serveur.")
                .ephemeral(true),
        );
        component.create_response(&ctx.http, response).await.ok();
        return;
    }

    // Creer un ticket d'appel
    let req = base
        .client()
        .post(format!("{}/api/tickets", base.base_url()))
        .json(&serde_json::json!({
            "title": format!("Appel de sanction — {} (action: {})", component.user.name, &action_id[..8.min(action_id.len())]),
            "priority": "medium",
            "author_id": component.user.id.to_string(),
            "author_name": component.user.name,
            "server": found_guild,
            "category": "appel_sanction",
            "ticket_type": "appel_sanction",
        }));

    match base.auth(req).send().await {
        Ok(resp) if resp.status().is_success() => {
            let response = CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content("Votre appel a ete enregistre. Un ticket a ete cree et un moderateur senior va l'examiner.")
                    .ephemeral(true),
            );
            component.create_response(&ctx.http, response).await.ok();
            info!(user = %component.user.name, action_id = action_id, "Appel de sanction cree via bouton DM");
        }
        _ => {
            let response = CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content("Erreur lors de la creation de l'appel. Utilisez `/appeal` dans le serveur.")
                    .ephemeral(true),
            );
            component.create_response(&ctx.http, response).await.ok();
        }
    }
}

/// Construit un bouton "Contester" pour les DMs de sanction.
#[allow(dead_code)]
pub fn build_appeal_button(action_id: &str) -> serenity::builder::CreateActionRow {
    let button = serenity::builder::CreateButton::new(format!("{}{}", APPEAL_PREFIX, action_id))
        .label("Contester cette sanction")
        .style(serenity::all::ButtonStyle::Secondary);

    serenity::builder::CreateActionRow::Buttons(vec![button])
}

