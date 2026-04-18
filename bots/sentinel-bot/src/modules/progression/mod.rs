//! Module progression — XP, levels, streaks (ex progression-bot).

pub const MODULE_BOT_NAME: &str = "progression-bot";

pub mod api_client;
pub mod level_channel;
pub mod level_cmd;
pub mod multipliers;
pub mod stats_cmd;
pub mod streaks;
pub mod tracker;
pub mod xp_cooldown;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use dashmap::DashMap;
use serenity::all::{CommandInteraction, Context, CreateCommand, CreateMessage};
use serenity::model::channel::Message;
use serenity::model::guild::Member;
use serenity::model::id::{ChannelId, RoleId, UserId, GuildId};
use serenity::model::voice::VoiceState;
use serenity::prelude::*;
use tracing::{info, warn};

use sentinel_shared::api_client::BaseApiClient;
use sentinel_shared::discord_helpers::{
    guild_config_or_default, is_module_enabled, is_module_enabled_or_reply_command,
};
use sentinel_shared::embeds::success_embed;
use sentinel_shared::heartbeat::ApiClientKey;

use api_client::{ApiClient, RewardEntry};
use streaks::StreakTracker;
use tracker::StatsTracker;
use xp_cooldown::XpCooldown;

// ── TypeMapKeys ──

pub struct StatsApiKey;
impl TypeMapKey for StatsApiKey {
    type Value = ApiClient;
}

pub struct TrackerKey;
impl TypeMapKey for TrackerKey {
    type Value = StatsTracker;
}

pub struct XpCooldownKey;
impl TypeMapKey for XpCooldownKey {
    type Value = XpCooldown;
}

pub struct StreakTrackerKey;
impl TypeMapKey for StreakTrackerKey {
    type Value = StreakTracker;
}

pub struct RewardsCacheKey;
impl TypeMapKey for RewardsCacheKey {
    type Value = Arc<RewardsCache>;
}

// ── RewardsCache ──

pub struct RewardsCache {
    cache: DashMap<String, (Vec<RewardEntry>, Instant)>,
}

impl RewardsCache {
    pub fn new() -> Self {
        Self { cache: DashMap::new() }
    }

    pub fn get(&self, guild_id: &str) -> Option<Vec<RewardEntry>> {
        self.cache.get(guild_id).and_then(|entry| {
            if entry.1.elapsed().as_secs() < 300 {
                Some(entry.0.clone())
            } else {
                None
            }
        })
    }

    pub fn set(&self, guild_id: &str, rewards: Vec<RewardEntry>) {
        self.cache.insert(guild_id.to_string(), (rewards, Instant::now()));
    }
}

// ── Init TypeMapKeys ──

/// Insere les TypeMapKeys du module progression dans le TypeMap partage.
pub fn init_typemap(
    data: &mut serenity::prelude::TypeMap,
    api: &Arc<sentinel_shared::api_client::BaseApiClient>,
    grpc: &Arc<sentinel_shared::grpc_client::SentinelGrpcClient>,
) {
    data.insert::<StatsApiKey>(api_client::ApiClient::new(Arc::clone(api), Arc::clone(grpc)));
    data.insert::<TrackerKey>(tracker::StatsTracker::new());
    data.insert::<XpCooldownKey>(xp_cooldown::XpCooldown::new());
    data.insert::<StreakTrackerKey>(streaks::StreakTracker::new());
    data.insert::<RewardsCacheKey>(Arc::new(RewardsCache::new()));
}

// ── Slash commands ──

pub fn register_commands() -> Vec<CreateCommand> {
    vec![level_cmd::register(), stats_cmd::register()]
}

pub async fn handle_command(ctx: &Context, command: &CommandInteraction) {
    if !is_module_enabled_or_reply_command(ctx, command, MODULE_BOT_NAME).await {
        return;
    }
    match command.data.name.as_str() {
        "level" => level_cmd::handle(ctx, command).await,
        "stats" => stats_cmd::handle(ctx, command).await,
        _ => {}
    }
}

// ── Event handlers (free functions) ──

/// Appele sur chaque message — XP texte, streaks, multipliers, level-up.
pub async fn on_message(ctx: &Context, msg: &Message) {
    let guild_id = match msg.guild_id {
        Some(id) => id,
        None => return,
    };

    // Charger la config guild (helper partage : data.read() + get_guild_config)
    let guild_config =
        guild_config_or_default(ctx, &guild_id.to_string(), MODULE_BOT_NAME).await;
    if !BaseApiClient::config_bool(&guild_config, "enabled", true) {
        return;
    }

    let data = ctx.data.read().await;

    // Track localement
    if let Some(tracker) = data.get::<TrackerKey>() {
        tracker.record_message(guild_id.get(), msg.author.id.get()).await;
    }

    // Envoyer au backend
    let api = match data.get::<StatsApiKey>() {
        Some(a) => a,
        None => return,
    };

    if let Err(e) = api
        .record_messages(
            &guild_id.to_string(),
            &msg.author.id.to_string(),
            &msg.author.name,
            1,
        )
        .await
    {
        warn!(error = %e, "Impossible d'envoyer les stats messages au backend");
    }

    let cooldown_secs = BaseApiClient::config_u64(&guild_config, "xp_cooldown_secs", 60);
    let can_gain = if let Some(cooldown) = data.get::<XpCooldownKey>() {
        cooldown.can_gain_xp(guild_id.get(), msg.author.id.get(), cooldown_secs)
    } else {
        true
    };

    if !can_gain {
        return;
    }

    if let Some(cooldown) = data.get::<XpCooldownKey>() {
        cooldown.record_xp(guild_id.get(), msg.author.id.get());
    }

    // Streak tracking
    let streak_enabled = BaseApiClient::config_bool(&guild_config, "streak_enabled", true);
    let streak_mult = if streak_enabled {
        if let Some(streak_tracker) = data.get::<StreakTrackerKey>() {
            if !streak_tracker.has(guild_id.get(), msg.author.id.get()) {
                if let Ok(streak_data) = api.get_streak(&guild_id.to_string(), &msg.author.id.to_string()).await {
                    streak_tracker.seed(
                        guild_id.get(),
                        msg.author.id.get(),
                        streak_data.streak_current,
                        streak_data.streak_best,
                        streak_data.streak_last_day,
                        streak_data.streak_last_year,
                    );
                }
            }

            let now = time::OffsetDateTime::now_utc();
            let update = streak_tracker.record_activity(
                guild_id.get(),
                msg.author.id.get(),
                now.ordinal() as u32,
                now.year(),
            );

            if update.new_day {
                let (current, best) = streak_tracker.get_streak(guild_id.get(), msg.author.id.get());
                api.update_streak(
                    &guild_id.to_string(),
                    &msg.author.id.to_string(),
                    current,
                    best,
                    now.ordinal() as u32,
                    now.year(),
                ).await;
                if let Some(base) = data.get::<ApiClientKey>() {
                    base.publish_event("streak_updated", serde_json::json!({
                        "guild_id": guild_id.to_string(),
                        "user_id": msg.author.id.to_string(),
                        "username": msg.author.name,
                        "streak_current": current,
                        "streak_best": best,
                    }));
                }
            }

            update.xp_multiplier
        } else {
            1.0
        }
    } else {
        1.0
    };

    // Channel & role multipliers
    let channel_mults = multipliers::parse_multipliers(
        &BaseApiClient::config_or(&guild_config, "xp_channel_multipliers", ""),
    );
    let role_mults = multipliers::parse_multipliers(
        &BaseApiClient::config_or(&guild_config, "xp_role_multipliers", ""),
    );

    let channel_mult = multipliers::get_channel_multiplier(&channel_mults, msg.channel_id.get());
    let user_roles: Vec<u64> = msg.member.as_ref()
        .map(|m| m.roles.iter().map(|r| r.get()).collect())
        .unwrap_or_default();
    let role_mult = multipliers::get_role_multiplier(&role_mults, &user_roles);

    let base_xp = BaseApiClient::config_u64(&guild_config, "xp_per_message", 15) as f64;
    let final_xp = (base_xp * channel_mult * role_mult * streak_mult)
        .round()
        .clamp(1.0, 1000.0) as i64;

    match api
        .add_xp(
            &guild_id.to_string(),
            &msg.author.id.to_string(),
            &msg.author.name,
            final_xp,
            "text",
        )
        .await
    {
        Ok(result) => {
            if result.leveled_up {
                let level = result.user.level_text;
                let embed = success_embed("\u{1f4dd} LEVEL UP Texte !")
                    .description(format!(
                        "<@{}> est maintenant **niveau {} en texte** !",
                        msg.author.id, level
                    ))
                    .thumbnail(msg.author.face());

                if let Some(ch_id) = level_channel::resolve_level_up_channel(&guild_config) {
                    let target = ChannelId::new(ch_id);
                    if let Err(e) = target.send_message(
                        &ctx.http,
                        CreateMessage::new().embed(embed),
                    ).await {
                        warn!(error = %e, "Failed to send text level-up message");
                    }
                }
            }

            let needs_role_check = result.leveled_up || (result.old_level == 0 && result.user.level_text > 0);
            if needs_role_check {
                let lt = result.user.level_text;
                let lv = result.user.level_voice;
                let lg = result.user.level;
                drop(data);
                check_and_assign_all_roles(ctx, guild_id, msg.author.id, lt, lv, lg).await;
            }
        }
        Err(e) => {
            tracing::debug!(error = %e, "Erreur ajout XP message");
        }
    }
}

/// Appele sur voice_state_update — XP vocal, suivi sessions.
pub async fn on_voice_state_update(ctx: &Context, old: Option<VoiceState>, new: &VoiceState) {
    let guild_id = match new.guild_id {
        Some(id) => id,
        None => return,
    };

    let user_id = new.user_id;

    if !is_module_enabled(ctx, &guild_id.to_string(), MODULE_BOT_NAME).await {
        return;
    }

    let data = ctx.data.read().await;

    let was_in_voice = old.as_ref().and_then(|s| s.channel_id).is_some();
    let is_in_voice = new.channel_id.is_some();

    let is_afk_now = new.self_mute && new.self_deaf;
    let was_afk = old
        .as_ref()
        .map(|s| s.self_mute && s.self_deaf)
        .unwrap_or(false);

    let tracker = data.get::<TrackerKey>();
    let api = data.get::<StatsApiKey>();

    match (was_in_voice, is_in_voice) {
        (false, true) => {
            if let Some(tracker) = tracker {
                tracker
                    .voice_join(guild_id.get(), user_id.get(), is_afk_now)
                    .await;
            }
        }
        (true, true) => {
            if was_afk != is_afk_now {
                if let Some(tracker) = tracker {
                    tracker
                        .set_voice_afk(guild_id.get(), user_id.get(), is_afk_now)
                        .await;
                }
            }
        }
        (true, false) => {
            if let Some(tracker) = tracker {
                let seconds = tracker.voice_leave(guild_id.get(), user_id.get()).await;

                if seconds > 0 {
                    let username = user_id
                        .to_user(&ctx.http)
                        .await
                        .map(|u| u.name)
                        .unwrap_or_else(|_| user_id.to_string());

                    let (channel_id_str, channel_name) = if let Some(old_state) = &old {
                        if let Some(ch_id) = old_state.channel_id {
                            let name = ch_id.to_channel(&ctx.http).await
                                .ok()
                                .and_then(|c| c.guild())
                                .map(|c| c.name.clone())
                                .unwrap_or_default();
                            (ch_id.to_string(), name)
                        } else {
                            (String::new(), String::new())
                        }
                    } else {
                        (String::new(), String::new())
                    };

                    if let Some(api) = api {
                        if let Err(e) = api
                            .record_voice(
                                &guild_id.to_string(),
                                &user_id.to_string(),
                                &username,
                                seconds,
                                &channel_id_str,
                                &channel_name,
                            )
                            .await
                        {
                            warn!(error = %e, "Impossible d'envoyer les stats vocal au backend");
                        }

                        let xp_per_minute = if let Some(base) = data.get::<ApiClientKey>() {
                            let gc = base.get_guild_config_for(&guild_id.to_string(), MODULE_BOT_NAME).await.unwrap_or_default();
                            BaseApiClient::config_u64(&gc, "xp_per_voice_minute", 5) as i64
                        } else {
                            5
                        };
                        let xp_amount = (seconds / 60) as i64 * xp_per_minute;
                        if xp_amount > 0 {
                            match api
                                .add_xp(
                                    &guild_id.to_string(),
                                    &user_id.to_string(),
                                    &username,
                                    xp_amount,
                                    "voice",
                                )
                                .await
                            {
                                Ok(result) => {
                                    if result.leveled_up {
                                        let level = result.user.level_voice;
                                        let embed = success_embed("\u{1f3a4} LEVEL UP Vocal !")
                                            .description(format!(
                                                "<@{}> est maintenant **niveau {} en vocal** !",
                                                user_id, level
                                            ));

                                        let voice_guild_config = if let Some(base) = data.get::<ApiClientKey>() {
                                            base.get_guild_config_for(&guild_id.to_string(), MODULE_BOT_NAME).await.unwrap_or_default()
                                        } else {
                                            HashMap::new()
                                        };

                                        if let Some(ch_id) = level_channel::resolve_level_up_channel(&voice_guild_config) {
                                            let ch = ChannelId::new(ch_id);
                                            if let Err(e) = ch.send_message(&ctx.http, CreateMessage::new().embed(embed)).await {
                                                warn!(error = %e, "Failed to send voice level-up message");
                                            }
                                        }
                                    }

                                    let needs_role_check = result.leveled_up || (result.old_level == 0 && result.user.level_voice > 0);
                                    if needs_role_check {
                                        let lt = result.user.level_text;
                                        let lv = result.user.level_voice;
                                        let lg = result.user.level;
                                        drop(data);
                                        check_and_assign_all_roles(ctx, guild_id, user_id, lt, lv, lg).await;
                                        return;
                                    }
                                }
                                Err(e) => {
                                    tracing::debug!(error = %e, "Erreur ajout XP vocal");
                                }
                            }
                        }
                    }
                }
            }
        }
        _ => {}
    }
}

/// Attribue le role par defaut au nouveau membre (config guild).
pub async fn assign_default_role(ctx: &Context, new_member: &Member) {
    let guild_id = new_member.guild_id;

    let data = ctx.data.read().await;
    let default_role_id = if let Some(base) = data.get::<ApiClientKey>() {
        let config = match base.get_guild_config_for(&guild_id.to_string(), MODULE_BOT_NAME).await {
            Ok(cfg) => cfg,
            Err(e) => {
                tracing::warn!(error = %e, guild_id = %guild_id, "Echec chargement config guild");
                HashMap::new()
            }
        };
        let role_str = BaseApiClient::config_or(&config, "default_role_id", "");
        if role_str.is_empty() { None } else { role_str.parse::<u64>().ok() }
    } else {
        None
    };
    drop(data);

    if let Some(role_id) = default_role_id {
        match new_member.add_role(&ctx.http, RoleId::new(role_id)).await {
            Ok(_) => info!(guild=%guild_id, user=%new_member.user.id, role=%role_id, "Role par defaut attribue"),
            Err(e) => warn!(guild=%guild_id, user=%new_member.user.id, error=%e, "Echec attribution role par defaut"),
        }
    }
}

// ── Helper interne ──

/// Verifie TOUS les rewards (texte, vocal, jours) et attribue les roles manquants.
async fn check_and_assign_all_roles(
    ctx: &Context,
    guild_id: GuildId,
    user_id: UserId,
    level_text: i32,
    level_voice: i32,
    level_global: i32,
) {
    let member = match guild_id.member(&ctx.http, user_id).await {
        Ok(m) => m,
        Err(_) => return,
    };

    let member_roles: Vec<u64> = member.roles.iter().map(|r| r.get()).collect();

    let days_since_join = member.joined_at
        .map(|ts| {
            let now = serenity::model::Timestamp::now();
            (now.unix_timestamp() - ts.unix_timestamp()) / 86400
        })
        .unwrap_or(0);

    let data = ctx.data.read().await;

    let xp_role_mode = if let Some(base) = data.get::<ApiClientKey>() {
        let config = match base.get_guild_config_for(&guild_id.to_string(), MODULE_BOT_NAME).await {
            Ok(cfg) => cfg,
            Err(e) => {
                tracing::warn!(error = %e, guild_id = %guild_id, "Echec chargement config guild");
                HashMap::new()
            }
        };
        BaseApiClient::config_or(&config, "xp_role_mode", "separate").to_string()
    } else {
        "separate".to_string()
    };

    let (effective_text, effective_voice) = match xp_role_mode.as_str() {
        "max" => {
            let max_level = level_text.max(level_voice);
            (max_level, max_level)
        }
        "total" => (level_global, level_global),
        _ => (level_text, level_voice),
    };

    let guild_str = guild_id.to_string();
    let rewards_cache = data.get::<RewardsCacheKey>().cloned();
    let rewards = if let Some(ref cache) = rewards_cache {
        if let Some(cached) = cache.get(&guild_str) {
            cached
        } else if let Some(api) = data.get::<StatsApiKey>() {
            match api.get_all_rewards(&guild_str).await {
                Ok(r) => { cache.set(&guild_str, r.clone()); r }
                Err(_) => return,
            }
        } else {
            return;
        }
    } else if let Some(api) = data.get::<StatsApiKey>() {
        match api.get_all_rewards(&guild_str).await {
            Ok(r) => r,
            Err(_) => return,
        }
    } else {
        return;
    };

    if rewards.is_empty() {
        return;
    }

    let sources = ["text", "voice", "days"];

    for source in &sources {
        let mut source_rewards: Vec<&RewardEntry> = rewards
            .iter()
            .filter(|r| r.source == *source)
            .collect();
        source_rewards.sort_by(|a, b| b.level.cmp(&a.level));

        let effective_level = match *source {
            "text" => effective_text,
            "voice" => effective_voice,
            "days" => days_since_join as i32,
            _ => 0,
        };

        let best_reward = source_rewards.iter().find(|r| effective_level >= r.level);

        let all_source_role_ids: Vec<u64> = source_rewards
            .iter()
            .filter_map(|r| r.role_id.parse::<u64>().ok())
            .collect();

        match best_reward {
            Some(reward) => {
                if let Ok(best_role_id) = reward.role_id.parse::<u64>() {
                    if !member_roles.contains(&best_role_id) {
                        match member.add_role(&ctx.http, RoleId::new(best_role_id)).await {
                            Ok(_) => info!(guild=%guild_id, user=%user_id, role=%best_role_id, source=%source, "Role attribue"),
                            Err(e) => warn!(guild=%guild_id, user=%user_id, role=%best_role_id, error=%e, "Echec attribution role"),
                        }
                    }

                    for role_id in &all_source_role_ids {
                        if *role_id != best_role_id && member_roles.contains(role_id) {
                            match member.remove_role(&ctx.http, RoleId::new(*role_id)).await {
                                Ok(_) => info!(guild=%guild_id, user=%user_id, role=%role_id, source=%source, "Ancien role retire"),
                                Err(e) => warn!(guild=%guild_id, user=%user_id, role=%role_id, error=%e, "Echec retrait ancien role"),
                            }
                        }
                    }
                }
            }
            None => {
                for role_id in &all_source_role_ids {
                    if member_roles.contains(role_id) {
                        if let Err(e) = member.remove_role(&ctx.http, RoleId::new(*role_id)).await {
                            warn!(error = %e, "Failed to remove unqualified role");
                        }
                    }
                }
            }
        }
    }
}
