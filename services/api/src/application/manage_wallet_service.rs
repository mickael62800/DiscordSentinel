//! Implementation du use case wallet unifie.
//!
//! # Architecture
//!
//! Ce service expose DEUX modes :
//!
//! 1. **Standalone** (`credit`, `debit`, `transfer`) — le service ouvre sa
//!    propre tx, fait le UPDATE + INSERT wallet_transactions, commit, puis
//!    detecte les taunts (faillite/jackpot) et les retourne dans la
//!    `WalletMutation`. Point d'entree simple pour les call sites qui n'ont
//!    pas de tx composite.
//!
//! 2. **Tx en cours** (`credit_tx`, `debit_tx`, `post_commit_taunts`) — le
//!    call site passe sa propre tx sqlx. Le service fait l'UPDATE + INSERT
//!    sur cette tx, sans commit. Le champ `maybe_bankruptcy` /
//!    `maybe_jackpot_amount` de la `TxWalletMutation` permet au call site
//!    de rappeler `post_commit_taunts` APRES son `tx.commit()` pour
//!    recuperer la liste de `TauntEvent` a propager.
//!
//! Ce double mode permet de migrer progressivement les ~25 call sites de
//! mutations `user_wallets` sans casser leurs tx composites (mise pari +
//! insert bet_row, record_casino_win + coude_players + casino_log, etc.).
//!
//! # Detection de taunts
//!
//! - **Faillite** : debit dont le passage strict `previous > 0 → new == 0`.
//! - **Jackpot** : credit avec `amount >= threshold_from_taunts_config`
//!   (default 10_000, configurable par `jackpot_threshold`).

use std::sync::Arc;

use async_trait::async_trait;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::domain::entities::TauntEvent;
use crate::domain::errors::DomainError;
use crate::ports::inbound::manage_coude_taunts::ManageCoudeTauntsUseCase;
use crate::ports::inbound::manage_wallet::{
    ManageWalletUseCase, TxWalletMutation, WalletMutation,
};
use crate::ports::outbound::WalletRepository;

pub struct ManageWalletService {
    repo: Arc<dyn WalletRepository>,
    taunts_uc: Arc<dyn ManageCoudeTauntsUseCase>,
}

impl ManageWalletService {
    pub fn new(
        repo: Arc<dyn WalletRepository>,
        taunts_uc: Arc<dyn ManageCoudeTauntsUseCase>,
    ) -> Self {
        Self { repo, taunts_uc }
    }

    async fn read_balance(&self, guild_id: &str, user_id: &str) -> i64 {
        self.repo
            .get(guild_id, user_id)
            .await
            .ok()
            .flatten()
            .map(|w| w.coins)
            .unwrap_or(0)
    }
}

/// Ecrit une ligne dans `wallet_transactions` au sein de la tx fournie.
async fn insert_wallet_tx(
    tx: &mut Transaction<'_, Postgres>,
    guild_id: &str,
    user_id: &str,
    signed_amount: i64,
    balance_after: i64,
    source: &str,
    description: &str,
) -> Result<(), DomainError> {
    sqlx::query(
        "INSERT INTO wallet_transactions (id, guild_id, user_id, amount, balance_after, source, description, created_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, NOW())",
    )
    .bind(Uuid::new_v4())
    .bind(guild_id)
    .bind(user_id)
    .bind(signed_amount)
    .bind(balance_after)
    .bind(source)
    .bind(description)
    .execute(&mut **tx)
    .await
    .map_err(|e| DomainError::Internal(format!("insert wallet_transactions: {e}")))?;
    Ok(())
}

#[async_trait]
impl ManageWalletUseCase for ManageWalletService {
    async fn credit(
        &self,
        guild_id: &str,
        user_id: &str,
        amount: i64,
        source: &str,
        description: &str,
    ) -> Result<WalletMutation, DomainError> {
        if amount <= 0 {
            return Err(DomainError::ValidationError(
                "Montant credit doit etre positif".into(),
            ));
        }

        let previous = self.read_balance(guild_id, user_id).await;
        let wallet = self
            .repo
            .credit(guild_id, user_id, amount, source, description)
            .await?;

        let mut triggered_taunts = Vec::new();

        // Jackpot detection : le palier est porte par la taunts config (default
        // 10_000 cote service taunts).
        if let Ok(Some(evt)) = self
            .taunts_uc
            .on_jackpot(guild_id, user_id, amount)
            .await
        {
            triggered_taunts.push(evt);
        }

        Ok(WalletMutation {
            new_balance: wallet.coins,
            previous_balance: previous,
            triggered_taunts,
        })
    }

    async fn debit(
        &self,
        guild_id: &str,
        user_id: &str,
        amount: i64,
        source: &str,
        description: &str,
    ) -> Result<WalletMutation, DomainError> {
        if amount <= 0 {
            return Err(DomainError::ValidationError(
                "Montant debit doit etre positif".into(),
            ));
        }

        let previous = self.read_balance(guild_id, user_id).await;
        let wallet = self
            .repo
            .debit(guild_id, user_id, amount, source, description)
            .await?;

        let mut triggered_taunts = Vec::new();

        // Faillite : passage strict de >0 a 0.
        if previous > 0 && wallet.coins == 0 {
            if let Ok(Some(evt)) = self.taunts_uc.on_bankruptcy(guild_id, user_id).await {
                triggered_taunts.push(evt);
            }
        }

        Ok(WalletMutation {
            new_balance: wallet.coins,
            previous_balance: previous,
            triggered_taunts,
        })
    }

    async fn transfer(
        &self,
        guild_id: &str,
        from_user: &str,
        to_user: &str,
        amount: i64,
        source: &str,
        description: &str,
    ) -> Result<Vec<TauntEvent>, DomainError> {
        if amount <= 0 {
            return Err(DomainError::ValidationError(
                "Montant transfer doit etre positif".into(),
            ));
        }
        if from_user == to_user {
            return Err(DomainError::ValidationError(
                "Impossible de se transferer a soi-meme".into(),
            ));
        }

        let sender_before = self.read_balance(guild_id, from_user).await;

        self.repo
            .transfer(guild_id, from_user, to_user, amount, source, description)
            .await?;

        let sender_after = self.read_balance(guild_id, from_user).await;

        let mut triggered = Vec::new();

        // Faillite cote emetteur.
        if sender_before > 0 && sender_after == 0 {
            if let Ok(Some(evt)) = self.taunts_uc.on_bankruptcy(guild_id, from_user).await {
                triggered.push(evt);
            }
        }
        // Jackpot cote recepteur.
        if let Ok(Some(evt)) = self.taunts_uc.on_jackpot(guild_id, to_user, amount).await {
            triggered.push(evt);
        }

        Ok(triggered)
    }

    async fn get_balance(&self, guild_id: &str, user_id: &str) -> Result<i64, DomainError> {
        Ok(self.read_balance(guild_id, user_id).await)
    }

    // ── Mode "tx en cours" ────────────────────────────────────────────────

    async fn credit_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        guild_id: &str,
        user_id: &str,
        amount: i64,
        source: &str,
        description: &str,
    ) -> Result<TxWalletMutation, DomainError> {
        if amount <= 0 {
            return Err(DomainError::ValidationError(
                "Montant credit doit etre positif".into(),
            ));
        }

        // Lock le wallet et lit le solde actuel dans la tx. On fait un
        // SELECT FOR UPDATE prealable pour exposer `previous_balance`
        // fiable (utile pour detection jackpot meme si on pourrait s'en
        // passer sur un simple credit).
        let previous: Option<i64> = sqlx::query_scalar(
            "SELECT coins FROM user_wallets WHERE guild_id = $1 AND user_id = $2 FOR UPDATE",
        )
        .bind(guild_id)
        .bind(user_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(|e| DomainError::Internal(format!("credit_tx select: {e}")))?;

        let previous = previous
            .ok_or_else(|| DomainError::NotFound("Portefeuille introuvable".into()))?;

        let balance_after: i64 = sqlx::query_scalar(
            "UPDATE user_wallets SET coins = coins + $1, total_earned = total_earned + $1, updated_at = NOW() \
             WHERE guild_id = $2 AND user_id = $3 RETURNING coins",
        )
        .bind(amount)
        .bind(guild_id)
        .bind(user_id)
        .fetch_one(&mut **tx)
        .await
        .map_err(|e| DomainError::Internal(format!("credit_tx update: {e}")))?;

        insert_wallet_tx(tx, guild_id, user_id, amount, balance_after, source, description).await?;

        Ok(TxWalletMutation {
            new_balance: balance_after,
            previous_balance: previous,
            maybe_bankruptcy: false,
            maybe_jackpot_amount: Some(amount),
        })
    }

    async fn debit_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        guild_id: &str,
        user_id: &str,
        amount: i64,
        source: &str,
        description: &str,
    ) -> Result<TxWalletMutation, DomainError> {
        if amount <= 0 {
            return Err(DomainError::ValidationError(
                "Montant debit doit etre positif".into(),
            ));
        }

        // Lock + verifie solde suffisant.
        let previous: Option<i64> = sqlx::query_scalar(
            "SELECT coins FROM user_wallets WHERE guild_id = $1 AND user_id = $2 FOR UPDATE",
        )
        .bind(guild_id)
        .bind(user_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(|e| DomainError::Internal(format!("debit_tx select: {e}")))?;

        let previous = previous
            .ok_or_else(|| DomainError::NotFound("Portefeuille introuvable".into()))?;

        if previous < amount {
            return Err(DomainError::ValidationError(format!(
                "Solde insuffisant : tu as {} coins, il en faut {} (manque {}).",
                previous,
                amount,
                amount - previous
            )));
        }

        let balance_after: i64 = sqlx::query_scalar(
            "UPDATE user_wallets SET coins = coins - $1, total_spent = total_spent + $1, updated_at = NOW() \
             WHERE guild_id = $2 AND user_id = $3 RETURNING coins",
        )
        .bind(amount)
        .bind(guild_id)
        .bind(user_id)
        .fetch_one(&mut **tx)
        .await
        .map_err(|e| DomainError::Internal(format!("debit_tx update: {e}")))?;

        insert_wallet_tx(tx, guild_id, user_id, -amount, balance_after, source, description).await?;

        Ok(TxWalletMutation {
            new_balance: balance_after,
            previous_balance: previous,
            maybe_bankruptcy: previous > 0 && balance_after == 0,
            maybe_jackpot_amount: None,
        })
    }

    async fn post_commit_taunts(
        &self,
        guild_id: &str,
        user_id: &str,
        mutation: &TxWalletMutation,
    ) -> Vec<TauntEvent> {
        let mut out = Vec::new();
        if mutation.maybe_bankruptcy {
            if let Ok(Some(evt)) = self.taunts_uc.on_bankruptcy(guild_id, user_id).await {
                out.push(evt);
            }
        }
        if let Some(amount) = mutation.maybe_jackpot_amount {
            if let Ok(Some(evt)) = self.taunts_uc.on_jackpot(guild_id, user_id, amount).await {
                out.push(evt);
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::{StreakKind, TauntEvent};
    use crate::domain::errors::DomainError;
    use crate::domain::entities::{CoudeTauntsConfig, Wallet, WalletTransaction};
    use crate::ports::inbound::manage_coude_taunts::ManageCoudeTauntsUseCase;
    use async_trait::async_trait;
    use chrono::Utc;
    use std::sync::Mutex;
    use uuid::Uuid;

    // ── Mocks ──

    struct MockWalletRepo {
        balance: Mutex<i64>,
    }
    impl MockWalletRepo {
        fn new(initial: i64) -> Self {
            Self { balance: Mutex::new(initial) }
        }
        fn wallet(&self, coins: i64) -> Wallet {
            Wallet {
                id: Uuid::new_v4(),
                guild_id: "g".into(),
                user_id: "u".into(),
                username: "u".into(),
                coins,
                total_earned: 0,
                total_spent: 0,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            }
        }
    }
    #[async_trait]
    impl WalletRepository for MockWalletRepo {
        async fn get_or_create(&self, _g: &str, _u: &str, _n: &str, _s: i64) -> Result<Wallet, DomainError> {
            Ok(self.wallet(*self.balance.lock().unwrap()))
        }
        async fn get(&self, _g: &str, _u: &str) -> Result<Option<Wallet>, DomainError> {
            Ok(Some(self.wallet(*self.balance.lock().unwrap())))
        }
        async fn credit(&self, _g: &str, _u: &str, amount: i64, _s: &str, _d: &str) -> Result<Wallet, DomainError> {
            let mut b = self.balance.lock().unwrap();
            *b += amount;
            Ok(self.wallet(*b))
        }
        async fn debit(&self, _g: &str, _u: &str, amount: i64, _s: &str, _d: &str) -> Result<Wallet, DomainError> {
            let mut b = self.balance.lock().unwrap();
            if *b < amount {
                return Err(DomainError::ValidationError("insuffisant".into()));
            }
            *b -= amount;
            Ok(self.wallet(*b))
        }
        async fn transfer(&self, _g: &str, _f: &str, _t: &str, amount: i64, _s: &str, _d: &str) -> Result<(), DomainError> {
            let mut b = self.balance.lock().unwrap();
            if *b < amount {
                return Err(DomainError::ValidationError("insuffisant".into()));
            }
            *b -= amount;
            Ok(())
        }
        async fn leaderboard(&self, _g: &str, _l: i64) -> Result<Vec<Wallet>, DomainError> { Ok(vec![]) }
        async fn get_transactions(&self, _g: &str, _u: &str, _l: i64) -> Result<Vec<WalletTransaction>, DomainError> { Ok(vec![]) }
        async fn list_by_guild(&self, _g: &str) -> Result<Vec<Wallet>, DomainError> { Ok(vec![]) }
        async fn reset_wallet(&self, _g: &str, _u: &str, b: i64) -> Result<Wallet, DomainError> {
            *self.balance.lock().unwrap() = b;
            Ok(self.wallet(b))
        }
        async fn reset_all_wallets(&self, _g: &str, _b: i64) -> Result<u64, DomainError> { Ok(0) }
    }

    struct MockTaunts {
        bankruptcy_calls: Mutex<u32>,
        jackpot_calls: Mutex<u32>,
        last_jackpot_amount: Mutex<Option<i64>>,
    }
    impl MockTaunts {
        fn new() -> Self {
            Self {
                bankruptcy_calls: Mutex::new(0),
                jackpot_calls: Mutex::new(0),
                last_jackpot_amount: Mutex::new(None),
            }
        }
        fn fake_event(kind: StreakKind) -> TauntEvent {
            TauntEvent {
                channel_id: "c".into(),
                target_user_id: "u".into(),
                message: "taunt".into(),
                nickname_suffix: String::new(),
                streak_kind: kind.as_str(),
                streak_value: 1,
            }
        }
    }
    #[async_trait]
    impl ManageCoudeTauntsUseCase for MockTaunts {
        async fn on_player_won(&self, _g: &str, _u: &str) -> Result<Option<TauntEvent>, DomainError> { Ok(None) }
        async fn on_player_lost(&self, _g: &str, _u: &str) -> Result<Option<TauntEvent>, DomainError> { Ok(None) }
        async fn on_player_drew(&self, _g: &str, _u: &str) -> Result<(), DomainError> { Ok(()) }
        async fn on_player_stolen_from(&self, _g: &str, _u: &str) -> Result<Option<TauntEvent>, DomainError> { Ok(None) }
        async fn on_player_defended_steal(&self, _g: &str, _u: &str) -> Result<(), DomainError> { Ok(()) }
        async fn on_bj_natural(&self, _g: &str, _u: &str) -> Result<Option<TauntEvent>, DomainError> { Ok(None) }
        async fn on_bj_hand_won(&self, _g: &str, _u: &str) -> Result<Option<TauntEvent>, DomainError> { Ok(None) }
        async fn on_bj_hand_bust(&self, _g: &str, _u: &str) -> Result<Option<TauntEvent>, DomainError> { Ok(None) }
        async fn on_bankruptcy(&self, _g: &str, _u: &str) -> Result<Option<TauntEvent>, DomainError> {
            *self.bankruptcy_calls.lock().unwrap() += 1;
            Ok(Some(Self::fake_event(StreakKind::EcoBankruptcy)))
        }
        async fn on_jackpot(&self, _g: &str, _u: &str, amount: i64) -> Result<Option<TauntEvent>, DomainError> {
            *self.jackpot_calls.lock().unwrap() += 1;
            *self.last_jackpot_amount.lock().unwrap() = Some(amount);
            if amount >= 10_000 {
                Ok(Some(Self::fake_event(StreakKind::EcoJackpot)))
            } else {
                Ok(None)
            }
        }
        async fn on_generous_donor(&self, _g: &str, _u: &str, _a: i64) -> Result<Option<TauntEvent>, DomainError> { Ok(None) }
        async fn get_config(&self, _g: &str) -> Result<CoudeTauntsConfig, DomainError> {
            Ok(CoudeTauntsConfig { guild_id: "g".into(), channel_id: None, enabled: false })
        }
        async fn set_channel(&self, _g: &str, _c: Option<&str>) -> Result<(), DomainError> { Ok(()) }
        async fn set_enabled(&self, _g: &str, _e: bool) -> Result<(), DomainError> { Ok(()) }
        async fn set_opt_out(&self, _g: &str, _u: &str, _o: bool) -> Result<(), DomainError> { Ok(()) }
        async fn is_opted_out(&self, _g: &str, _u: &str) -> Result<bool, DomainError> { Ok(false) }
        async fn list_opt_outs(&self, _g: &str) -> Result<Vec<String>, DomainError> { Ok(vec![]) }
    }

    // ── Tests ──

    #[tokio::test]
    async fn credit_triggers_jackpot_when_amount_above_threshold() {
        let repo = Arc::new(MockWalletRepo::new(100));
        let taunts = Arc::new(MockTaunts::new());
        let svc = ManageWalletService::new(repo.clone(), taunts.clone());

        let m = svc.credit("g", "u", 15_000, "test", "d").await.unwrap();
        assert_eq!(m.new_balance, 15_100);
        assert_eq!(m.previous_balance, 100);
        assert_eq!(m.triggered_taunts.len(), 1);
        assert_eq!(*taunts.jackpot_calls.lock().unwrap(), 1);
    }

    #[tokio::test]
    async fn debit_full_balance_triggers_bankruptcy_taunt() {
        let repo = Arc::new(MockWalletRepo::new(500));
        let taunts = Arc::new(MockTaunts::new());
        let svc = ManageWalletService::new(repo.clone(), taunts.clone());

        let m = svc.debit("g", "u", 500, "test", "d").await.unwrap();
        assert_eq!(m.new_balance, 0);
        assert_eq!(m.previous_balance, 500);
        assert_eq!(m.triggered_taunts.len(), 1);
        assert_eq!(*taunts.bankruptcy_calls.lock().unwrap(), 1);
    }

    #[tokio::test]
    async fn debit_partial_does_not_trigger_bankruptcy() {
        let repo = Arc::new(MockWalletRepo::new(500));
        let taunts = Arc::new(MockTaunts::new());
        let svc = ManageWalletService::new(repo.clone(), taunts.clone());

        let m = svc.debit("g", "u", 100, "test", "d").await.unwrap();
        assert_eq!(m.triggered_taunts.len(), 0);
        assert_eq!(*taunts.bankruptcy_calls.lock().unwrap(), 0);
    }

    #[tokio::test]
    async fn credit_rejects_non_positive_amount() {
        let repo = Arc::new(MockWalletRepo::new(100));
        let taunts = Arc::new(MockTaunts::new());
        let svc = ManageWalletService::new(repo, taunts);

        assert!(svc.credit("g", "u", 0, "t", "d").await.is_err());
        assert!(svc.credit("g", "u", -1, "t", "d").await.is_err());
    }
}
