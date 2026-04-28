// Bounded contexts.
pub mod ai;
pub mod audit;
pub mod casino;
pub mod community;
pub mod coude;
pub mod moderation;
pub mod system;

// Re-exports preservant l'API publique historique.

// ── ai ─────────────────────────────────────────────────────────────────────
pub use ai::analyze_image_service::AnalyzeImageService;
pub use ai::analyze_message_service::{score_classifications, AnalyzeMessageService};

// ── audit ──────────────────────────────────────────────────────────────────
pub use audit::manage_audit_logs_service::ManageAuditLogsService;
pub use audit::manage_discord_action_messages_service::ManageDiscordActionMessagesService;
pub use audit::manage_security_service::ManageSecurityService;
pub use audit::manage_stats_service::ManageStatsService;
pub use audit::manage_watched_users_service::ManageWatchedUsersService;

// ── casino ─────────────────────────────────────────────────────────────────
pub use casino::blackjack_service::{BlackjackActionResult, BlackjackService};
pub use casino::manage_slot_service::{self, ManageSlotService};
pub use casino::manage_wallet_service::ManageWalletService;
pub use casino::manage_wheel_service::{self, ManageWheelService};

// ── community ──────────────────────────────────────────────────────────────
pub use community::manage_conduct_service::ManageConductService;
pub use community::manage_levels_service::ManageLevelsService;
pub use community::manage_members_service::ManageMembersService;
pub use community::manage_role_panels_service::ManageRolePanelsService;
pub use community::manage_welcome_config_service::ManageWelcomeConfigService;
pub use community::voice_channels::ManageVoiceChannelsService;

// ── coude ──────────────────────────────────────────────────────────────────
pub use coude::coude_guild_settings::{self, CoudeGuildSettings};
pub use coude::expire_combats_batch_service::ExpireCombatsBatchService;
pub use coude::manage_coude_bets_service::ManageCoudeBetsService;
pub use coude::manage_coude_cashbox_service::ManageCoudeCashboxService;
pub use coude::manage_coude_catalog_service::ManageCoudeCatalogService;
pub use coude::manage_coude_combats_service::ManageCoudeCombatsService;
pub use coude::manage_coude_curses_service::{self, ManageCoudeCursesService};
pub use coude::manage_coude_economy_service::ManageCoudeEconomyService;
pub use coude::manage_coude_heist_service::ManageCoudeHeistService;
pub use coude::manage_coude_inventory_service::ManageCoudeInventoryService;
pub use coude::manage_coude_players_service::ManageCoudePlayersService;
pub use coude::manage_coude_safety_net_service::{self, ManageCoudeSafetyNetService};
pub use coude::manage_coude_social_service::ManageCoudeSocialService;
pub use coude::manage_coude_steal_boosts_service::ManageCoudeStealBoostsService;
pub use coude::manage_coude_steal_protections_service::ManageCoudeStealProtectionsService;
pub use coude::manage_coude_taunts_service::ManageCoudeTauntsService;
pub use coude::manage_coude_vendetta_service::{self, ManageCoudeVendettaService};
pub use coude::play_tout_ou_rien_service::{self, PlayToutOuRienService};
pub use coude::play_travaux_service::{self, PlayTravauxService};
pub use coude::resolve_betting_batch_service::ResolveBettingBatchService;
pub use coude::resolve_combat_now_service::ResolveCombatNowService;
pub use coude::resolve_friendly_duel_service::ResolveFriendlyDuelService;
pub use coude::roll_steal_service::{self, RollStealService};

// ── moderation ─────────────────────────────────────────────────────────────
pub use moderation::manage_automod_reviews_service::ManageAutomodReviewsService;
pub use moderation::manage_infractions_service::ManageInfractionsService;
pub use moderation::manage_moderation_service::ManageModerationService;
pub use moderation::manage_notes_service::ManageNotesService;
pub use moderation::manage_reminders_service::ManageRemindersService;
pub use moderation::manage_rules_service::ManageRulesService;
pub use moderation::manage_strikes_service::ManageStrikesService;

// ── system ─────────────────────────────────────────────────────────────────
pub use system::export_service::{self, ExecuteExportUseCase, ExportService};
pub use system::manage_tickets_service::ManageTicketsService;
