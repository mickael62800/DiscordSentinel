use crate::domain::errors::DomainError;

/// Helper centralise : convertit une erreur sqlx en DomainError::Internal.
/// Utilise par tous les repositories Postgres.
pub(crate) fn pg_err(e: sqlx::Error) -> DomainError {
    DomainError::Internal(e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pg_err_wraps_sqlx_row_not_found_into_internal() {
        let err = pg_err(sqlx::Error::RowNotFound);
        match err {
            DomainError::Internal(msg) => {
                assert!(!msg.is_empty());
            }
            other => panic!("expected Internal, got {other:?}"),
        }
    }

    #[test]
    fn pg_err_wraps_protocol_error() {
        let err = pg_err(sqlx::Error::Protocol("connexion fermee".into()));
        match err {
            DomainError::Internal(msg) => assert!(msg.contains("connexion fermee")),
            other => panic!("expected Internal, got {other:?}"),
        }
    }

    #[test]
    fn pg_err_message_matches_display() {
        let source_err = sqlx::Error::PoolClosed;
        let display = source_err.to_string();
        let wrapped = pg_err(source_err);
        match wrapped {
            DomainError::Internal(msg) => assert_eq!(msg, display),
            other => panic!("expected Internal, got {other:?}"),
        }
    }
}

mod bot_config_repository;
mod conduct_repository;
mod guild_repository;
mod infraction_repository;
mod log_repository;
mod moderation_repository;
mod rule_repository;
mod security_event_repository;
mod stats_repository;
mod ticket_repository;
mod voice_channel_repository;
mod watched_user_repository;
mod audit_log_repository;
mod level_repository;
mod daily_activity_repository;
mod role_panel_repository;
mod analytics_repository;
mod discord_role_repository;
mod notes_repository;
mod reminder_repository;
mod strike_repository;

pub use analytics_repository::PgAnalyticsRepository;
pub use audit_log_repository::PgAuditLogRepository;
pub use daily_activity_repository::PgDailyActivityRepository;
pub use role_panel_repository::PgRolePanelRepository;
pub use level_repository::PgLevelRepository;
pub use bot_config_repository::PgBotConfigRepository;
pub use conduct_repository::PgConductRepository;
pub use guild_repository::PgGuildRepository;
pub use infraction_repository::PgInfractionRepository;
pub use log_repository::PgLogRepository;
pub use moderation_repository::PgModerationRepository;
pub use rule_repository::PgRuleRepository;
pub use security_event_repository::PgSecurityEventRepository;
pub use stats_repository::PgStatsRepository;
pub use ticket_repository::PgTicketRepository;
pub use voice_channel_repository::PgVoiceChannelRepository;
pub use watched_user_repository::PgWatchedUserRepository;
pub use discord_role_repository::PgDiscordRoleRepository;
pub use notes_repository::PgNotesRepository;
pub use reminder_repository::PgReminderRepository;
pub use strike_repository::PgStrikeRepository;

mod member_repository;
pub use member_repository::PgMemberRepository;

mod wallet_repository;
pub use wallet_repository::PgWalletRepository;

pub(crate) mod wallet_tx_log;

mod blackjack_repository;
pub use blackjack_repository::PgBlackjackRepository;

mod coude_player_repository;
pub use coude_player_repository::PgCoudePlayerRepository;

mod coude_combat_repository;
pub use coude_combat_repository::PgCoudeCombatRepository;

mod coude_bet_repository;
pub use coude_bet_repository::PgCoudeBetRepository;

mod coude_economy_repository;
pub use coude_economy_repository::PgCoudeEconomyRepository;

mod coude_inventory_repository;
pub use coude_inventory_repository::PgCoudeInventoryRepository;

mod coude_social_repository;
pub use coude_social_repository::PgCoudeSocialRepository;

mod coude_cashbox_repository;
pub use coude_cashbox_repository::PgCoudeCashboxRepository;

mod coude_steal_protection_repository;
pub use coude_steal_protection_repository::PgCoudeStealProtectionRepository;

mod coude_steal_boost_repository;
pub use coude_steal_boost_repository::PgCoudeStealBoostRepository;

mod coude_taunts_repository;
pub use coude_taunts_repository::PgCoudeTauntsRepository;

mod coude_heist_repository;
pub use coude_heist_repository::PgCoudeHeistRepository;

mod user_activity_repository;
pub use user_activity_repository::PgUserActivityRepository;

mod welcome_config_repository;
pub use welcome_config_repository::PgWelcomeConfigRepository;

mod evidence_repository;
pub use evidence_repository::PgEvidenceRepository;

mod review_repository;
pub use review_repository::PgReviewRepository;

mod modstats_repository;
pub use modstats_repository::PgModstatsRepository;

mod game_repository;
pub use game_repository::PgGameRepository;

mod sponsorship_repository;
pub use sponsorship_repository::PgSponsorshipRepository;

mod temp_role_repository;
pub use temp_role_repository::PgTempRoleRepository;

mod pending_action_repository;
pub use pending_action_repository::PgPendingActionRepository;

mod blackjack_table_repository;
pub use blackjack_table_repository::PgBlackjackTableRepository;

mod slot_repository;
pub use slot_repository::PgSlotRepository;
