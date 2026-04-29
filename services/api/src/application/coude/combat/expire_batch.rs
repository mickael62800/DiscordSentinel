//! Orchestration de l'expiration batch des combats pending Coup de Coude.
//! Phase 4 refacto. Remplace la logique inline de
//! `coude-worker/src/jobs/expire_combats.rs`.
//!
//! Respect strict de l'hexagonal : tout passe par les ports outbound et
//! les use cases satellites (bets). Aucun SQL direct ici.

#[cfg(test)]
#[path = "tests/expire_batch.rs"]
mod tests;

use std::sync::Arc;

use async_trait::async_trait;
use tracing::warn;

use crate::domain::entities::coude::expire::cowardice_penalty;
use crate::domain::entities::coude::cowardice_relief::should_count_as_cowardice;
use crate::domain::entities::coude::cashbox::CashboxSource;
use crate::domain::errors::DomainError;
use crate::ports::inbound::coude::expire_combats_batch::ExpireCombatsBatchUseCase;
use crate::ports::inbound::coude::expire_combats_batch::ExpiredCombatOutput;
use crate::ports::inbound::coude::manage_bets::ManageCoudeBetsUseCase;
use crate::ports::outbound::coude::cashbox_repository::CashboxRepository;
use crate::ports::outbound::coude::combat_repository::CombatRepository;
use crate::ports::outbound::coude::player_repository::PlayerRepository;
use crate::ports::outbound::casino::wallet_repository::WalletRepository;
/// 24h par defaut, override par guild via bot_guild_config.
const DEFAULT_EXPIRY_HOURS: i64 = 24;

pub struct ExpireCombatsBatchService {
    combat_repo: Arc<dyn CombatRepository>,
    player_repo: Arc<dyn PlayerRepository>,
    wallet_repo: Arc<dyn WalletRepository>,
    cashbox_repo: Arc<dyn CashboxRepository>,
    bets_uc: Arc<dyn ManageCoudeBetsUseCase>,
}

impl ExpireCombatsBatchService {
    pub fn new(
        combat_repo: Arc<dyn CombatRepository>,
        player_repo: Arc<dyn PlayerRepository>,
        wallet_repo: Arc<dyn WalletRepository>,
        cashbox_repo: Arc<dyn CashboxRepository>,
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
            // Penalite defenseur : domain rule dans coude_expire.rs
            let penalty = cowardice_penalty(combat.mise);
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

            // Migration wallet finale : `record_coins_lost` est desormais
            // stats-only (increment `total_lost`). La mutation wallet +
            // log `wallet_transactions` est deja faite par `wallet_repo.debit`
            // ci-dessus. Corrige le double-debit historique.
            if debit_ok {
                if let Err(e) = self
                    .player_repo
                    .record_coins_lost(&combat.guild_id, &combat.defender_id, penalty)
                    .await
                {
                    warn!(error = %e, "Echec record_coins_lost (stats) defenseur expire");
                }
            }

            // Sprint 1 (4.2) — relief : si le defenseur est <= 20% HP, on
            // n incremente PAS cowardice. Refus legitime quand on est mourant.
            let should_count = match self
                .player_repo
                .get(&combat.guild_id, &combat.defender_id)
                .await
            {
                Ok(Some(player)) => should_count_as_cowardice(player.hp_current, player.hp_max),
                _ => true, // si erreur fetch, fallback sur comportement historique
            };

            if should_count {
                if let Err(e) = self
                    .player_repo
                    .increment_cowardice(&combat.guild_id, &combat.defender_id)
                    .await
                {
                    warn!(error = %e, "Echec increment_cowardice defenseur expire");
                }
            }

            // Rembourser les paris (refund_all).
            if let Err(e) = self.bets_uc.refund(combat.id).await {
                warn!(error = %e, combat_id = %combat.id, "Echec refund paris expire");
            }

            out.push(ExpiredCombatOutput {
                combat_id: combat.id.to_string(),
                guild_id: combat.guild_id.clone(),
                channel_id: combat.channel_id.clone().unwrap_or_default().into(),
                defender_id: combat.defender_id.clone(),
                defender_name: combat.defender_name.clone(),
                penalty,
            });
        }
        Ok(out)
    }
}
