use crate::domain::entities::coude::social::Event;
use crate::domain::entities::coude::social::LeaderboardCategory;
use crate::domain::entities::coude::social::LeaderboardEntry;
use crate::domain::entities::coude::social::NewDailyChaos;
use crate::domain::entities::coude::social::Season;
use crate::domain::errors::DomainError;
use async_trait::async_trait;
use chrono::DateTime;
use chrono::Utc;

/// Repository pour les fonctionnalités "sociales" Coup de Coude :
/// cooldowns, classements, événements serveur, daily chaos, saisons.
#[async_trait]
pub trait SocialRepository: Send + Sync {
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

    /// Pose atomiquement un cooldown SEULEMENT s'il n'en existe pas deja un
    /// actif (insert-if-absent). Retourne `true` si CET appel a cree le
    /// cooldown (claim gagne), `false` si un cooldown actif existait deja.
    ///
    /// Sert de verrou anti-TOCTOU : deux appels concurrents -> un seul
    /// obtient `true`. `ttl_secs` calcule l'expiration comme `set_cooldown`.
    async fn try_claim_cooldown(
        &self,
        guild_id: &str,
        user_id: &str,
        key: &str,
        ttl_secs: i64,
    ) -> Result<bool, DomainError>;

    /// Libere un claim pose par `try_claim_cooldown` (DELETE de la ligne).
    /// Utilise pour relacher le verrou si l'operation protegee echoue.
    async fn clear_cooldown(
        &self,
        guild_id: &str,
        user_id: &str,
        key: &str,
    ) -> Result<(), DomainError>;

    // ── Leaderboard ──

    async fn leaderboard(
        &self,
        guild_id: &str,
        category: LeaderboardCategory,
        limit: i64,
    ) -> Result<Vec<LeaderboardEntry>, DomainError>;

    // ── Événements ──

    async fn list_active_events(&self, guild_id: &str) -> Result<Vec<Event>, DomainError>;

    // ── Daily chaos ──

    async fn log_daily_chaos(&self, chaos: NewDailyChaos) -> Result<(), DomainError>;

    /// Nombre de chaos deja emis aujourd'hui pour cette guild.
    async fn count_daily_chaos_today(&self, guild_id: &str) -> Result<i64, DomainError>;

    // ── Saison ──

    /// Renvoie la saison active du guild. Bootstrap automatique si aucune
    /// saison n'existe : insertion de la saison suivante (numéro incrémenté).
    async fn get_or_bootstrap_current_season(&self, guild_id: &str) -> Result<Season, DomainError>;
}
