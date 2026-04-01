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
use tracing::{info, warn};

use sentinel_shared::api_client::BaseApiClient;
use sentinel_shared::embeds::{neutral_embed, success_embed};
use sentinel_shared::heartbeat::{ApiClientKey, register_guilds};

use crate::api_client::{ApiClient, SyncRole};
use crate::commands;
use crate::exclusive_groups;
use crate::prerequisites;
use crate::sponsorship::SponsorshipTracker;
use crate::temp_roles::{self, TempRoleTracker};

/// Cle TypeMap pour le client API specifique au community-bot.
pub struct RolesApiKey;
impl TypeMapKey for RolesApiKey {
    type Value = ApiClient;
}

pub struct TempRoleKey;
impl TypeMapKey for TempRoleKey {
    type Value = TempRoleTracker;
}

pub struct SponsorshipKey;
impl TypeMapKey for SponsorshipKey {
    type Value = SponsorshipTracker;
}

pub struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        info!(bot = %ready.user.name, guilds = ready.guilds.len(), "Community bot connecte");
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

        // Note: la sync des roles est geree par roles-bot (pas de duplication)
    }

    // -- Auto-role quand un membre rejoint --

    async fn guild_member_addition(&self, ctx: Context, new_member: Member) {
        let guild_id = new_member.guild_id;
        let data = ctx.data.read().await;
        let api = match data.get::<RolesApiKey>() {
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
                    if let Ok(member) = guild.member(&ctx_clone.http, user).await {
                        let _ = member.add_role(&ctx_clone.http, RoleId::new(role_id)).await;
                    }
                });
            } else {
                if let Ok(role_id) = ar.role_id.parse::<u64>() {
                    if let Ok(member) = guild_id.member(&ctx.http, new_member.user.id).await {
                        let _ = member.add_role(&ctx.http, RoleId::new(role_id)).await;
                    }
                }
            }
        }
    }

    // -- Clic sur un bouton de panel de roles --

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        match interaction {
            Interaction::Command(command) => {
                match command.data.name.as_str() {
                    "roles-panel" => commands::roles_panel::handle(&ctx, &command).await,
                    "parrain" => commands::sponsor::handle(&ctx, &command).await,
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

    // Lire la config guild pour les features avancees
    let guild_config = {
        let data = ctx.data.read().await;
        if let Some(base) = data.get::<ApiClientKey>() {
            base.get_guild_config(&guild_id.to_string()).await.unwrap_or_default()
        } else {
            std::collections::HashMap::new()
        }
    };

    let embed = if has_role {
        if let Ok(m) = guild_id.member(&ctx.http, component.user.id).await {
            let _ = m.remove_role(&ctx.http, role).await;
        }
        neutral_embed("\u{21a9}\u{fe0f} Role retire")
            .description(format!("Le role <@&{}> vous a ete retire.", role_id))
    } else {
        // Verifier les prerequis
        let prereqs_raw = BaseApiClient::config_or(&guild_config, "role_prerequisites", "");
        let prereqs = prerequisites::parse_prerequisites(&prereqs_raw);
        let user_roles: Vec<u64> = member.roles.iter().map(|r| r.get()).collect();
        let joined_days = member.joined_at
            .map(|j| {
                let now = serenity::model::Timestamp::now().unix_timestamp();
                ((now - j.unix_timestamp()) / 86400).max(0) as u64
            })
            .unwrap_or(0);

        if let Err(msg) = prerequisites::check_prerequisites(&prereqs, role_id, &user_roles, joined_days) {
            let embed = neutral_embed("Prerequis non remplis").description(msg);
            let response = CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new().embed(embed).ephemeral(true),
            );
            let _ = component.create_response(&ctx.http, response).await;
            return;
        }

        // Retirer les roles exclusifs en conflit
        let groups_raw = BaseApiClient::config_or(&guild_config, "exclusive_groups", "");
        let groups = exclusive_groups::parse_groups(&groups_raw);
        let conflicts = exclusive_groups::get_conflicting_roles(&groups, role_id);
        if !conflicts.is_empty() {
            if let Ok(m) = guild_id.member(&ctx.http, component.user.id).await {
                for conflict_id in &conflicts {
                    let _ = m.remove_role(&ctx.http, RoleId::new(*conflict_id)).await;
                }
            }
        }

        // Ajouter le role
        if let Ok(m) = guild_id.member(&ctx.http, component.user.id).await {
            let _ = m.add_role(&ctx.http, role).await;
        }

        // Verifier si c'est un role temporaire
        let temp_raw = BaseApiClient::config_or(&guild_config, "temp_roles", "");
        let temp_roles_config = temp_roles::parse_temp_roles(&temp_raw);
        if let Some(duration) = temp_roles::get_temp_duration(&temp_roles_config, role_id) {
            let data = ctx.data.read().await;
            if let Some(tracker) = data.get::<TempRoleKey>() {
                tracker.add(guild_id.get(), component.user.id.get(), role_id, duration);
            }
            // Persister via l'API
            if let Some(api) = data.get::<RolesApiKey>() {
                let expires_at = (chrono::Utc::now() + chrono::Duration::seconds(duration as i64)).to_rfc3339();
                api.create_temp_role(
                    &guild_id.to_string(),
                    &component.user.id.to_string(),
                    &role_id.to_string(),
                    &expires_at,
                ).await;
            }
        }

        let mut desc = format!("Le role <@&{}> vous a ete attribue.", role_id);
        if !conflicts.is_empty() {
            desc.push_str("\n*(roles exclusifs retires automatiquement)*");
        }
        success_embed("\u{2705} Role attribue").description(desc)
    };

    let msg = CreateInteractionResponseMessage::new()
        .embed(embed)
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
