use async_trait::async_trait;

use crate::domain::errors::DomainError;

/// Use case "gérer l'économie Coup de Coude".
///
/// Couvre les transferts inter-joueurs, le vol, le casino et les compteurs
/// quotidiens associés. Les opérations purement économiques d'un seul joueur
/// (`record_coins_earned/lost`, `adjust_coins`) sont gérées par
/// `ManageCoudePlayersUseCase`.
#[async_trait]
pub trait ManageCoudeEconomyUseCase: Send + Sync {
    async fn transfer(
        &self,
        guild_id: &str,
        from_id: &str,
        to_id: &str,
        amount: i64,
    ) -> Result<(), DomainError>;

    /// Retourne le montant réellement volé (erreur si la victime n'a rien).
    async fn steal(
        &self,
        guild_id: &str,
        thief_id: &str,
        victim_id: &str,
        amount: i64,
    ) -> Result<i64, DomainError>;

    async fn record_casino_win(
        &self,
        guild_id: &str,
        user_id: &str,
        gain: i64,
    ) -> Result<(), DomainError>;

    async fn record_casino_loss(
        &self,
        guild_id: &str,
        user_id: &str,
        lost: i64,
    ) -> Result<(), DomainError>;

    async fn record_casino_faillite(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<i64, DomainError>;

    async fn count_casino_today(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<i64, DomainError>;

    async fn sum_casino_gains_today(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<i64, DomainError>;

    async fn count_steal_today(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<i64, DomainError>;
}
