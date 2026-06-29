use crate::domain::entities::coude::social::DailyChaosOutcome;
use crate::domain::entities::coude::social::Event;
use crate::domain::entities::coude::social::LeaderboardCategory;
use crate::domain::entities::coude::social::LeaderboardEntry;
use crate::domain::entities::coude::social::NewDailyChaos;
use crate::domain::entities::coude::social::Season;
use crate::domain::errors::DomainError;
use async_trait::async_trait;
use chrono::DateTime;
use chrono::Utc;

/// Use case "fonctionnalités sociales Coup de Coude".
#[async_trait]
pub trait ManageCoudeSocialUseCase: Send + Sync {
    async fn check_cooldown(
        &self,
        guild_id: &str,
        user_id: &str,
        action: &str,
    ) -> Result<Option<DateTime<Utc>>, DomainError>;

    async fn set_cooldown(
        &self,
        guild_id: &str,
        user_id: &str,
        action: &str,
        duration_secs: i64,
    ) -> Result<(), DomainError>;

    async fn leaderboard(
        &self,
        guild_id: &str,
        category: LeaderboardCategory,
        limit: i64,
    ) -> Result<Vec<LeaderboardEntry>, DomainError>;

    async fn list_active_events(&self, guild_id: &str) -> Result<Vec<Event>, DomainError>;

    async fn log_daily_chaos(&self, chaos: NewDailyChaos) -> Result<(), DomainError>;

    /// Tente de declencher un chaos journalier. L'API decide de tout :
    /// - Compte les chaos deja emis aujourd'hui (cap 5)
    /// - Tire 2 joueurs aleatoires avec assez de coins
    /// - Calcule le montant (20% des coins de la victime)
    /// - Fait le transfert + log
    /// Retourne None si pas de chaos (cap atteint, pas de joueurs eligibles).
    async fn trigger_daily_chaos(
        &self,
        guild_id: &str,
    ) -> Result<Option<DailyChaosOutcome>, DomainError>;

    async fn current_season(&self, guild_id: &str) -> Result<Season, DomainError>;
}
