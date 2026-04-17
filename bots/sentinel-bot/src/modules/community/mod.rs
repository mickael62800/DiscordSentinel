//! Module community — panels de roles, auto-roles, sponsorship, temp roles
//! (ex community-bot + roles-bot).

pub mod api_client;
pub mod cooldown;
pub mod exclusive_groups;
pub mod prerequisites;
pub mod roles_panel;
pub mod sponsor;
pub mod sponsorship;
pub mod temp_roles;

use std::sync::Arc;

use serenity::all::{
    CommandInteraction, ComponentInteraction, Context, CreateActionRow, CreateButton,
    CreateCommand, CreateEmbed, CreateInteractionResponse, CreateInteractionResponseMessage,
    CreateMessage,
};
use serenity::model::application::ButtonStyle;
use serenity::model::channel::Message;
use serenity::model::guild::Member;
use serenity::model::id::RoleId;
use serenity::prelude::*;
use tracing::{info, warn};

use sentinel_shared::api_client::BaseApiClient;
use sentinel_shared::embeds::{neutral_embed, success_embed};
use sentinel_shared::heartbeat::ApiClientKey;

use api_client::{ApiClient, RolePanelDetail, SyncRole};
use cooldown::InteractionCooldown;
use sponsorship::SponsorshipTracker;
use temp_roles::TempRoleTracker;

// ── TypeMapKeys ──

pub struct RolesApiKey;
impl TypeMapKey for RolesApiKey {
    type Value = ApiClient;
}

pub struct CooldownKey;
impl TypeMapKey for CooldownKey {
    type Value = Arc<InteractionCooldown>;
}

pub struct TempRoleKey;
impl TypeMapKey for TempRoleKey {
    type Value = TempRoleTracker;
}

pub struct SponsorshipKey;
impl TypeMapKey for SponsorshipKey {
    type Value = SponsorshipTracker;
}

// ── Slash commands ──

// ── Init TypeMapKeys ──

pub fn init_typemap(
    data: &mut serenity::prelude::TypeMap,
    api: &Arc<sentinel_shared::api_client::BaseApiClient>,
    grpc: &Arc<sentinel_shared::grpc_client::SentinelGrpcClient>,
) {
    data.insert::<RolesApiKey>(ApiClient::new(Arc::clone(api), Arc::clone(grpc)));
    data.insert::<CooldownKey>(Arc::new(InteractionCooldown::new()));
    data.insert::<TempRoleKey>(TempRoleTracker::new());
    data.insert::<SponsorshipKey>(SponsorshipTracker::new());
}

pub fn register_commands() -> Vec<CreateCommand> {
    vec![roles_panel::register(), sponsor::register()]
}

pub async fn handle_command(ctx: &Context, command: &CommandInteraction) {
    match command.data.name.as_str() {
        "roles-panel" => roles_panel::handle(ctx, command).await,
        "parrain" => sponsor::handle(ctx, command).await,
        _ => {}
    }
}

// ── Component interactions ──

/// Retourne true si ce custom_id est gere par le module community.
pub fn handles_component(cid: &str) -> bool {
    cid.starts_with("role_")
        || cid.starts_with("sponsor_accept:")
        || cid.starts_with("sponsor_refuse:")
}

pub async fn on_component(ctx: &Context, component: &ComponentInteraction) {
    let cid = component.data.custom_id.as_str();
    if cid.starts_with("role_") {
        handle_role_button(ctx, component).await;
    } else if cid.starts_with("sponsor_accept:") || cid.starts_with("sponsor_refuse:") {
        sponsor::handle_button(ctx, component).await;
    }
}

// ── Event handlers ──

/// Auto-roles quand un nouveau membre rejoint.
pub async fn on_member_add(ctx: &Context, new_member: &Member) {
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
                    if let Err(e) = member.add_role(&ctx_clone.http, RoleId::new(role_id)).await {
                        warn!(error = %e, "Failed to add delayed auto-role");
                    }
                }
            });
        } else if let Ok(role_id) = ar.role_id.parse::<u64>() {
            if let Ok(member) = guild_id.member(&ctx.http, new_member.user.id).await {
                if let Err(e) = member.add_role(&ctx.http, RoleId::new(role_id)).await {
                    warn!(error = %e, "Failed to add auto-role");
                }
            }
        }
    }
}

/// Charge les roles temporaires actifs depuis l'API au demarrage.
pub async fn load_temp_roles(ctx: &Context, guild_ids: &[serenity::model::id::GuildId]) {
    let data = ctx.data.read().await;
    if let (Some(api), Some(tracker)) = (data.get::<RolesApiKey>(), data.get::<TempRoleKey>()) {
        for guild_id in guild_ids {
            let gid = guild_id.to_string();
            match api.list_temp_roles(&gid).await {
                Ok(entries) => {
                    let mut loaded = 0u32;
                    for entry in entries {
                        let g = entry.guild_id.parse::<u64>().unwrap_or(0);
                        let u = entry.user_id.parse::<u64>().unwrap_or(0);
                        let r = entry.role_id.parse::<u64>().unwrap_or(0);
                        if g > 0 && u > 0 && r > 0 {
                            tracker.add_with_expiry_timestamp(g, u, r, &entry.expires_at);
                            loaded += 1;
                        }
                    }
                    if loaded > 0 {
                        info!(guild = %gid, count = loaded, "Roles temporaires recharges");
                    }
                }
                Err(e) => {
                    warn!(error = %e, guild = %gid, "Echec chargement roles temporaires");
                }
            }
        }
    }
}

/// Spawn le background task de nettoyage des roles temporaires expires (60s loop).
pub fn spawn_temp_role_cleanup(ctx: Context) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;

            let data = ctx.data.read().await;
            let Some(tracker) = data.get::<TempRoleKey>() else { continue };

            let expired = tracker.expired();
            for temp in &expired {
                let guild_id = serenity::model::id::GuildId::new(temp.guild_id);
                let user_id = serenity::model::id::UserId::new(temp.user_id);
                let role_id = RoleId::new(temp.role_id);

                if let Ok(member) = guild_id.member(&ctx.http, user_id).await {
                    if member.remove_role(&ctx.http, role_id).await.is_ok() {
                        info!(
                            guild = %temp.guild_id,
                            user = %temp.user_id,
                            role = %temp.role_id,
                            "Role temporaire expire et retire"
                        );
                    }
                }
                tracker.remove(temp.guild_id, temp.user_id, temp.role_id);

                if let Some(api) = data.get::<RolesApiKey>() {
                    api.delete_temp_role(
                        &temp.guild_id.to_string(),
                        &temp.user_id.to_string(),
                        &temp.role_id.to_string(),
                    )
                    .await;
                }
            }
        }
    });
}

/// Synchronise les roles Discord de toutes les guilds vers l'API.
pub async fn sync_all_guild_roles(ctx: &Context) {
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
                member_count: 0,
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

// ── Helpers ──

/// Envoie un panel de roles dans un channel avec des boutons.
pub async fn send_role_panel(
    ctx: &Context,
    channel_id: serenity::model::id::ChannelId,
    panel: &RolePanelDetail,
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

    channel_id
        .send_message(&ctx.http, CreateMessage::new().embed(embed).components(rows))
        .await
}

/// Gere le clic sur un bouton de role (toggle).
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

    // Rate limit anti-spam
    {
        let data = ctx.data.read().await;
        if let Some(cooldown) = data.get::<CooldownKey>() {
            let key = format!("role_{}", role_id);
            if let Some(remaining) = cooldown.check_and_set(component.user.id.get(), &key, 2) {
                let response = CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content(format!(
                            "\u{23f1}\u{fe0f} Calme-toi un peu... attends {remaining}s avant de refaire cette action."
                        ))
                        .ephemeral(true),
                );
                let _ = component.create_response(&ctx.http, response).await;
                return;
            }
        }
    }

    let member = match guild_id.member(&ctx.http, component.user.id).await {
        Ok(m) => m,
        Err(_) => return,
    };

    let role = RoleId::new(role_id);
    let has_role = member.roles.contains(&role);

    let guild_config = {
        let data = ctx.data.read().await;
        if let Some(base) = data.get::<ApiClientKey>() {
            match base.get_guild_config(&guild_id.to_string()).await {
                Ok(c) => c,
                Err(e) => {
                    warn!(error = %e, "Failed to fetch guild config for role button");
                    std::collections::HashMap::new()
                }
            }
        } else {
            std::collections::HashMap::new()
        }
    };

    let embed = if has_role {
        if let Ok(m) = guild_id.member(&ctx.http, component.user.id).await {
            if let Err(e) = m.remove_role(&ctx.http, role).await {
                warn!(error = %e, "Failed to remove role");
            }
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
            if let Err(e) = component.create_response(&ctx.http, response).await {
                warn!(error = %e, "Failed to send prerequisite check response");
            }
            return;
        }

        // Temp roles
        let temp_raw = BaseApiClient::config_or(&guild_config, "temp_roles", "");
        let temp_roles_config = temp_roles::parse_temp_roles(&temp_raw);
        let temp_duration = temp_roles::get_temp_duration(&temp_roles_config, role_id);

        if let Some(duration) = temp_duration {
            let expires_at = (chrono::Utc::now() + chrono::Duration::seconds(duration as i64)).to_rfc3339();
            let api_result = {
                let data = ctx.data.read().await;
                if let Some(api) = data.get::<RolesApiKey>() {
                    api.create_temp_role(
                        &guild_id.to_string(),
                        &component.user.id.to_string(),
                        &role_id.to_string(),
                        &expires_at,
                    ).await
                } else {
                    Err("ApiClient indisponible".to_string())
                }
            };

            if let Err(e) = api_result {
                warn!(error = %e, "Echec persistance temp_role — abort");
                let embed = neutral_embed("Erreur")
                    .description("Impossible d'enregistrer le role temporaire cote serveur.");
                let response = CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new().embed(embed).ephemeral(true),
                );
                let _ = component.create_response(&ctx.http, response).await;
                return;
            }

            let data = ctx.data.read().await;
            if let Some(tracker) = data.get::<TempRoleKey>() {
                tracker.add(guild_id.get(), component.user.id.get(), role_id, duration);
            }
        }

        // Exclusive groups
        let groups_raw = BaseApiClient::config_or(&guild_config, "exclusive_groups", "");
        let groups = exclusive_groups::parse_groups(&groups_raw);
        let conflicts = exclusive_groups::get_conflicting_roles(&groups, role_id);
        if !conflicts.is_empty() {
            if let Ok(m) = guild_id.member(&ctx.http, component.user.id).await {
                for conflict_id in &conflicts {
                    if let Err(e) = m.remove_role(&ctx.http, RoleId::new(*conflict_id)).await {
                        warn!(error = %e, conflict_role = %conflict_id, "Failed to remove conflicting role");
                    }
                }
            }
        }

        // Ajouter le role
        if let Ok(m) = guild_id.member(&ctx.http, component.user.id).await {
            if let Err(e) = m.add_role(&ctx.http, role).await {
                warn!(error = %e, "Failed to add role");
                if temp_duration.is_some() {
                    let data = ctx.data.read().await;
                    if let Some(tracker) = data.get::<TempRoleKey>() {
                        tracker.remove(guild_id.get(), component.user.id.get(), role_id);
                    }
                    if let Some(api) = data.get::<RolesApiKey>() {
                        api.delete_temp_role(
                            &guild_id.to_string(),
                            &component.user.id.to_string(),
                            &role_id.to_string(),
                        ).await;
                    }
                }
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
    if let Err(e) = component.create_response(&ctx.http, response).await {
        warn!(error = %e, "Failed to send role toggle response");
    }
}
