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

    // ── Vol ──
    //
    // NOTE migration wallet unifie (`/voler`) : `ManageCoudeEconomyService::steal`
    // delegue desormais a `ManageWalletUseCase::transfer` et appelle
    // `record_steal_stats` pour les compteurs. La methode `steal`
    // ci-dessous est conservee pour les call sites legacy non encore
    // migres (ex: `ManageCoudeSocialService::run_daily_chaos`) qui
    // enchainent wallet SQL + stats dans une meme tx.

    /// Vol atomique : debite la victime du minimum entre `amount` et
    /// son solde, credite le voleur, incremente `total_lost` /
    /// `total_stolen` / `total_earned`. Retourne le montant reellement
    /// vole (0 si la victime n'a rien). Utilise uniquement par le
    /// daily chaos (pas par `/voler`).
    async fn steal(
        &self,
        guild_id: &str,
        thief_id: &str,
        victim_id: &str,
        amount: i64,
    ) -> Result<i64, DomainError>;

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
