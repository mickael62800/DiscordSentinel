use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::domain::entities::{
    CoudeCurrentSeason, CoudeEvent, CoudeLeaderboardEntry, LeaderboardCategory, NewDailyChaos,
};
use crate::domain::errors::DomainError;

/// Repository pour les fonctionnalités "sociales" Coup de Coude :
/// cooldowns, classements, événements serveur, daily chaos, saisons.
#[async_trait]
pub trait CoudeSocialRepository: Send + Sync {
    // ── Cooldowns ──

    /// Retourne la date d'expiration du cooldown actif (`> NOW()`), ou `None`
    /// si aucun n'est en cours.
    async fn get_cooldown(
        &self,
        guild_id: &str,
        user_id: &str,
        action: &str,
    ) -> Result<Option<DateTime<Utc>>, DomainError>;

    /// Upsert un cooldown : `expires_at = NOW() + duration_secs`.
    async fn set_cooldown(
        &self,
        guild_id: &str,
        user_id: &str,
        action: &str,
        duration_secs: i64,
    ) -> Result<(), DomainError>;

    // ── Leaderboard ──

    async fn leaderboard(
        &self,
        guild_id: &str,
        category: LeaderboardCategory,
        limit: i64,
    ) -> Result<Vec<CoudeLeaderboardEntry>, DomainError>;

    // ── Événements ──

    async fn list_active_events(&self, guild_id: &str) -> Result<Vec<CoudeEvent>, DomainError>;

    // ── Daily chaos ──

    async fn log_daily_chaos(&self, chaos: NewDailyChaos) -> Result<(), DomainError>;

    // ── Saison ──

    /// Renvoie la saison active du guild. Bootstrap automatique si aucune
    /// saison n'existe : insertion de la saison suivante (numéro incrémenté).
    async fn get_or_bootstrap_current_season(
        &self,
        guild_id: &str,
    ) -> Result<CoudeCurrentSeason, DomainError>;
}
