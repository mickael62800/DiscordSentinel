//! Orchestration de l'expiration batch des combats pending Coup de Coude.
//! Phase 4 refacto. Remplace la logique inline de
//! `coude-worker/src/jobs/expire_combats.rs`.
//!
//! Respect strict de l'hexagonal : tout passe par les ports outbound et
//! les use cases satellites (bets). Aucun SQL direct ici.

use std::sync::Arc;

use async_trait::async_trait;
use tracing::warn;

use crate::domain::entities::CashboxSource;
use crate::domain::errors::DomainError;
use crate::ports::inbound::expire_combats_batch::{
    ExpireCombatsBatchUseCase, ExpiredCombatOutput,
};
use crate::ports::inbound::ManageCoudeBetsUseCase;
use crate::ports::outbound::{
    CoudeCashboxRepository, CoudeCombatRepository, CoudePlayerRepository, WalletRepository,
};

/// 24h par defaut, override par guild via bot_guild_config.
const DEFAULT_EXPIRY_HOURS: i64 = 24;

pub struct ExpireCombatsBatchService {
    combat_repo: Arc<dyn CoudeCombatRepository>,
    player_repo: Arc<dyn CoudePlayerRepository>,
    wallet_repo: Arc<dyn WalletRepository>,
    cashbox_repo: Arc<dyn CoudeCashboxRepository>,
    bets_uc: Arc<dyn ManageCoudeBetsUseCase>,
}

impl ExpireCombatsBatchService {
    pub fn new(
        combat_repo: Arc<dyn CoudeCombatRepository>,
        player_repo: Arc<dyn CoudePlayerRepository>,
        wallet_repo: Arc<dyn WalletRepository>,
        cashbox_repo: Arc<dyn CoudeCashboxRepository>,
        bets_uc: Arc<dyn ManageCoudeBetsUseCase>,
    ) -> Self {
        Self { combat_repo, player_repo, wallet_repo, cashbox_repo, bets_uc }
    }
}

#[async_trait]
impl ExpireCombatsBatchUseCase for ExpireCombatsBatchService {
    async fn expire_batch(&self) -> Result<Vec<ExpiredCombatOutput>, DomainError> {
        // Claim atomique (UPDATE status='expired' + RETURNING).
        let expired = self
            .combat_repo
            .claim_expired_pending_combats(DEFAULT_EXPIRY_HOURS)
            .await?;

        let mut out = Vec::with_capacity(expired.len());
        for combat in &expired {
            // Penalite defenseur : 20% de la mise, minimum 1 coin.
            let penalty = ((combat.mise as f64 * 0.20).max(1.0)) as i64;
            let desc = format!("Penalite lachete combat {}", combat.id);

            let debit_ok = match self
                .wallet_repo
                .debit(
                    &combat.guild_id,
                    &combat.defender_id,
                    penalty,
                    "coude_combat_expire_penalty",
                    &desc,
                )
                .await
            {
                Ok(_) => true,
                Err(e) => {
                    warn!(error = %e, combat_id = %combat.id, "Echec debit penalite defenseur");
                    false
                }
            };

            if debit_ok {
                // Phase 9 : la penalite de lachete alimente la caisse communautaire.
                if let Err(e) = self
                    .cashbox_repo
                    .deposit(&combat.guild_id, penalty, CashboxSource::CowardicePenalty)
                    .await
                {
                    warn!(error = %e, combat_id = %combat.id, "Echec deposit cashbox penalite");
                }
            }

            if let Err(e) = self
                .player_repo
                .record_coins_lost(&combat.guild_id, &combat.defender_id, penalty)
                .await
            {
                warn!(error = %e, "Echec record_coins_lost defenseur expire");
            }

            if let Err(e) = self
                .player_repo
                .increment_cowardice(&combat.guild_id, &combat.defender_id)
                .await
            {
                warn!(error = %e, "Echec increment_cowardice defenseur expire");
            }

            // Rembourser les paris (refund_all).
            if let Err(e) = self.bets_uc.refund(combat.id).await {
                warn!(error = %e, combat_id = %combat.id, "Echec refund paris expire");
            }

            out.push(ExpiredCombatOutput {
                combat_id: combat.id.to_string(),
                guild_id: combat.guild_id.clone(),
                channel_id: combat.channel_id.clone().unwrap_or_default(),
                defender_id: combat.defender_id.clone(),
                defender_name: combat.defender_name.clone(),
                penalty,
            });
        }
        Ok(out)
    }
}
