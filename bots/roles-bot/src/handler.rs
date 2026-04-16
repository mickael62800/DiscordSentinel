use serenity::async_trait;
use serenity::builder::{CreateActionRow, CreateButton, CreateEmbed, CreateMessage};
use serenity::model::application::{ButtonStyle, Interaction};
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::prelude::*;
use tracing::{info, warn};

use sentinel_shared::heartbeat::register_guilds;

use crate::api_client::{ApiClient, SyncRole};
use crate::commands;

/// Cle TypeMap pour le client API specifique au roles-bot.
pub struct RolesApiKey;
impl TypeMapKey for RolesApiKey {
    type Value = ApiClient;
}

pub struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        info!(bot = %ready.user.name, guilds = ready.guilds.len(), "Roles bot connecte");
        register_guilds(&ctx, &ready).await;

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

        // Sync initiale des roles Discord vers l'API
        sync_all_guild_roles(&ctx).await;

        // Sync periodique (configurable via env, defaut 300s = 5 min).
        let sync_interval = std::env::var("ROLES_SYNC_INTERVAL_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(300u64);
        let ctx_clone = ctx.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(sync_interval)).await;
                sync_all_guild_roles(&ctx_clone).await;
            }
        });
    }

    // Note : guild_member_addition (auto-roles) et les clics sur les
    // boutons `role_*` sont geres par community-bot, qui a la version
    // feature-rich (prerequis, exclusive_groups, temp_roles). Roles-bot
    // se contente de deployer les panels (/roles-panel deploy) et de
    // synchroniser les roles Discord vers le backend API.

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::Command(command) = interaction {
            if command.data.name.as_str() == "roles-panel" {
                commands::roles_panel::handle(&ctx, &command).await;
            }
        }
    }
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

    // Bug historique : la boucle precedente appelait `.components(vec![row])`
    // a chaque iteration, ce qui REMPLACE la liste a chaque fois. Resultat :
    // un panel avec plus de 5 boutons n'affichait que la derniere rangee.
    let message = CreateMessage::new().embed(embed).components(rows);

    channel_id.send_message(&ctx.http, message).await
}

/// Synchronise les roles Discord de toutes les guilds vers l'API backend.
async fn sync_all_guild_roles(ctx: &Context) {
    let data = ctx.data.read().await;
    let api = match data.get::<RolesApiKey>() {
        Some(a) => a,
        None => return,
    };

    let guilds = ctx.cache.guilds();
    for guild_id in guilds {
        let roles = match guild_id.roles(&ctx.http).await {
            Ok(r) => r,
            Err(e) => {
                warn!(error = %e, guild = %guild_id, "Erreur recuperation roles Discord");
                continue;
            }
        };

        let sync_roles: Vec<SyncRole> = roles
            .values()
            .map(|r| SyncRole {
                id: r.id.to_string(),
                name: r.name.clone(),
                color: r.colour.0 as i32,
                position: r.position as i32,
                permissions: r.permissions.bits().to_string(),
                mentionable: r.mentionable,
                managed: r.managed,
                icon: r.icon.as_ref().map(|i| i.to_string()),
                member_count: 0, // Discord ne fournit pas ce chiffre via l'API roles
            })
            .collect();

        let count = sync_roles.len();
        if let Err(e) = api.sync_discord_roles(&guild_id.to_string(), sync_roles).await {
            warn!(error = %e, guild = %guild_id, "Erreur sync roles vers API");
        } else {
            info!(guild = %guild_id, roles = count, "Roles Discord synchronises");
        }
    }
}
