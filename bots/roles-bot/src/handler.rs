use serenity::async_trait;
use serenity::builder::{
    CreateActionRow, CreateButton, CreateEmbed, CreateInteractionResponse,
    CreateInteractionResponseMessage, CreateMessage,
};
use serenity::model::application::{ButtonStyle, ComponentInteraction, Interaction};
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::model::guild::Member;
use serenity::model::id::RoleId;
use serenity::prelude::*;
use tracing::{error, info, warn};

use crate::api_client::ApiClient;
use crate::commands;

pub struct ApiClientKey;
impl TypeMapKey for ApiClientKey {
    type Value = ApiClient;
}

pub struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        info!(bot = %ready.user.name, guilds = ready.guilds.len(), "Roles bot connecte");
        {
            let data = ctx.data.read().await;
            if let Some(api) = data.get::<ApiClientKey>() {
                api.send_log("info", "", "Roles bot demarre");
            }
        }

        // Enregistrer les commandes
        for guild_status in &ready.guilds {
            let guild_id = guild_status.id;
            if let Err(e) = guild_id
                .set_commands(&ctx.http, commands::all())
                .await
            {
                warn!(error = %e, guild = %guild_id, "Erreur enregistrement commandes");
            }
        }

        // Enregistrer les guilds
        let data = ctx.data.read().await;
        if let Some(api) = data.get::<ApiClientKey>() {
            for guild_status in &ready.guilds {
                let guild_id = guild_status.id;
                if let Ok(guild) = guild_id.to_partial_guild(&ctx.http).await {
                    let _ = api.register_guild(
                        &guild_id.to_string(),
                        &guild.name,
                        guild.approximate_member_count.unwrap_or(0) as i32,
                    ).await;
                }
            }
        }
    }

    // ── Auto-role quand un membre rejoint ──

    async fn guild_member_addition(&self, ctx: Context, new_member: Member) {
        let guild_id = new_member.guild_id;
        let data = ctx.data.read().await;
        let api = match data.get::<ApiClientKey>() {
            Some(a) => a,
            None => return,
        };

        let auto_roles = match api.get_auto_roles(&guild_id.to_string()).await {
            Ok(r) => r,
            Err(e) => {
                warn!(error = %e, "Erreur chargement auto-roles");
                return;
            }
        };

        for ar in &auto_roles {
            if !ar.enabled { continue; }

            if ar.delay_secs > 0 {
                let ctx_clone = ctx.clone();
                let guild = guild_id;
                let user = new_member.user.id;
                let role_id: u64 = match ar.role_id.parse() {
                    Ok(r) => r,
                    Err(_) => continue,
                };
                let delay = ar.delay_secs as u64;

                tokio::spawn(async move {
                    tokio::time::sleep(tokio::time::Duration::from_secs(delay)).await;
                    if let Ok(mut member) = guild.member(&ctx_clone.http, user).await {
                        let _ = member.add_role(&ctx_clone.http, RoleId::new(role_id)).await;
                    }
                });
            } else {
                if let Ok(role_id) = ar.role_id.parse::<u64>() {
                    if let Ok(mut member) = guild_id.member(&ctx.http, new_member.user.id).await {
                        let _ = member.add_role(&ctx.http, RoleId::new(role_id)).await;
                    }
                }
            }
        }
    }

    // ── Clic sur un bouton de panel de roles ──

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        match interaction {
            Interaction::Command(command) => {
                match command.data.name.as_str() {
                    "roles-panel" => commands::roles_panel::handle(&ctx, &command).await,
                    _ => {}
                }
            }
            Interaction::Component(component) => {
                if component.data.custom_id.starts_with("role_") {
                    handle_role_button(&ctx, &component).await;
                }
            }
            _ => {}
        }
    }
}

async fn handle_role_button(ctx: &Context, component: &ComponentInteraction) {
    let custom_id = &component.data.custom_id;
    let role_id_str = custom_id.strip_prefix("role_").unwrap_or("");
    let role_id: u64 = match role_id_str.parse() {
        Ok(id) => id,
        Err(_) => return,
    };

    let guild_id = match component.guild_id {
        Some(g) => g,
        None => return,
    };

    let member = match guild_id.member(&ctx.http, component.user.id).await {
        Ok(m) => m,
        Err(_) => return,
    };

    let role = RoleId::new(role_id);
    let has_role = member.roles.contains(&role);

    let message = if has_role {
        if let Ok(mut m) = guild_id.member(&ctx.http, component.user.id).await {
            let _ = m.remove_role(&ctx.http, role).await;
        }
        format!("Role <@&{}> retire !", role_id)
    } else {
        if let Ok(mut m) = guild_id.member(&ctx.http, component.user.id).await {
            let _ = m.add_role(&ctx.http, role).await;
        }
        format!("Role <@&{}> attribue !", role_id)
    };

    let msg = CreateInteractionResponseMessage::new()
        .content(message)
        .ephemeral(true);
    let response = CreateInteractionResponse::Message(msg);
    let _ = component.create_response(&ctx.http, response).await;
}

/// Envoie un panel de roles dans un channel avec des boutons.
pub async fn send_role_panel(
    ctx: &Context,
    channel_id: serenity::model::id::ChannelId,
    panel: &crate::api_client::RolePanelDetail,
) -> Result<Message, serenity::Error> {
    let mut embed = CreateEmbed::new()
        .title(&panel.panel.title)
        .color(0x5865F2);

    if !panel.panel.description.is_empty() {
        embed = embed.description(&panel.panel.description);
    }

    let mut desc_parts = Vec::new();
    for entry in &panel.entries {
        let emoji = entry.emoji.as_deref().unwrap_or("");
        desc_parts.push(format!("{} **{}**", emoji, entry.label));
    }
    if !desc_parts.is_empty() {
        embed = embed.description(desc_parts.join("\n"));
    }

    let buttons: Vec<CreateButton> = panel
        .entries
        .iter()
        .map(|entry| {
            let style = match entry.style.as_str() {
                "secondary" => ButtonStyle::Secondary,
                "success" => ButtonStyle::Success,
                "danger" => ButtonStyle::Danger,
                _ => ButtonStyle::Primary,
            };
            let mut btn = CreateButton::new(format!("role_{}", entry.role_id))
                .label(&entry.label)
                .style(style);
            if let Some(ref emoji) = entry.emoji {
                if let Ok(e) = emoji.parse::<serenity::model::channel::ReactionType>() {
                    btn = btn.emoji(e);
                }
            }
            btn
        })
        .collect();

    let rows: Vec<CreateActionRow> = buttons
        .chunks(5)
        .map(|chunk| CreateActionRow::Buttons(chunk.to_vec()))
        .collect();

    let mut message = CreateMessage::new().embed(embed);
    for row in rows {
        message = message.components(vec![row]);
    }

    channel_id.send_message(&ctx.http, message).await
}
