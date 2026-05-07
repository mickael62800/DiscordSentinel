//! Implementation du use case Roue du Destin.
//!
//! Flow :
//!   1. Verifier que le user n a pas deja claim aujourd hui (sinon Validation)
//!   2. Tx atomique :
//!      - spin RNG (entropie OS, non-deterministe en prod)
//!      - debit/credit wallet selon payout (positif = credit, negatif = debit)
//!      - log spin
//!      - mark daily claimed
//!   3. Apres commit : post_commit_taunts (faillite/jackpot eco)

use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use rand::SeedableRng;
use uuid::Uuid;

use sentinel_core::domain::entities::casino::wheel::is_memorable_case;
use sentinel_core::domain::entities::casino::wheel::spin_with_rng_curses as wheel_spin_with_rng_curses;
use sentinel_core::domain::entities::coude::curse::CurseKind;
use sentinel_core::domain::entities::coude::taunt::TauntEvent;
use sentinel_core::domain::entities::casino::wheel::WheelSpin;
use sentinel_core::domain::entities::casino::wheel::WheelTopWinner;
use sentinel_core::domain::errors::DomainError;
use crate::ports::inbound::casino::manage_wallet::ManageWalletUseCase;
use crate::ports::inbound::casino::manage_wheel::ManageWheelUseCase;
use crate::ports::inbound::casino::manage_wheel::WheelSpinCommand;
use crate::ports::inbound::casino::manage_wheel::WheelSpinResult;
use crate::ports::outbound::coude::curses_repository::CursesRepository;
use crate::ports::outbound::casino::wheel_repository::WheelRepository;
pub struct ManageWheelService {
    repo: Arc<dyn WheelRepository>,
    wallet_uc: Arc<dyn ManageWalletUseCase>,
    curses_repo: Option<Arc<dyn CursesRepository>>,
    pg_pool: sqlx::PgPool,
}

impl ManageWheelService {
    pub fn new(
        repo: Arc<dyn WheelRepository>,
        wallet_uc: Arc<dyn ManageWalletUseCase>,
        pg_pool: sqlx::PgPool,
    ) -> Self {
        Self { repo, wallet_uc, curses_repo: None, pg_pool }
    }

    /// Branche le repo des maledictions pour activer "Heartbreak"
    /// (cf. COUPE_AMELIORATIONS 5.1) : le spinner maudit ne peut pas
    /// tomber sur la licorne. Optionnel pour preserver les call-sites
    /// de test et eviter une regression silencieuse.
    pub fn with_curses_repo(mut self, repo: Arc<dyn CursesRepository>) -> Self {
        self.curses_repo = Some(repo);
        self
    }
}

#[async_trait]
impl ManageWheelUseCase for ManageWheelService {
    async fn spin(&self, cmd: WheelSpinCommand) -> Result<WheelSpinResult, DomainError> {
        // 1. Verif daily.
        if self.repo.has_claimed_today(&cmd.guild_id, &cmd.user_id).await? {
            return Err(DomainError::ValidationError(
                "Tu as deja tire la Roue du Destin aujourd hui.".into(),
            ));
        }

        // 2. Detection malediction "Heartbreak" — bloque la licorne pour
        //    cette tirage. Echec silencieux : si le repo casse, on spin
        //    quand meme normalement (le user ne perdra rien de plus).
        let block_licorne = if let Some(curses_repo) = &self.curses_repo {
            match curses_repo
                .get_active_for_target(&cmd.guild_id, &cmd.user_id)
                .await
            {
                Ok(Some(c)) if c.kind == CurseKind::Heartbreak => true,
                _ => false,
            }
        } else {
            false
        };

        // 3. Spin RNG (entropie OS).
        let mut rng = rand::rngs::StdRng::from_entropy();
        let outcome = wheel_spin_with_rng_curses(&mut rng, block_licorne);
        let payout = outcome.case.payout;

        // 3. Tx atomique.
        let mut tx = self.pg_pool.begin().await
            .map_err(|e| DomainError::Internal(format!("begin tx wheel: {e}")))?;

        let mut taunt_mutations = Vec::new();

        // Wallet : credit ou debit selon le signe du payout.
        if payout > 0 {
            let m = self.wallet_uc
                .credit_tx(&mut tx, &cmd.guild_id, &cmd.user_id, payout, "wheel_payout",
                    &format!("Roue du Destin : {}", outcome.case.label))
                .await?;
            taunt_mutations.push((cmd.user_id.clone(), m));
        } else if payout < 0 {
            // Clamp : on ne peut pas debiter plus que le solde.
            let balance = self.wallet_uc.get_balance(&cmd.guild_id, &cmd.user_id).await?;
            let actual_debit = (-payout).min(balance);
            if actual_debit > 0 {
                let m = self.wallet_uc
                    .debit_tx(&mut tx, &cmd.guild_id, &cmd.user_id, actual_debit, "wheel_loss",
                        &format!("Roue du Destin : {}", outcome.case.label))
                    .await?;
                taunt_mutations.push((cmd.user_id.clone(), m));
            }
        }

        // Log spin.
        let spin = WheelSpin {
            id: Uuid::new_v4(),
            guild_id: cmd.guild_id.clone(),
            user_id: cmd.user_id.clone(),
            username: cmd.username.clone(),
            case_key: outcome.case.key.to_string(),
            case_label: outcome.case.label.to_string(),
            payout,
            created_at: Utc::now(),
        };
        self.repo.log_spin_in_tx(&mut tx, &spin).await?;

        // Mark daily.
        self.repo.mark_claimed_in_tx(&mut tx, &cmd.guild_id, &cmd.user_id).await?;

        tx.commit().await.map_err(|e| DomainError::Internal(format!("commit tx wheel: {e}")))?;

        // 4. Post-commit taunts.
        let mut triggered_taunts: Vec<TauntEvent> = Vec::new();
        for (uid, mutation) in &taunt_mutations {
            let evs = self.wallet_uc.post_commit_taunts(&cmd.guild_id, uid, mutation).await;
            triggered_taunts.extend(evs);
        }

        let balance_after = self.wallet_uc.get_balance(&cmd.guild_id, &cmd.user_id).await?;
        let is_memorable = is_memorable_case(outcome.case.key);

        Ok(WheelSpinResult {
            spin,
            case: outcome.case,
            balance_after,
            is_memorable,
            triggered_taunts,
        })
    }

    async fn recent_spins(&self, guild_id: &str, limit: i64) -> Result<Vec<WheelSpin>, DomainError> {
        self.repo.recent_spins(guild_id, limit).await
    }

    async fn top_winners(
        &self,
        guild_id: &str,
        days: i64,
        limit: i64,
    ) -> Result<Vec<WheelTopWinner>, DomainError> {
        self.repo.top_winners(guild_id, days, limit).await
    }
}

#[cfg(test)]
#[path = "tests/manage_wheel.rs"]
mod tests;
