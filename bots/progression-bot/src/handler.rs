use std::sync::Arc;
use std::time::Instant;

use dashmap::DashMap;
use serenity::async_trait;
use serenity::model::application::Interaction;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::model::voice::VoiceState;
use serenity::prelude::*;
use tracing::{error, info, warn};

use serenity::builder::CreateMessage;

use sentinel_shared::embeds::success_embed;

use sentinel_shared::api_client::BaseApiClient;
use sentinel_shared::heartbeat::{ApiClientKey, register_guilds};

use crate::api_client::{ApiClient, RewardEntry};
use crate::commands;
use crate::multipliers;
use crate::streaks::StreakTracker;
use crate::tracker::StatsTracker;
use crate::xp_cooldown::XpCooldown;

/// Cache des rewards par guild avec TTL de 5 minutes.
pub struct RewardsCache {
    cache: DashMap<String, (Vec<RewardEntry>, Instant)>,
}

impl RewardsCache {
    pub fn new() -> Self {
        Self { cache: DashMap::new() }
    }

    pub fn get(&self, guild_id: &str) -> Option<Vec<RewardEntry>> {
        self.cache.get(guild_id).and_then(|entry| {
            if entry.1.elapsed().as_secs() < 300 { // 5 minutes TTL
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

pub struct RewardsCacheKey;
impl TypeMapKey for RewardsCacheKey {
    type Value = Arc<RewardsCache>;
}

/// Cle TypeMap pour le client API specifique au progression-bot.
pub struct StatsApiKey;

impl TypeMapKey for StatsApiKey {
    type Value = ApiClient;
}

/// Cle pour acceder au StatsTracker dans le TypeMap.
pub struct TrackerKey;

pub struct XpCooldownKey;
impl TypeMapKey for XpCooldownKey {
    type Value = XpCooldown;
}

pub struct StreakTrackerKey;
impl TypeMapKey for StreakTrackerKey {
    type Value = StreakTracker;
}

impl TypeMapKey for TrackerKey {
    type Value = StatsTracker;
}

pub struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        info!(bot = %ready.user.name, "Progression bot connecte");

        // Enregistrer les guilds via le shared helper
        register_guilds(&ctx, &ready).await;

        if let Err(e) = serenity::model::application::Command::set_global_commands(
            &ctx.http,
            commands::all(),
        )
        .await
        {
            error!(error = %e, "Impossible d'enregistrer les slash commands");
        } else {
            info!("Slash commands enregistrees");
        }
    }

    /// Compteur de messages — envoie au backend via l'API.
    async fn message(&self, ctx: Context, msg: Message) {
        if msg.author.bot {
            return;
        }

        let guild_id = match msg.guild_id {
            Some(id) => id,
            None => return,
        };

        let data = ctx.data.read().await;

        // Charger la config guild UNE SEULE FOIS
        let guild_config = if let Some(api) = data.get::<ApiClientKey>() {
            let config = match api.get_guild_config(&guild_id.to_string()).await {
                Ok(cfg) => cfg,
                Err(e) => {
                    tracing::warn!(error = %e, guild_id = %guild_id, "Echec chargement config guild");
                    std::collections::HashMap::new()
                }
            };
            if !BaseApiClient::config_bool(&config, "enabled", true) {
                return;
            }
            config
        } else {
            std::collections::HashMap::new()
        };

        // Track localement (fallback pour les commandes)
        if let Some(tracker) = data.get::<TrackerKey>() {
            tracker.record_message(guild_id.get(), msg.author.id.get()).await;
        }

        // Envoyer au backend
        if let Some(api) = data.get::<StatsApiKey>() {
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
                // Message tracke mais pas d'XP
                return;
            }

            // Record XP gain
            if let Some(cooldown) = data.get::<XpCooldownKey>() {
                cooldown.record_xp(guild_id.get(), msg.author.id.get());
            }

            // Streak tracking
            let streak_enabled = BaseApiClient::config_bool(&guild_config, "streak_enabled", true);
            let streak_mult = if streak_enabled {
                if let Some(streak_tracker) = data.get::<StreakTrackerKey>() {
                    // Si l'utilisateur n'est pas en cache (premier message apres restart),
                    // charger son streak depuis l'API
                    if !streak_tracker.has(guild_id.get(), msg.author.id.get()) {
                        if let Some(api) = data.get::<StatsApiKey>() {
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
                    }

                    let now = time::OffsetDateTime::now_utc();
                    let update = streak_tracker.record_activity(
                        guild_id.get(),
                        msg.author.id.get(),
                        now.ordinal() as u32,
                        now.year(),
                    );

                    // Persister le streak via l'API si c'est un nouveau jour
                    if update.new_day {
                        let (current, best) = streak_tracker.get_streak(guild_id.get(), msg.author.id.get());
                        if let Some(api) = data.get::<StatsApiKey>() {
                            api.update_streak(
                                &guild_id.to_string(),
                                &msg.author.id.to_string(),
                                current,
                                best,
                                now.ordinal() as u32,
                                now.year(),
                            ).await;
                        }
                        // Event temps reel pour le desktop
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

            let base_xp = 15.0;
            let final_xp = (base_xp * channel_mult * role_mult * streak_mult)
                .round()
                .min(1000.0)
                .max(1.0) as i64;

            // Ajouter l'XP texte
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

                        // Envoyer dans le canal dedie si configure
                        if let Some(ch_id) = crate::level_channel::resolve_level_up_channel(&guild_config) {
                            let target = serenity::model::id::ChannelId::new(ch_id);
                            if let Err(e) = target.send_message(
                                &ctx.http,
                                CreateMessage::new().embed(embed),
                            ).await {
                                warn!(error = %e, "Failed to send text level-up message");
                            }
                        }
                        // Si pas configure, on n'envoie pas (evite de polluer tous les salons)
                    }

                    // Verifier les roles seulement au level-up ou premiere activite (retour d'un membre)
                    let needs_role_check = result.leveled_up || (result.old_level == 0 && result.user.level_text > 0);
                    if needs_role_check {
                        let lt = result.user.level_text;
                        let lv = result.user.level_voice;
                        let lg = result.user.level;
                        drop(data);
                        check_and_assign_all_roles(&ctx, guild_id, msg.author.id, lt, lv, lg).await;
                    }
                }
                Err(e) => {
                    tracing::debug!(error = %e, "Erreur ajout XP message");
                }
            }
        }
    }

    /// Suivi des sessions vocales (join/leave).
    async fn voice_state_update(&self, ctx: Context, old: Option<VoiceState>, new: VoiceState) {
        let guild_id = match new.guild_id {
            Some(id) => id,
            None => return,
        };

        let user_id = new.user_id;
        let data = ctx.data.read().await;

        if let Some(api) = data.get::<ApiClientKey>() {
            let config = match api.get_guild_config(&guild_id.to_string()).await {
                Ok(cfg) => cfg,
                Err(e) => {
                    tracing::warn!(error = %e, guild_id = %guild_id, "Echec chargement config guild");
                    std::collections::HashMap::new()
                }
            };
            if !BaseApiClient::config_bool(&config, "enabled", true) {
                return;
            }
        }

        let was_in_voice = old.as_ref().and_then(|s| s.channel_id).is_some();
        let is_in_voice = new.channel_id.is_some();

        let tracker = data.get::<TrackerKey>();
        let api = data.get::<StatsApiKey>();

        match (was_in_voice, is_in_voice) {
            (false, true) => {
                // Rejoint un salon vocal
                if let Some(tracker) = tracker {
                    tracker.voice_join(guild_id.get(), user_id.get()).await;
                }
            }
            (true, false) => {
                // Quitte le salon vocal — calculer la duree et envoyer au backend
                if let Some(tracker) = tracker {
                    let seconds = tracker.voice_leave(guild_id.get(), user_id.get()).await;

                    if seconds > 0 {
                        // Recuperer le nom d'utilisateur
                        let username = user_id
                            .to_user(&ctx.http)
                            .await
                            .map(|u| u.name)
                            .unwrap_or_else(|_| user_id.to_string());

                        // Recuperer le channel_id et channel_name du salon quitte
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

                            // Ajouter XP vocal (5 XP par minute)
                            let xp_amount = (seconds / 60) as i64 * 5;
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

                                            // Utiliser le canal dedie si configure
                                            let voice_guild_config = if let Some(base) = data.get::<ApiClientKey>() {
                                                base.get_guild_config(&guild_id.to_string()).await.unwrap_or_default()
                                            } else {
                                                std::collections::HashMap::new()
                                            };

                                            if let Some(ch_id) = crate::level_channel::resolve_level_up_channel(&voice_guild_config) {
                                                let ch = serenity::model::id::ChannelId::new(ch_id);
                                                if let Err(e) = ch.send_message(&ctx.http, CreateMessage::new().embed(embed)).await {
                                                    warn!(error = %e, "Failed to send voice level-up message");
                                                }
                                            }
                                            // Si pas configure, on n'envoie pas (evite de polluer les salons)
                                        }

                                        // Verifier les roles seulement au level-up ou premiere activite
                                        let needs_role_check = result.leveled_up || (result.old_level == 0 && result.user.level_voice > 0);
                                        if needs_role_check {
                                            let lt = result.user.level_text;
                                            let lv = result.user.level_voice;
                                            let lg = result.user.level;
                                            // Drop le read lock AVANT check_and_assign (evite deadlock)
                                            drop(data);
                                            check_and_assign_all_roles(&ctx, guild_id, user_id, lt, lv, lg).await;
                                            return; // data droppee, on ne peut plus l'utiliser
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

    /// Gestion des slash commands.
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::Command(command) = interaction {
            match command.data.name.as_str() {
                "stats" => commands::stats::handle(&ctx, &command).await,
                "level" => commands::level::handle(&ctx, &command).await,
                _ => {}
            }
        }
    }

    /// Nouveau membre — attribuer le role par defaut.
    async fn guild_member_addition(&self, ctx: Context, new_member: serenity::model::guild::Member) {
        let guild_id = new_member.guild_id;

        let data = ctx.data.read().await;
        let default_role_id = if let Some(base) = data.get::<ApiClientKey>() {
            let config = match base.get_guild_config(&guild_id.to_string()).await {
                Ok(cfg) => cfg,
                Err(e) => {
                    tracing::warn!(error = %e, guild_id = %guild_id, "Echec chargement config guild");
                    std::collections::HashMap::new()
                }
            };
            let role_str = BaseApiClient::config_or(&config, "default_role_id", "");
            if role_str.is_empty() { None } else { role_str.parse::<u64>().ok() }
        } else {
            None
        };
        drop(data);

        if let Some(role_id) = default_role_id {
            match new_member.add_role(&ctx.http, serenity::model::id::RoleId::new(role_id)).await {
                Ok(_) => info!(guild=%guild_id, user=%new_member.user.id, role=%role_id, "Role par defaut attribue au nouveau membre"),
                Err(e) => warn!(guild=%guild_id, user=%new_member.user.id, error=%e, "Echec attribution role par defaut"),
            }
        }
    }
}

/// Verifie TOUS les rewards (texte, vocal, jours) et attribue les roles manquants.
/// Fonctionne aussi pour les utilisateurs qui reviennent : leurs niveaux sont en base,
/// donc tous les roles en dessous de leur niveau seront re-attribues.
///
/// Modes de calcul (parametre guild "xp_role_mode") :
/// - "separate" (defaut) : texte = niveau texte, vocal = niveau vocal
/// - "max" : on prend le max(niveau texte, niveau vocal) pour les 2
/// - "total" : on prend le niveau global (XP total)
async fn check_and_assign_all_roles(
    ctx: &Context,
    guild_id: serenity::model::id::GuildId,
    user_id: serenity::model::id::UserId,
    level_text: i32,
    level_voice: i32,
    level_global: i32,
) {
    // Recuperer le membre (pour les roles actuels et la date d'arrivee)
    let member = match guild_id.member(&ctx.http, user_id).await {
        Ok(m) => m,
        Err(_) => return,
    };

    let member_roles: Vec<u64> = member.roles.iter().map(|r| r.get()).collect();

    // Calculer les jours d'anciennete
    let days_since_join = member.joined_at
        .map(|ts| {
            let now = serenity::model::Timestamp::now();
            (now.unix_timestamp() - ts.unix_timestamp()) / 86400
        })
        .unwrap_or(0);

    let data = ctx.data.read().await;

    // Lire le mode de calcul XP depuis la config guild
    let xp_role_mode = if let Some(base) = data.get::<ApiClientKey>() {
        let config = match base.get_guild_config(&guild_id.to_string()).await {
            Ok(cfg) => cfg,
            Err(e) => {
                tracing::warn!(error = %e, guild_id = %guild_id, "Echec chargement config guild");
                std::collections::HashMap::new()
            }
        };
        BaseApiClient::config_or(&config, "xp_role_mode", "separate").to_string()
    } else {
        "separate".to_string()
    };

    // Calculer les niveaux effectifs selon le mode
    let (effective_text, effective_voice) = match xp_role_mode.as_str() {
        "max" => {
            let max_level = level_text.max(level_voice);
            (max_level, max_level)
        }
        "total" => (level_global, level_global),
        _ => (level_text, level_voice), // "separate" par defaut
    };

    // Recuperer les rewards (avec cache TTL 5 min)
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

    // Grouper les rewards par source pour trouver le plus haut role qualifie par categorie
    // et retirer les roles inferieurs
    let sources = ["text", "voice", "days"];

    for source in &sources {
        // Tous les rewards de cette source, tries par level decroissant
        let mut source_rewards: Vec<&crate::api_client::RewardEntry> = rewards
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

        // Trouver le plus haut role qualifie
        let best_reward = source_rewards.iter().find(|r| effective_level >= r.level);

        // Tous les role_ids de cette source (pour savoir lesquels retirer)
        let all_source_role_ids: Vec<u64> = source_rewards
            .iter()
            .filter_map(|r| r.role_id.parse::<u64>().ok())
            .collect();

        match best_reward {
            Some(reward) => {
                if let Ok(best_role_id) = reward.role_id.parse::<u64>() {
                    // Ajouter le meilleur role si pas deja present
                    if !member_roles.contains(&best_role_id) {
                        match member.add_role(&ctx.http, serenity::model::id::RoleId::new(best_role_id)).await {
                            Ok(_) => info!(guild=%guild_id, user=%user_id, role=%best_role_id, source=%source, "Role attribue"),
                            Err(e) => warn!(guild=%guild_id, user=%user_id, role=%best_role_id, error=%e, "Echec attribution role"),
                        }
                    }

                    // Retirer tous les autres roles de cette source
                    for role_id in &all_source_role_ids {
                        if *role_id != best_role_id && member_roles.contains(role_id) {
                            match member.remove_role(&ctx.http, serenity::model::id::RoleId::new(*role_id)).await {
                                Ok(_) => info!(guild=%guild_id, user=%user_id, role=%role_id, source=%source, "Ancien role retire"),
                                Err(e) => warn!(guild=%guild_id, user=%user_id, role=%role_id, error=%e, "Echec retrait ancien role"),
                            }
                        }
                    }
                }
            }
            None => {
                // Pas qualifie pour aucun role de cette source — retirer tous les roles de cette source
                for role_id in &all_source_role_ids {
                    if member_roles.contains(role_id) {
                        if let Err(e) = member.remove_role(&ctx.http, serenity::model::id::RoleId::new(*role_id)).await {
                            warn!(error = %e, "Failed to remove unqualified role");
                        }
                    }
                }
            }
        }
    }
}
