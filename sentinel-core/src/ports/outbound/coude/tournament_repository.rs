//! Port outbound pour le tournoi hebdomadaire "Coup de Coude" (migration 139).
//!
//! Regroupe les requetes SQL d'agregation utilisees pour construire le
//! classement du tournoi courant et l'historique. L'assemblage (rangs, prize
//! pool) est de la logique metier et vit dans `ManageTournamentsService`.

use async_trait::async_trait;
use chrono::DateTime;
use chrono::Utc;

use crate::domain::entities::coude::tournament::PastTournament;
use crate::domain::errors::DomainError;

#[async_trait]
pub trait TournamentRepository: Send + Sync {
    /// Somme des `wallet_transactions.amount` par membre sur la fenetre
    /// `[week_start, week_end]`, triee par gain net decroissant, plafonnee a
    /// `limit`. Renvoie `(user_id, net_gain)`.
    async fn weekly_net_gains(
        &self,
        guild_id: &str,
        week_start: DateTime<Utc>,
        week_end: DateTime<Utc>,
        limit: i64,
    ) -> Result<Vec<(String, i64)>, DomainError>;

    /// Resout les pseudos de `user_ids` via `user_wallets`. Ne renvoie que les
    /// lignes trouvees : `(user_id, username)`.
    async fn usernames(
        &self,
        guild_id: &str,
        user_ids: &[String],
    ) -> Result<Vec<(String, String)>, DomainError>;

    /// Solde courant de la caisse communautaire (`coude_cashbox`) si elle
    /// existe, pour l'estimation du prize pool.
    async fn cashbox_balance(&self, guild_id: &str) -> Result<Option<i64>, DomainError>;

    /// Historique des tournois passes d'une guild (les plus recents d'abord),
    /// plafonne a `limit`.
    async fn list_past_tournaments(
        &self,
        guild_id: &str,
        limit: i64,
    ) -> Result<Vec<PastTournament>, DomainError>;
}
