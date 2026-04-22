mod bot_config;
mod dashboard_stats;
mod guild;
mod infraction;
mod log_entry;
mod message_analysis;
mod moderation_action;
mod rule;
mod security_event;
mod ticket;
mod user_stats;
mod conduct;
mod voice_channel;
mod watched_user;
mod audit_log;
mod image_analysis;
mod level;
mod daily_activity;
mod role_panel;
mod discord_role;
mod ia_config;
mod strikes;
mod sanction_reminder;
mod user_note;
mod ai_models;
mod coude_purge;
mod rbac;
mod config_parsers;
mod coude_expire;
pub mod analytics;

pub use coude_expire::cowardice_penalty;

pub use ai_models::{
    format_model_display_name, is_valid_model_type, path_basename, SUPPORTED_MODEL_TYPES,
};
pub use coude_purge::COUDE_PURGE_TABLES;
pub use rbac::{
    is_owner_self_demotion, truncate_display_name, would_revoke_last_owner,
    RBAC_DISPLAY_NAME_MAX,
};
pub use config_parsers::{is_worker_service, parse_bool_config, parse_i64_config};

pub use ia_config::IaConfig;
pub use audit_log::{
    is_security_audit_event, AuditLog, AUDIT_EVENT_MEMBER_NICKNAME_HISTORY,
    AUDIT_EVENT_SECURITY_PREFIX,
};
pub use image_analysis::{is_allowed_image_content_type, is_image_size_acceptable, ImageAnalysis, ImageClassification, ALLOWED_IMAGE_CONTENT_TYPES, MAX_IMAGE_BASE64_LEN};
pub use bot_config::{BotDefinition, BotGuildConfig};
pub use conduct::{
    apply_conduct_penalty, apply_conduct_regen, ConductConfig, ConductPointsLog,
    UserConductPoints, MUTE_AT_ZERO_POINTS_DURATION_MINS,
};
pub use daily_activity::DailyActivity;
pub use role_panel::{AutoRole, RolePanel, RolePanelDetail, RolePanelEntry};
pub use dashboard_stats::DashboardStats;
pub use guild::Guild;
pub use infraction::Infraction;
pub use level::{xp_progress, xp_for_level, level_from_xp, LevelConfig, LevelReward, UserLevel, XpSource};
pub use log_entry::LogEntry;
pub use message_analysis::MessageAnalysis;
pub use moderation_action::{ModerationAction, UserModerationHistory};
pub use rule::Rule;
pub use security_event::SecurityEvent;
pub use ticket::{Ticket, TicketDetail, TicketMessage};
pub use user_stats::{GuildStatsOverview, GuildVoiceStats, UserStats, VoiceSessionStats};
pub use voice_channel::{
    VoiceChannel, VoiceChannelBan, VoiceChannelCoAdmin, VoiceChannelConfig,
    VoiceChannelDetail, VoiceChannelInviteLink, VoiceChannelTheme,
    VoiceChannelWhitelistEntry,
};
pub use watched_user::{classify_risk_level, WatchedUser};
pub use discord_role::{parse_discord_permissions_bitfield, DiscordRole};
pub use strikes::{StrikeConfig, StrikeResult, StrikeThreshold, UserStrike};
pub use sanction_reminder::SanctionReminder;
pub use user_note::UserNote;

mod user_activity;
pub use user_activity::UserActivity;

mod guild_member;
pub use guild_member::{GuildMember, MemberSummary, MemberConduct, MemberInfractions, MemberModeration, MemberStats};

mod blackjack;
pub use blackjack::{
    calculate_score, create_deck, is_blackjack_game_over, BlackjackConfig, BlackjackGame, Card,
    BLACKJACK_FINAL_STATUSES, BLACKJACK_SHOE_DECKS, BLACKJACK_SHOE_TOTAL_CARDS,
    DEFAULT_BLACKJACK_MAX_PLAYERS,
};

mod moderation_review;
pub use moderation_review::{
    is_valid_review_status, truncate_review_text, validate_evidence_url,
    DEFAULT_MUTE_DURATION_SECS, MAX_EVIDENCE_URL_LEN, MAX_REVIEW_TEXT_LEN, VALID_REVIEW_STATUSES,
};

mod coude_tournament;
pub use coude_tournament::{
    current_week_bounds, estimate_tournament_prize_pool, week_bounds_for,
    TOURNAMENT_PRIZE_POOL_PERCENT,
};

mod job_whitelists;
pub use job_whitelists::{
    is_valid_ai_job_type, is_valid_export_format, is_valid_export_job_type,
    VALID_AI_JOB_TYPES, VALID_EXPORT_FORMATS, VALID_EXPORT_JOB_TYPES,
};

mod purge;
pub use purge::{validate_purge_days_allow_zero, validate_purge_days_strictly_positive};

mod guild_member_reset;
pub use guild_member_reset::{
    MemberResetTable, CHANNELS_CACHE_TTL_SECS, DISCORD_LIST_MEMBERS_CAP, MEMBER_RESET_TABLES,
    MEMBERS_CACHE_TTL_SECS,
};

mod coude_limits;
pub use coude_limits::{
    DEFAULT_COUDE_COMBATS_LIMIT, DEFAULT_COUDE_OPPONENT_COUNT,
    DEFAULT_COUDE_SOCIAL_LEADERBOARD_LIMIT,
};

mod game;
pub use game::{
    format_custom_emoji, is_allowed_emoji_mime, normalize_game_name, normalize_optional_tag,
    parse_role_color_hex, slugify_emoji_name, DEFAULT_GAME_ROLE_COLOR, MAX_EMOJI_IMAGE_BYTES,
};

mod wallet;
pub use wallet::{
    clamp_debit_to_balance, resolve_reset_balance, resolve_starting_coins,
    validate_positive_amount, validate_transfer_distinct_users, Wallet, WalletTransaction,
};

mod coude_player;
pub use coude_player::{
    title_for_level as coude_title_for_level, xp_for_level as coude_xp_for_level, CombatStat,
    CoudePlayer, XpProgress, COUDE_MAX_LEVEL,
};

mod coude_combat;
pub use coude_combat::{CombatResolution, CoudeCombat, NewCoudeCombat};

mod combat_resolution_rules;
pub use combat_resolution_rules::{
    apply_insurance_to_loss, compute_combat_xp, format_bet_payout_lines, CombatXpAwards,
    InsuranceAdjustment, COMBAT_XP_LOSER, COMBAT_XP_WINNER_BASE, COMBAT_XP_WINNER_UNDERDOG,
    UNDERDOG_LEVEL_GAP,
};

mod coude_bet;
pub use coude_bet::{
    calculate_bet_resolution, BetPayout, BetResolutionPlan, CoudeBet,
    FighterBetBonus as CoudeFighterBetBonus, NewCoudeBet, RefundSummary,
};

mod coude_inventory;
pub use coude_inventory::{
    CoudeInsurance, CoudeInventoryItem, CoudePrime, NewCoudePrime,
};

mod coude_social;
pub use coude_social::{
    clamp_leaderboard_limit, daily_chaos_amount, CoudeCurrentSeason, CoudeEvent,
    CoudeLeaderboardEntry, DailyChaosOutcome, LeaderboardCategory, NewDailyChaos,
    DAILY_CHAOS_MAX, DEFAULT_CHAOS_PERCENT, LEADERBOARD_MAX_LIMIT, LEADERBOARD_MIN_LIMIT,
    MIN_COINS_ELIGIBLE,
};

mod coude_cashbox;
pub use coude_cashbox::{
    CashboxRedistribution, CashboxRedistributionEntry, CashboxSource, CoudeCashbox,
};

mod coude_steal_protection;
pub use coude_steal_protection::{
    find_protection_item, CoudeStealProtection, StealProtectionDuration, StealProtectionItemDef,
    STEAL_PROTECTION_ITEMS,
};

mod coude_steal_boost;
pub use coude_steal_boost::{
    find_boost_item, sum_roll_bonus_for_active_keys, CoudeStealBoost, StealBoostDuration,
    StealBoostItemDef, STEAL_BOOST_ITEMS,
};

mod coude_taunt;
pub use coude_taunt::{
    build_taunt_event, build_taunt_event_single, crossed_threshold, nickname_suffix_for,
    CoudeTauntsConfig, StreakKind, TauntEvent, TAUNT_THRESHOLDS,
};

mod coude_balance;
pub use coude_balance::{CoudeBalanceParams, DoubleCoupMode};

mod coude_heist;
pub use coude_heist::{
    compute_success_chance, find_heist_tool, CoudeHeistAttempt, CoudePrisonState, HeistOutcome,
    HeistToolDef, HEIST_BASE_SUCCESS_PERCENT, HEIST_COOLDOWN_DAYS, HEIST_GAIN_MAX_PERCENT,
    HEIST_GAIN_MIN_PERCENT, HEIST_ITEM_BONUS_PERCENT, HEIST_MAX_SUCCESS_PERCENT,
    HEIST_PRISON_HOURS, HEIST_TOOLS,
};
