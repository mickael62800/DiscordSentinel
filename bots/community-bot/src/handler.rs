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

use std::sync::Arc;

use crate::api_client::ApiClient;
use crate::commands;
use crate::cooldown::InteractionCooldown;
use crate::exclusive_groups;
use crate::prerequisites;
use crate::sponsorship::SponsorshipTracker;
use crate::temp_roles::{self, TempRoleTracker};

/// Cle TypeMap pour le client API specifique au community-bot.
pub struct RolesApiKey;
impl TypeMapKey for RolesApiKey {
    type Value = ApiClient;
}

/// Cle TypeMap pour le rate limiter des interactions (anti-spam).
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

pub struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        info!(bot = %ready.user.name, guilds = ready.guilds.len(), "Community bot connecte");
        register_guilds(&ctx, &ready).await;

        // Vider les anciennes commandes globales (evite les doublons quand on
        // a d'abord deploye en global puis migre en per-guild).
        if let Err(e) = serenity::model::application::Command::set_global_commands(&ctx.http, vec![]).await {
            warn!(error = %e, "Echec nettoyage commandes globales community-bot");
        }

        // Enregistrer les commandes par guild (instantane vs 1h pour global)
        for guild_status in &ready.guilds {
            let guild_id = guild_status.id;
            if let Err(e) = guild_id
                .set_commands(&ctx.http, commands::all())
                .await
            {
                warn!(error = %e, guild = %guild_id, "Erreur enregistrement commandes");
            }
        }

        // Charger les roles temporaires actifs depuis l'API
        {
            let data = ctx.data.read().await;
            if let (Some(api), Some(tracker)) = (data.get::<RolesApiKey>(), data.get::<TempRoleKey>()) {
                for guild_status in &ready.guilds {
                    let gid = guild_status.id.to_string();
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
                                info!(guild = %gid, count = loaded, "Roles temporaires recharges depuis l'API");
                            }
                        }
                        Err(e) => {
                            warn!(error = %e, guild = %gid, "Echec chargement roles temporaires");
                        }
                    }
                }
            }
        }
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
                let cid = component.data.custom_id.as_str();
                if cid.starts_with("role_") {
                    handle_role_button(&ctx, &component).await;
                } else if cid.starts_with("sponsor_accept:") || cid.starts_with("sponsor_refuse:") {
                    commands::sponsor::handle_button(&ctx, &component).await;
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

    // Rate limit anti-spam : 2s par (user, role) pour eviter un user qui
    // clique 50 fois/s avec un self-bot. Chaque clic declenche ~5 appels
    // API/Discord, donc sans ca un seul user peut DDoS le bot entier.
    {
        let data = ctx.data.read().await;
        if let Some(cooldown) = data.get::<CooldownKey>() {
            let key = format!("role_{}", role_id);
            if let Some(remaining) = cooldown.check_and_set(component.user.id.get(), &key, 2) {
                let response = CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content(format!(
                            "⏱️ Calme-toi un peu... attends {remaining}s avant de refaire cette action."
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

    // Lire la config guild pour les features avancees
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

        // C1 — Si c'est un role temporaire, persister en API AVANT d'assigner
        // le role Discord. Si l'API echoue, on abort sans modifier Discord.
        // Pour les roles non-temporaires, pas de persistance necessaire.
        let temp_raw = BaseApiClient::config_or(&guild_config, "temp_roles", "");
        let temp_roles_config = temp_roles::parse_temp_roles(&temp_raw);
        let temp_duration = temp_roles::get_temp_duration(&temp_roles_config, role_id);

        if let Some(duration) = temp_duration {
            // Persister d'abord via l'API
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
                    .description("Impossible d'enregistrer le role temporaire cote serveur. Rien n'a ete applique.");
                let response = CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new().embed(embed).ephemeral(true),
                );
                if let Err(e) = component.create_response(&ctx.http, response).await {
                    warn!(error = %e, "Failed to send temp_role API error response");
                }
                return;
            }

            // API OK — tracker local
            let data = ctx.data.read().await;
            if let Some(tracker) = data.get::<TempRoleKey>() {
                tracker.add(guild_id.get(), component.user.id.get(), role_id, duration);
            }
        }

        // Retirer les roles exclusifs en conflit (apres la persistance OK)
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

        // Ajouter le role (enfin — apres validation API et cleanup conflits)
        if let Ok(m) = guild_id.member(&ctx.http, component.user.id).await {
            if let Err(e) = m.add_role(&ctx.http, role).await {
                warn!(error = %e, "Failed to add role");
                // Si c'etait un temp_role, rollback la persistance API
                if temp_duration.is_some() {
                    let data = ctx.data.read().await;
                    if let Some(tracker) = data.get::<TempRoleKey>() {
                        tracker.remove(guild_id.get(), component.user.id.get(), role_id);
                    }
                    // Rollback API : supprimer le temp_role persiste.
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

    let message = CreateMessage::new().embed(embed).components(rows);

    channel_id.send_message(&ctx.http, message).await
}

