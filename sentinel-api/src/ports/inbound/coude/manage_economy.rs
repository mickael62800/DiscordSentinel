use async_trait::async_trait;

use sentinel_core::domain::entities::coude::taunt::TauntEvent;
use sentinel_core::domain::errors::DomainError;

/// Resultat d'un vol reussi (migration wallet unifie).
///
/// `stolen` est le montant reellement debite/credite (clamp au solde
/// de la victime), `taunt_events` contient les taunts declenches par
/// la mutation wallet : faillite cote victime si son solde passe de
/// >0 a 0, jackpot cote voleur si le montant depasse le seuil
/// configure. Le taunt "victim streak" (`on_player_stolen_from`)
/// reste gere separement cote bot via `track_steal_victim` car il
/// depend du nombre de vols subis et non du montant.
#[derive(Debug, Clone)]
pub struct StealOutcome {
    pub stolen: i64,
    pub taunt_events: Vec<TauntEvent>,
}

/// Use case "gérer l'économie Coup de Coude".
///
/// Couvre les transferts inter-joueurs, le vol, le casino et les compteurs
/// quotidiens associés. Les opérations purement économiques d'un seul joueur
/// (`record_coins_earned/lost`, `adjust_coins`) sont gérées par
/// `ManageCoudePlayersUseCase`.
#[async_trait]
pub trait ManageCoudeEconomyUseCase: Send + Sync {
    /// Transfert atomique entre deux joueurs. Depuis la migration wallet
    /// unifie, retourne les `TauntEvent` declenches : faillite cote emetteur
    /// (solde passe de >0 a 0), jackpot cote recepteur (amount >= seuil
    /// config), don genereux cote emetteur (amount >= seuil config). Le bot
    /// dispatche la liste via `taunts_dispatch::dispatch_all`.
    async fn transfer(
        &self,
        guild_id: &str,
        from_id: &str,
        to_id: &str,
        amount: i64,
    ) -> Result<Vec<TauntEvent>, DomainError>;

    /// Vol reussi : debite la victime et credite le voleur de min(amount,
    /// solde victime). Depuis la migration wallet unifie, la mutation
    /// wallet est deleguee a `ManageWalletUseCase::transfer` (faillite
    /// cote victime + jackpot cote voleur auto-detectes) ; les compteurs
    /// stats (`total_stolen`, `total_lost`) restent geres par le repo
    /// economy. Retourne `StealOutcome { stolen, taunt_events }`.
    /// Erreur si la victime n'a rien.
    async fn steal(
        &self,
        guild_id: &str,
        thief_id: &str,
        victim_id: &str,
        amount: i64,
    ) -> Result<StealOutcome, DomainError>;

    /// Vol rate : applique la penalite configuree au voleur. Debite au
    /// plus `amount` coins (clamp au solde reel comme le legacy
    /// `record_coins_lost`). Delegue a `ManageWalletUseCase::debit` pour
    /// la mutation wallet (detection faillite cote voleur incluse). Les
    /// compteurs stats (`total_lost`) sont aussi mis a jour. Retourne
    /// `(lost, taunt_events)`.
    async fn steal_fail_penalty(
        &self,
        guild_id: &str,
        thief_id: &str,
        amount: i64,
    ) -> Result<(i64, Vec<TauntEvent>), DomainError>;

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
