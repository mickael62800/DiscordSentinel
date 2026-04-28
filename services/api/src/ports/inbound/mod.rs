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
pub use ai::analyze_image::{AnalyzeImageCommand, AnalyzeImageUseCase};
pub use ai::analyze_message::{
    AnalyzeMessageCommand, AnalyzeMessageUseCase, ContextMessageEntry,
};

// ── audit ──────────────────────────────────────────────────────────────────
pub use audit::manage_audit_logs::{self, CreateAuditLogCommand, ManageAuditLogsUseCase};
pub use audit::manage_discord_action_messages::{
    self, ManageDiscordActionMessagesUseCase,
};
pub use audit::manage_security::{
    AnalyzeNewMemberCommand, ManageSecurityUseCase, ReportSecurityEventCommand, SecurityDecision,
};
pub use audit::manage_stats::{self, ManageStatsUseCase};
pub use audit::manage_watched_users::{self, ManageWatchedUsersUseCase};

// ── casino ─────────────────────────────────────────────────────────────────
pub use casino::manage_slot::{self, ManageSlotUseCase, SpinCommand, SpinResult};
pub use casino::manage_wallet::{self, ManageWalletUseCase, WalletMutation};
pub use casino::manage_wheel::{self, ManageWheelUseCase, WheelSpinCommand, WheelSpinResult};

// ── community ──────────────────────────────────────────────────────────────
pub use community::manage_conduct::{
    AddPointsCommand, DeductPointsCommand, ManageConductUseCase, SaveConductConfigCommand,
};
pub use community::manage_levels::{self, ManageLevelsUseCase};
pub use community::manage_members::{
    self, ManageMembersUseCase, RegisterMemberCommand, SyncMembersCommand, UpdateMemberCommand,
};
pub use community::manage_role_panels::{self, ManageRolePanelsUseCase};
pub use community::manage_voice_channels::{
    BanFromChannelCommand, CreateInviteLinkCommand, CreateThemeCommand, CreateVoiceChannelCommand,
    ManageCoAdminCommand, ManageVoiceChannelsUseCase, ManageWhitelistCommand,
    TransferOwnershipCommand, UpdateVoiceChannelCommand, UseInviteLinkCommand,
};
pub use community::manage_welcome_config::{
    self, ManageWelcomeConfigUseCase, WelcomeConfigPatch,
};

// ── coude ──────────────────────────────────────────────────────────────────
pub use coude::expire_combats_batch::{
    self, ExpireCombatsBatchUseCase, ExpiredCombatOutput,
};
pub use coude::manage_coude_bets::{
    self, ManageCoudeBetsUseCase, PlaceBetOutcome, ResolveBetsOutcome,
};
pub use coude::manage_coude_cashbox::{
    self, ManageCoudeCashboxUseCase, RedistributionOutcome,
};
pub use coude::manage_coude_catalog::{
    self, AntiTheftItemInfo, ClassInfo, CoudeCatalog, LevelEntry, ManageCoudeCatalogUseCase,
    MatchmakingBucket, ShopItemInfo,
};
pub use coude::manage_coude_combats::{self, ManageCoudeCombatsUseCase};
pub use coude::manage_coude_curses::{self, CastedCurse, ManageCoudeCursesUseCase};
pub use coude::manage_coude_economy::{
    self, ManageCoudeEconomyUseCase, StealOutcome,
};
pub use coude::manage_coude_heist::{
    self, HeistCooldownStatus, ManageCoudeHeistUseCase, PrisonStatusInfo,
};
pub use coude::manage_coude_inventory::{self, ManageCoudeInventoryUseCase};
pub use coude::manage_coude_players::{self, ManageCoudePlayersUseCase};
pub use coude::manage_coude_safety_net::{self, ManageCoudeSafetyNetUseCase};
pub use coude::manage_coude_social::{self, ManageCoudeSocialUseCase};
pub use coude::manage_coude_steal_boosts::{self, ManageCoudeStealBoostsUseCase};
pub use coude::manage_coude_steal_protections::{
    self, ManageCoudeStealProtectionsUseCase, StealProtectionTrigger,
};
pub use coude::manage_coude_taunts::{self, ManageCoudeTauntsUseCase};
pub use coude::manage_coude_vendetta::{self, ManageCoudeVendettaUseCase};
pub use coude::play_tout_ou_rien::{
    self, PlayToutOuRienCommand, PlayToutOuRienUseCase, ToutOuRienResolution,
    MIN_BALANCE_FOR_PLAY,
};
pub use coude::play_travaux::{
    self, PlayTravauxCommand, PlayTravauxUseCase, TravauxResolution,
};
pub use coude::resolve_betting_batch::{
    self, ResolveBettingBatchUseCase, ResolvedBettingCombatOutput,
};
pub use coude::resolve_combat_now::{
    self, ResolveCombatNowOutput, ResolveCombatNowUseCase, ResolvedCombatEmbedField,
    VendettaHumiliation,
};
pub use coude::resolve_friendly_duel::{
    self, FriendlyDuelInput, FriendlyDuelOutput, ResolveFriendlyDuelUseCase,
};
pub use coude::roll_steal::{self, RollStealCommand, RollStealUseCase, StealRoll};

// ── moderation ─────────────────────────────────────────────────────────────
pub use moderation::manage_automod_reviews::{
    self, ManageAutomodReviewsUseCase, ResolveAutomodReviewCommand,
};
pub use moderation::manage_infractions::{InfractionFilters, ManageInfractionsUseCase};
pub use moderation::manage_moderation::{
    LogModerationCommand, LoggedModerationAction, ManageModerationUseCase,
};
pub use moderation::manage_notes::{self, AddNoteCommand, ManageNotesUseCase};
pub use moderation::manage_reminders::{CreateReminderCommand, ManageRemindersUseCase};
pub use moderation::manage_rules::{CreateRuleCommand, ManageRulesUseCase};
pub use moderation::manage_strikes::{
    AddStrikeCommand, ManageStrikesUseCase, SaveStrikeConfigCommand,
};

// ── system ─────────────────────────────────────────────────────────────────
pub use system::manage_tickets::{
    AssignTicketCommand, CreateTicketCommand, ManageTicketsUseCase, ReplyTicketCommand,
    UpdateTicketChannelCommand,
};
