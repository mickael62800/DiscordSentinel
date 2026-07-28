//! Implementation du use case Roue du Destin.
//!
//! Flow (repris de l'ancien `manage_wheel_service` Sentinel, simplifie) :
//!   1. Claim atomique du tirage du jour (`try_claim_today`) — seule la
//!      premiere requete concurrente du jour obtient `true`.
//!   2. Spin RNG (entropie OS en prod).
//!   3. Wallet : credit si payout > 0, debit CLAMPE AU SOLDE si payout < 0
//!      (un joueur ne passe jamais en negatif), rien si payout = 0.
//!   4. Journalisation du spin.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use rand::SeedableRng;
use uuid::Uuid;

use crate::application::wallet_service::get_or_create_wallet;
use crate::domain::entities::wallet::Wallet;
use crate::domain::entities::wallet::WalletMutation;
use crate::domain::entities::wheel::is_memorable_case;
use crate::domain::entities::wheel::spin_with_rng;
use crate::domain::entities::wheel::WheelSpin;
use crate::domain::errors::DomainError;
use crate::ports::inbound::get_wallet::GetWalletUseCase;
use crate::ports::inbound::play_wheel::PlayWheelCommand;
use crate::ports::inbound::play_wheel::PlayWheelResult;
use crate::ports::inbound::play_wheel::PlayWheelUseCase;
use crate::ports::outbound::wallet_repository::WalletRepository;
use crate::ports::outbound::wheel_repository::WheelRepository;

pub struct PlayWheelService {
    wheel_repo: Arc<dyn WheelRepository>,
    wallet_repo: Arc<dyn WalletRepository>,
}

impl PlayWheelService {
    pub fn new(wheel_repo: Arc<dyn WheelRepository>, wallet_repo: Arc<dyn WalletRepository>) -> Self {
        Self {
            wheel_repo,
            wallet_repo,
        }
    }
}

#[async_trait]
impl PlayWheelUseCase for PlayWheelService {
    async fn spin(&self, cmd: PlayWheelCommand) -> Result<PlayWheelResult, DomainError> {
        // 1. Claim quotidien atomique.
        let claimed = self
            .wheel_repo
            .try_claim_today(&cmd.guild_id, &cmd.user_id)
            .await?;
        if !claimed {
            return Err(DomainError::Validation(
                "Tu as deja tire la Roue du Destin aujourd'hui.".into(),
            ));
        }

        // 2. Spin RNG (entropie OS).
        let mut rng = rand::rngs::StdRng::from_entropy();
        let outcome = spin_with_rng(&mut rng);
        let payout = outcome.case.payout;

        // 3. Wallet : regles pures de credit/debit (creation via le socle
        // partage : solde de depart credite pour un nouveau joueur).
        let mut wallet = get_or_create_wallet(
            self.wallet_repo.as_ref(),
            &cmd.guild_id,
            &cmd.user_id,
            &cmd.username,
        )
        .await?;
        wallet.username = cmd.username.clone();
        if payout > 0 {
            wallet.credit(payout)?;
            let mutation = WalletMutation {
                amount: payout,
                balance_after: wallet.coins,
                source: "wheel_payout".into(),
                description: format!("Roue du Destin : {}", outcome.case.label),
                reason: None,
            };
            self.wallet_repo
                .save_with_transaction(&wallet, &mutation)
                .await?;
        } else if payout < 0 {
            let actual = wallet.debit_clamped(-payout)?;
            if actual > 0 {
                let mutation = WalletMutation {
                    amount: -actual,
                    balance_after: wallet.coins,
                    source: "wheel_loss".into(),
                    description: format!("Roue du Destin : {}", outcome.case.label),
                    reason: None,
                };
                self.wallet_repo
                    .save_with_transaction(&wallet, &mutation)
                    .await?;
            }
        }

        // 4. Log du spin.
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
        self.wheel_repo.log_spin(&spin).await?;

        Ok(PlayWheelResult {
            is_memorable: is_memorable_case(&spin.case_key),
            balance_after: wallet.coins,
            spin,
        })
    }
}

#[async_trait]
impl GetWalletUseCase for PlayWheelService {
    async fn get(&self, guild_id: &str, user_id: &str) -> Result<Wallet, DomainError> {
        get_or_create_wallet(self.wallet_repo.as_ref(), guild_id, user_id, "").await
    }
}

#[cfg(test)]
#[path = "tests/play_wheel.rs"]
mod tests;
