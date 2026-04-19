use async_trait::async_trait;

use crate::domain::errors::DomainError;

/// Repository pour les opérations économiques Coup de Coude qui traversent
/// plusieurs lignes joueur ou enregistrent des logs séparés (casino, steal).
///
/// Les mutations multi-lignes (transfer, steal) sont atomiques au niveau du
/// repo (transaction + `FOR UPDATE`). Les validations métier restent côté
/// service.
#[async_trait]
pub trait CoudeEconomyRepository: Send + Sync {
    // ── Transferts entre joueurs ──
    //
    // NOTE migration wallet unifie : `transfer` a ete retire de ce repo.
    // La logique SQL est desormais centralisee dans
    // `WalletRepository::transfer` + `ManageWalletUseCase::transfer`, qui
    // gere aussi la detection de faillite/jackpot via le service taunts.
    // `ManageCoudeEconomyService::transfer` delegue directement a ce use
    // case (voir son implementation).

    /// Vol : débite la victime du minimum entre `amount` et son solde réel,
    /// crédite le voleur de la même somme. Retourne le montant réellement volé
    /// (peut être 0 si la victime n'a pas de coins).
    ///
    /// Met à jour `total_lost` / `total_stolen` / `total_earned` pour les
    /// compteurs historiques.
    async fn steal(
        &self,
        guild_id: &str,
        thief_id: &str,
        victim_id: &str,
        amount: i64,
    ) -> Result<i64, DomainError>;

    // ── Casino ──

    /// Enregistre un gain casino : incrémente `casino_wins`, crédite le joueur,
    /// loggue le gain dans `coude_casino_log` pour le tracking quotidien.
    async fn record_casino_win(
        &self,
        guild_id: &str,
        user_id: &str,
        gain: i64,
    ) -> Result<(), DomainError>;

    /// Enregistre une perte casino : incrémente `casino_losses`, débite le
    /// joueur (plancher 0), loggue la perte (montant négatif) dans
    /// `coude_casino_log`.
    async fn record_casino_loss(
        &self,
        guild_id: &str,
        user_id: &str,
        lost: i64,
    ) -> Result<(), DomainError>;

    /// Faillite casino : remet les coins à 0, incrémente `casino_losses`,
    /// loggue la faillite. Retourne le `total_lost` cumulé après opération.
    async fn record_casino_faillite(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<i64, DomainError>;

    /// Nombre d'actions casino dans les 24h (via `coude_cooldowns`).
    async fn count_casino_today(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<i64, DomainError>;

    /// Somme des gains casino positifs dans les 24h (via `coude_casino_log`).
    async fn sum_casino_gains_today(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<i64, DomainError>;

    /// Nombre de vols effectués dans les 24h (via `coude_cooldowns`).
    async fn count_steal_today(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<i64, DomainError>;
}
