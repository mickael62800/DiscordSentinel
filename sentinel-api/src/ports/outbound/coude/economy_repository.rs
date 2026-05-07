use async_trait::async_trait;

use sentinel_core::domain::errors::DomainError;

/// Repository pour les opérations économiques Coup de Coude qui traversent
/// plusieurs lignes joueur ou enregistrent des logs séparés (casino, steal).
///
/// Les mutations multi-lignes (transfer, steal) sont atomiques au niveau du
/// repo (transaction + `FOR UPDATE`). Les validations métier restent côté
/// service.
#[async_trait]
pub trait EconomyRepository: Send + Sync {
    // ── Transferts entre joueurs ──
    //
    // NOTE migration wallet unifie : `transfer` a ete retire de ce repo.
    // La logique SQL est desormais centralisee dans
    // `WalletRepository::transfer` + `ManageWalletUseCase::transfer`, qui
    // gere aussi la detection de faillite/jackpot via le service taunts.
    // `ManageCoudeEconomyService::transfer` delegue directement a ce use
    // case (voir son implementation).

    // ── Vol ──
    //
    // NOTE migration wallet unifie (`/voler` + daily chaos) : toutes les
    // mutations wallet du vol passent desormais par
    // `ManageWalletUseCase::transfer`. Le repo ne conserve que les
    // compteurs stats (`record_steal_stats` / `record_steal_fail_stats`).
    // L'ancienne methode atomique `steal` a ete supprimee (plus aucun
    // caller).

    /// Compteurs `coude_players` apres un vol reussi : `total_lost` cote
    /// victime, `total_stolen` + `total_earned` cote voleur. Pas de
    /// mutation wallet.
    async fn record_steal_stats(
        &self,
        guild_id: &str,
        thief_id: &str,
        victim_id: &str,
        amount: i64,
    ) -> Result<(), DomainError>;

    /// Compteur `coude_players.total_lost` cote voleur apres un vol
    /// rate (penalite). Pas de mutation wallet.
    async fn record_steal_fail_stats(
        &self,
        guild_id: &str,
        thief_id: &str,
        amount: i64,
    ) -> Result<(), DomainError>;

    /// Lecture rapide du solde d'un joueur pour clamp pre-mutation
    /// (vol, penalite). Retourne `NotFound` si le wallet n'existe pas.
    async fn get_coins(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<i64, DomainError>;

    // ── Casino ──
    //
    // NOTE migration wallet unifie (Migration #5) : les mutations
    // `user_wallets` sont desormais deleguees a `ManageWalletUseCase`
    // (credit / debit) par `ManageCoudeEconomyService`. Le repo ne garde
    // que les compteurs `coude_players.casino_{wins,losses}` /
    // `total_earned` / `total_lost` et le log `coude_casino_log`. La
    // detection faillite / jackpot est centralisee cote wallet service.

    /// Stats casino gain : incremente `casino_wins` + `total_earned`
    /// cote `coude_players` et insere une ligne positive dans
    /// `coude_casino_log`. Pas de mutation wallet.
    async fn record_casino_win_stats(
        &self,
        guild_id: &str,
        user_id: &str,
        gain: i64,
    ) -> Result<(), DomainError>;

    /// Stats casino perte : incremente `casino_losses` + `total_lost`
    /// cote `coude_players` et insere une ligne negative dans
    /// `coude_casino_log`. Pas de mutation wallet.
    async fn record_casino_loss_stats(
        &self,
        guild_id: &str,
        user_id: &str,
        lost: i64,
    ) -> Result<(), DomainError>;

    /// Stats casino faillite : incremente `casino_losses` + `total_lost`
    /// cote `coude_players` et insere une ligne negative dans
    /// `coude_casino_log`. Retourne le `total_lost` cumule apres
    /// operation. Pas de mutation wallet.
    async fn record_casino_faillite_stats(
        &self,
        guild_id: &str,
        user_id: &str,
        cleared: i64,
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
