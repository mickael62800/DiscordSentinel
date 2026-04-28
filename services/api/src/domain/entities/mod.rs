// Bounded contexts (regroupent les ~80 entites par domaine fonctionnel).
pub mod ai;
pub mod audit;
pub mod casino;
pub mod community;
pub mod coude;
pub mod moderation;
pub mod system;

// Re-export pour preserver l'API publique historique : tout le reste du code
// continue a faire `use crate::domain::entities::Type` ou
// `use crate::domain::entities::analytics::Foo` sans changer un import.
// PR2 (cosmetique) renommera les fichiers `coude_*.rs` -> `*.rs` dans coude/.

// ── ai ─────────────────────────────────────────────────────────────────────
pub use ai::ai_models::{
    format_model_display_name, is_valid_model_type, path_basename, SUPPORTED_MODEL_TYPES,
};
pub use ai::image_analysis::{
    is_allowed_image_content_type, is_image_size_acceptable, ImageAnalysis, ImageClassification,
    ALLOWED_IMAGE_CONTENT_TYPES, MAX_IMAGE_BASE64_LEN,
};
pub use ai::message_analysis::MessageAnalysis;

// ── audit ──────────────────────────────────────────────────────────────────
pub use audit::audit_log::{
    is_security_audit_event, AuditLog, AUDIT_EVENT_MEMBER_NICKNAME_HISTORY,
    AUDIT_EVENT_SECURITY_PREFIX,
};
pub use audit::dashboard_stats::DashboardStats;
pub use audit::discord_action_message::{
    kinds as discord_action_kinds, DiscordActionMessage, NewDiscordActionMessage,
};
pub use audit::security_event::SecurityEvent;
pub use audit::user_activity::UserActivity;
pub use audit::user_stats::{
    GuildStatsOverview, GuildVoiceStats, UserStats, VoiceSessionStats,
};
pub use audit::watched_user::{classify_risk_level, WatchedUser};

// ── casino ─────────────────────────────────────────────────────────────────
pub use casino::blackjack::{
    calculate_score, create_deck, is_blackjack_game_over, BlackjackConfig, BlackjackGame, Card,
    BLACKJACK_FINAL_STATUSES, BLACKJACK_SHOE_DECKS, BLACKJACK_SHOE_TOTAL_CARDS,
    DEFAULT_BLACKJACK_MAX_PLAYERS,
};
pub use casino::game::{
    format_custom_emoji, is_allowed_emoji_mime, normalize_game_name, normalize_optional_tag,
    parse_role_color_hex, slugify_emoji_name, DEFAULT_GAME_ROLE_COLOR, MAX_EMOJI_IMAGE_BYTES,
};
pub use casino::slot::{
    compute_jackpot_contribution, compute_payout, evaluate_spin, parse_csv_multipliers,
    parse_csv_symbols, parse_csv_weights, spin_with_rng as slot_spin_with_rng,
    validate_slot_config, SlotConfig, SlotConfigError, SlotJackpotPool, SlotSpin, SlotTopWinner,
    SpinOutcome,
};
pub use casino::wallet::{
    clamp_debit_to_balance, resolve_reset_balance, resolve_starting_coins,
    validate_positive_amount, validate_transfer_distinct_users, Wallet, WalletTransaction,
};
pub use casino::wheel::{
    is_memorable_case, spin_with_rng as wheel_spin_with_rng,
    spin_with_rng_curses as wheel_spin_with_rng_curses, WheelCase, WheelOutcome, WheelSpin,
    WheelTopWinner, WHEEL_CASES,
};

// ── community ──────────────────────────────────────────────────────────────
pub use community::conduct::{
    apply_conduct_penalty, apply_conduct_regen, ConductConfig, ConductPointsLog,
    UserConductPoints, MUTE_AT_ZERO_POINTS_DURATION_MINS,
};
pub use community::daily_activity::DailyActivity;
pub use community::guild_member::{
    GuildMember, MemberConduct, MemberInfractions, MemberModeration, MemberStats, MemberSummary,
};
pub use community::guild_member_reset::{
    MemberResetTable, CHANNELS_CACHE_TTL_SECS, DISCORD_LIST_MEMBERS_CAP, MEMBER_RESET_TABLES,
    MEMBERS_CACHE_TTL_SECS,
};
pub use community::level::{
    level_from_xp, xp_for_level, xp_progress, LevelConfig, LevelReward, UserLevel, XpSource,
};
pub use community::role_panel::{AutoRole, RolePanel, RolePanelDetail, RolePanelEntry};
pub use community::voice_channel::{
    VoiceChannel, VoiceChannelBan, VoiceChannelCoAdmin, VoiceChannelConfig, VoiceChannelDetail,
    VoiceChannelInviteLink, VoiceChannelTheme, VoiceChannelWhitelistEntry,
};

// ── coude (le jeu) ─────────────────────────────────────────────────────────
pub use coude::bounty::{
    ActiveBounty, BountyStatus, BOUNTY_AUTO_OPEN_STREAK_THRESHOLD, BOUNTY_INITIAL_AMOUNT,
    BOUNTY_MIN_CONTRIBUTION,
};
pub use coude::branding::{
    coude_bet_footer, coude_combat_footer, COUDE_TAGLINE, COUDE_TAGLINE_SHORT, SENTINEL_TAGLINE,
};
pub use coude::coalition::{
    apply_coalition_penalty, ActiveCoalition, CoalitionMember, CoalitionStatus,
    COALITION_COST_PER_MEMBER, COALITION_DURATION_HOURS, COALITION_GAIN_MULTIPLIER,
    COALITION_MIN_MEMBERS,
};
pub use coude::combat_flavor::{pick_flavor_line, FLAVOR_LINES, FLAVOR_LINE_PROBABILITY};
pub use coude::combat_outcome_flags::{
    detect_outcome_flags, CombatOutcomeFlags, CLUTCH_HP_PCT_MAX, COMEBACK_HP_PCT_MAX,
    COMEBACK_MIN_ROUNDS_LOW_HP, PERFECT_HP_PCT_MIN,
};
pub use coude::combat_resolution_rules::{
    apply_insurance_to_loss, compute_combat_xp, format_bet_payout_lines, CombatXpAwards,
    InsuranceAdjustment, COMBAT_XP_LOSER, COMBAT_XP_WINNER_BASE, COMBAT_XP_WINNER_UNDERDOG,
    UNDERDOG_LEVEL_GAP,
};
pub use coude::coude_balance::{CoudeBalanceParams, DoubleCoupMode};
pub use coude::coude_bet::{
    calculate_bet_resolution, BetPayout, BetPayoutOutcome, BetResolutionPlan, CoudeBet,
    FighterBetBonus as CoudeFighterBetBonus, NewCoudeBet, RefundSummary,
};
pub use coude::coude_cashbox::{
    CashboxRedistribution, CashboxRedistributionEntry, CashboxSource, CoudeCashbox,
};
pub use coude::coude_combat::{CombatResolution, CoudeCombat, NewCoudeCombat};
pub use coude::coude_combat_validation::{
    check_min_hp_pct, check_surprise_hp_pct, validate_new_combat,
};
pub use coude::coude_economy::{clamp_steal_amount, clamp_steal_fail_penalty, ClampedSteal};
pub use coude::coude_expire::cowardice_penalty;
pub use coude::coude_heist::{
    compute_success_chance, find_heist_tool, CoudeHeistAttempt, CoudePrisonState, HeistOutcome,
    HeistToolDef, HEIST_BASE_SUCCESS_PERCENT, HEIST_COOLDOWN_DAYS, HEIST_GAIN_MAX_PERCENT,
    HEIST_GAIN_MIN_PERCENT, HEIST_ITEM_BONUS_PERCENT, HEIST_MAX_SUCCESS_PERCENT,
    HEIST_PRISON_HOURS, HEIST_TOOLS,
};
pub use coude::coude_inventory::{CoudeInsurance, CoudeInventoryItem, CoudePrime, NewCoudePrime};
pub use coude::coude_limits::{
    DEFAULT_COUDE_COMBATS_LIMIT, DEFAULT_COUDE_OPPONENT_COUNT,
    DEFAULT_COUDE_SOCIAL_LEADERBOARD_LIMIT,
};
pub use coude::coude_player::{
    title_for_level as coude_title_for_level, xp_for_level as coude_xp_for_level, CombatStat,
    CoudePlayer, XpProgress, COUDE_MAX_LEVEL,
};
pub use coude::coude_purge::COUDE_PURGE_TABLES;
pub use coude::coude_social::{
    clamp_leaderboard_limit, daily_chaos_amount, CoudeCurrentSeason, CoudeEvent,
    CoudeLeaderboardEntry, DailyChaosOutcome, LeaderboardCategory, NewDailyChaos,
    DAILY_CHAOS_MAX, DEFAULT_CHAOS_PERCENT, LEADERBOARD_MAX_LIMIT, LEADERBOARD_MIN_LIMIT,
    MIN_COINS_ELIGIBLE,
};
pub use coude::coude_steal_boost::{
    find_boost_item, sum_roll_bonus_for_active_keys, CoudeStealBoost, StealBoostDuration,
    StealBoostItemDef, STEAL_BOOST_ITEMS,
};
pub use coude::coude_steal_protection::{
    find_protection_item, CoudeStealProtection, StealProtectionDuration, StealProtectionItemDef,
    STEAL_PROTECTION_ITEMS,
};
pub use coude::coude_steal_roll::{
    steal_pct_range_bp, STEAL_D20_MAX, STEAL_D20_MIN, STEAL_PCT_ACTIVE_MAX_BP,
    STEAL_PCT_ACTIVE_MIN_BP, STEAL_PCT_AFK_MAX_BP, STEAL_PCT_AFK_MIN_BP,
};
pub use coude::coude_taunt::{
    build_taunt_event, build_taunt_event_single, crossed_threshold, nickname_suffix_for,
    CoudeTauntsConfig, StreakKind, TauntEvent, TAUNT_THRESHOLDS,
};
pub use coude::coude_tournament::{
    current_week_bounds, estimate_tournament_prize_pool, week_bounds_for,
    TOURNAMENT_PRIZE_POOL_PERCENT,
};
pub use coude::coude_travaux::{
    fail_flavor_at, success_flavor_at, task_at, TravauxTask, TRAVAUX_COINS_MAX, TRAVAUX_COINS_MIN,
    TRAVAUX_COOLDOWN_KEY, TRAVAUX_COOLDOWN_SECS, TRAVAUX_FAIL_FLAVORS, TRAVAUX_SUCCESS_FLAVORS,
    TRAVAUX_SUCCESS_PCT, TRAVAUX_TASKS, TRAVAUX_XP_PER_TASK,
};
pub use coude::cowardice_relief::{should_count_as_cowardice, COWARDICE_RELIEF_HP_PCT};
pub use coude::curse::{
    apply_banana_to_d20, apply_insomnia_to_taunt_weight, apply_leaky_wallet, lift_cost,
    pick_curse_by_index, poison_redirect_amount, ActiveCurse, CurseKind, BANANA_FAIL_PROBABILITY,
    CURSE_COST_COINS, CURSE_DURATION_HOURS, CURSE_LIFT_MULTIPLIER, FAUSSE_ASSURANCE_FEE_COINS,
    INSOMNIA_TAUNT_MULTIPLIER, LEAKY_WALLET_FEE_COINS, POISON_GAIN_REDIRECT_PCT,
    SLOWNESS_DELAY_SECS,
};
pub use coude::fake_spectators::{
    format_spectator_chat, pick_spectator_chat, SPECTATOR_COUNT_MAX, SPECTATOR_COUNT_MIN,
    SPECTATOR_LINES, SPECTATOR_USERNAMES,
};
pub use coude::lucky_shield::{
    apply_lucky_shield, apply_lucky_shield_with_multiplier,
    should_preserve_win_streak_after_shielded_defeat, LUCKY_SHIELD_LOSS_MULTIPLIER,
};
pub use coude::mythic_events::{
    format_mythic_announce, roll_mythic_event, MythicEvent, MYTHIC_EVENTS,
};
pub use coude::prestige::{
    can_prestige, prestige_gain_multiplier, prestige_gain_multiplier_with_params, prestige_stars,
    PRESTIGE_GAIN_BONUS_PCT, PRESTIGE_MAX_COUNT, PRESTIGE_UNLOCK_LEVEL,
};
pub use coude::refusal_count::{RefusalCount, HONOR_DEBT_THRESHOLD};
pub use coude::safety_net::{
    boost_bet_gain as safety_net_boost_bet_gain,
    boost_bet_gain_with_multiplier as safety_net_boost_bet_gain_with_multiplier,
    reduce_loss as safety_net_reduce_loss,
    reduce_loss_with_multiplier as safety_net_reduce_loss_with_multiplier,
    should_trigger as safety_net_should_trigger, ActiveSafetyNet,
    SAFETY_NET_BET_GAIN_MULTIPLIER, SAFETY_NET_DURATION_HOURS, SAFETY_NET_LOSS_MULTIPLIER,
    SAFETY_NET_TRIGGER_COINS,
};
pub use coude::season_theme::{
    apply_season_braquage_cooldown, compute_season_steal_bonus, season_chaos_multiplier,
    season_tank_def_bonus_pct, season_theme_by_key, theme_for_season, SeasonTheme,
    CURRENT_SEASON_THEME_CONFIG_KEY, SEASON_THEMES,
};
pub use coude::smart_default_bet::{quick_bet_buttons, suggest_default_bet, DEFAULT_BET_PCT};
pub use coude::tout_ou_rien::{
    coin_delta as tout_ou_rien_delta, resolve_outcome as tout_ou_rien_resolve, ToutOuRienOutcome,
    TOUT_OU_RIEN_COOLDOWN_KEY, TOUT_OU_RIEN_COOLDOWN_SECS, TOUT_OU_RIEN_LOSS_KEEP_PCT,
    TOUT_OU_RIEN_WIN_MULTIPLIER, TOUT_OU_RIEN_WIN_PROBABILITY,
};
pub use coude::tout_ou_rien_log::{ToutOuRienLogEntry, ToutOuRienLogOutcome, ToutOuRienUserStats};
pub use coude::ultimate::{ultimate_ready, UltimateKind, UltimateState, ULTIMATE_UNLOCK_LEVEL};
pub use coude::vendetta::{
    apply_revenge_bonus, ActiveVendetta, VendettaStatus, VENDETTA_BOURREAU_SUFFIX_PREFIX,
    VENDETTA_WINDOW_HOURS, VENDETTA_WIN_BONUS_MULTIPLIER,
};

// ── moderation ─────────────────────────────────────────────────────────────
pub use moderation::automod_review::{AppliedAction, AutomodReview, NewAutomodReview, SuggestedAction};
pub use moderation::infraction::Infraction;
pub use moderation::moderation_action::{ModerationAction, UserModerationHistory};
pub use moderation::moderation_review::{
    is_valid_review_status, resolve_mute_duration, truncate_review_text, validate_evidence_url,
    DEFAULT_MUTE_DURATION_SECS, MAX_EVIDENCE_URL_LEN, MAX_REVIEW_TEXT_LEN, VALID_REVIEW_STATUSES,
};
pub use moderation::purge::{
    validate_purge_days_allow_zero, validate_purge_days_strictly_positive,
};
pub use moderation::sanction_reminder::SanctionReminder;
pub use moderation::strikes::{StrikeConfig, StrikeResult, StrikeThreshold, UserStrike};
pub use moderation::user_note::UserNote;

// ── system (transverses) ───────────────────────────────────────────────────
// `analytics` reste accessible aussi en tant que sous-module qualifie
// (`crate::domain::entities::analytics::Foo`) :
pub use system::analytics;

pub use system::bot_config::{BotDefinition, BotGuildConfig};
pub use system::config_parsers::{is_worker_service, parse_bool_config, parse_i64_config};
pub use system::discord_role::{parse_discord_permissions_bitfield, DiscordRole};
pub use system::guild::Guild;
pub use system::job_whitelists::{
    is_valid_ai_job_type, is_valid_export_format, is_valid_export_job_type, VALID_AI_JOB_TYPES,
    VALID_EXPORT_FORMATS, VALID_EXPORT_JOB_TYPES,
};
pub use system::log_entry::LogEntry;
pub use system::rbac::{
    is_owner_self_demotion, truncate_display_name, would_revoke_last_owner, RBAC_DISPLAY_NAME_MAX,
};
pub use system::rule::Rule;
pub use system::ticket::{Ticket, TicketDetail, TicketMessage};
