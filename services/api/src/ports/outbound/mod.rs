// Bounded contexts.
pub mod audit;
pub mod casino;
pub mod community;
pub mod coude;
pub mod moderation;
pub mod system;

// Re-exports preservant l'API publique historique.

// ── audit ──────────────────────────────────────────────────────────────────
pub use audit::analytics_repository::AnalyticsRepository;
pub use audit::audit_log_repository::AuditLogRepository;
pub use audit::discord_action_message_repository::DiscordActionMessageRepository;
pub use audit::modstats_repository::{ModeratorStat, ModstatsRepository};
pub use audit::security_event_repository::SecurityEventRepository;
pub use audit::stats_repository::StatsRepository;
pub use audit::user_activity_repository::UserActivityRepository;
pub use audit::watched_user_repository::WatchedUserRepository;

// ── casino ─────────────────────────────────────────────────────────────────
pub use casino::blackjack_repository::BlackjackRepository;
pub use casino::blackjack_table_repository::{
    BlackjackTable, BlackjackTablePlayer, BlackjackTableRepository,
};
pub use casino::game_repository::{Game, GamePanel, GameRepository};
pub use casino::slot_repository::SlotRepository;
pub use casino::wallet_repository::WalletRepository;
pub use casino::wheel_repository::WheelRepository;

// ── community ──────────────────────────────────────────────────────────────
pub use community::conduct_repository::ConductRepository;
pub use community::daily_activity_repository::DailyActivityRepository;
pub use community::discord_role_repository::DiscordRoleRepository;
pub use community::level_repository::LevelRepository;
pub use community::member_repository::MemberRepository;
pub use community::role_panel_repository::RolePanelRepository;
pub use community::temp_role_repository::{TempRole, TempRoleRepository};
pub use community::voice_channel_repository::VoiceChannelRepository;
pub use community::welcome_config_repository::{WelcomeConfigData, WelcomeConfigRepository};

// ── coude ──────────────────────────────────────────────────────────────────
pub use coude::combat_query_repository::CombatQueryRepository;
pub use coude::coude_bet_repository::CoudeBetRepository;
pub use coude::coude_bounty_repository::CoudeBountyRepository;
pub use coude::coude_cashbox_repository::CoudeCashboxRepository;
pub use coude::coude_coalition_repository::CoudeCoalitionRepository;
pub use coude::coude_combat_repository::CoudeCombatRepository;
pub use coude::coude_curses_repository::CoudeCursesRepository;
pub use coude::coude_economy_repository::CoudeEconomyRepository;
pub use coude::coude_flavor_templates_repository::CoudeFlavorTemplatesRepository;
pub use coude::coude_heist_repository::CoudeHeistRepository;
pub use coude::coude_inventory_repository::CoudeInventoryRepository;
pub use coude::coude_player_repository::CoudePlayerRepository;
pub use coude::coude_refusal_count_repository::CoudeRefusalCountRepository;
pub use coude::coude_safety_net_repository::CoudeSafetyNetRepository;
pub use coude::coude_social_repository::CoudeSocialRepository;
pub use coude::coude_steal_boost_repository::CoudeStealBoostRepository;
pub use coude::coude_steal_protection_repository::CoudeStealProtectionRepository;
pub use coude::coude_taunts_repository::CoudeTauntsRepository;
pub use coude::coude_tout_ou_rien_repository::CoudeToutOuRienRepository;
pub use coude::coude_ultimate_repository::CoudeUltimateRepository;
pub use coude::coude_vendetta_repository::CoudeVendettaRepository;
pub use coude::sponsorship_repository::{Sponsorship, SponsorshipRepository};

// ── moderation ─────────────────────────────────────────────────────────────
pub use moderation::automod_review_repository::AutomodReviewRepository;
pub use moderation::evidence_repository::{EvidenceEntry, EvidenceRepository};
pub use moderation::infraction_repository::InfractionRepository;
pub use moderation::moderation_repository::ModerationRepository;
pub use moderation::notes_repository::NotesRepository;
pub use moderation::pending_action_repository::{PendingAction, PendingActionRepository};
pub use moderation::reminder_repository::ReminderRepository;
pub use moderation::review_repository::{ReviewEntry, ReviewRepository};
pub use moderation::rule_repository::RuleRepository;
pub use moderation::strike_repository::StrikeRepository;

// ── system ─────────────────────────────────────────────────────────────────
pub use system::bot_config_repository::BotConfigRepository;
pub use system::cache::CachePort;
pub use system::guild_repository::GuildRepository;
pub use system::log_repository::LogRepository;
pub use system::ticket_repository::TicketRepository;
