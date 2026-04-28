//! Implementation du use case slot machine.
//!
//! Orchestration complete du flow spin :
//!   1. Lecture config (bot_guild_config) + parsing CSV + validation domain
//!   2. Verification cooldown / daily bonus deja claim
//!   3. Tx atomique : debit wallet, log spin, alimente/claim jackpot pool,
//!      credit payout si gagne, mark daily si applicable
//!   4. Apres commit : post_commit_taunts (faillite + jackpot eco)
//!
//! La fonction `spin` est NON-determine en prod (RNG OS) mais le domain
//! `spin_with_rng` est seedable pour les tests.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use rand::SeedableRng;
use uuid::Uuid;

use crate::domain::entities::casino::slot::compute_jackpot_contribution;
use crate::domain::entities::casino::slot::compute_payout;
use crate::domain::entities::casino::slot::evaluate_spin;
use crate::domain::entities::casino::slot::parse_csv_multipliers;
use crate::domain::entities::casino::slot::parse_csv_symbols;
use crate::domain::entities::casino::slot::parse_csv_weights;
use crate::domain::entities::casino::slot::spin_with_rng as slot_spin_with_rng;
use crate::domain::entities::casino::slot::validate_slot_config;
use crate::domain::entities::casino::slot::SlotConfig;
use crate::domain::entities::casino::slot::SlotSpin;
use crate::domain::entities::casino::slot::SlotTopWinner;
use crate::domain::entities::casino::slot::SpinOutcome;
use crate::domain::entities::coude::taunt::TauntEvent;
use crate::domain::errors::DomainError;
use crate::ports::inbound::casino::manage_slot::ManageSlotUseCase;
use crate::ports::inbound::casino::manage_slot::SpinCommand;
use crate::ports::inbound::casino::manage_slot::SpinResult;
use crate::ports::inbound::casino::manage_wallet::ManageWalletUseCase;
use crate::ports::outbound::system::bot_config_repository::BotConfigRepository;
use crate::ports::outbound::casino::slot_repository::SlotRepository;
const MODULE_BOT_NAME: &str = "slot-bot";

pub struct ManageSlotService {
    repo: Arc<dyn SlotRepository>,
    bot_config_repo: Arc<dyn BotConfigRepository>,
    wallet_uc: Arc<dyn ManageWalletUseCase>,
    pg_pool: sqlx::PgPool,
}

impl ManageSlotService {
    pub fn new(
        repo: Arc<dyn SlotRepository>,
        bot_config_repo: Arc<dyn BotConfigRepository>,
        wallet_uc: Arc<dyn ManageWalletUseCase>,
        pg_pool: sqlx::PgPool,
    ) -> Self {
        Self { repo, bot_config_repo, wallet_uc, pg_pool }
    }

    /// Charge la config du bot 'slot-bot' pour la guild et la decode en
    /// `SlotConfig` typee. Fallback sur les defauts pour chaque cle absente.
    async fn load_config(&self, guild_id: &str) -> Result<SlotConfig, DomainError> {
        let entries = self
            .bot_config_repo
            .get_config(guild_id, MODULE_BOT_NAME)
            .await
            .unwrap_or_default();

        let mut cfg = SlotConfig::default();
        let mut symbols_raw: Option<String> = None;
        let mut weights_raw: Option<String> = None;
        let mut multipliers_raw: Option<String> = None;

        for entry in &entries {
            match entry.config_key.as_str() {
                "symbols" => symbols_raw = Some(entry.config_value.clone()),
                "weights" => weights_raw = Some(entry.config_value.clone()),
                "payout_3x_multipliers" => multipliers_raw = Some(entry.config_value.clone()),
                "payout_2x_enabled" => cfg.payout_2x_enabled = parse_bool(&entry.config_value, true),
                "jackpot_pool_share_pct" => {
                    if let Ok(v) = entry.config_value.parse() { cfg.jackpot_pool_share_pct = v; }
                }
                "jackpot_starting_pool" => {
                    if let Ok(v) = entry.config_value.parse() { cfg.jackpot_starting_pool = v; }
                }
                "min_bet" => if let Ok(v) = entry.config_value.parse() { cfg.min_bet = v; },
                "max_bet" => if let Ok(v) = entry.config_value.parse() { cfg.max_bet = v; },
                "default_bet" => if let Ok(v) = entry.config_value.parse() { cfg.default_bet = v; },
                "cooldown_secs" => if let Ok(v) = entry.config_value.parse() { cfg.cooldown_secs = v; },
                "daily_bonus_enabled" => cfg.daily_bonus_enabled = parse_bool(&entry.config_value, true),
                "daily_bonus_mise" => if let Ok(v) = entry.config_value.parse() { cfg.daily_bonus_mise = v; },
                _ => {}
            }
        }

        if let Some(s) = symbols_raw { cfg.symbols = parse_csv_symbols(&s); }
        if let Some(w) = weights_raw { cfg.weights = parse_csv_weights(&w); }
        if let Some(m) = multipliers_raw { cfg.multipliers_3x = parse_csv_multipliers(&m); }

        validate_slot_config(&cfg).map_err(|e| {
            DomainError::ValidationError(format!("Config slot-bot invalide : {}", e.as_str()))
        })?;

        Ok(cfg)
    }

    /// Flow interne commun a `spin` (payant) et `claim_daily_bonus` (gratuit).
    async fn run_spin(&self, cmd: &SpinCommand, cfg: &SlotConfig) -> Result<SpinResult, DomainError> {
        let mise = if cmd.is_daily { cfg.daily_bonus_mise } else { cmd.mise };

        // RNG OS-driven (non-deterministe).
        let mut rng = rand::rngs::StdRng::from_entropy();
        let symbol_indices = slot_spin_with_rng(&mut rng, cfg);
        let outcome = evaluate_spin(&symbol_indices, cfg);
        let symbol_strings: Vec<String> = symbol_indices
            .iter()
            .map(|i| cfg.symbols.get(*i).cloned().unwrap_or_default())
            .collect();

        // Tx atomique.
        let mut tx = self.pg_pool.begin().await
            .map_err(|e| DomainError::Internal(format!("begin tx slot: {e}")))?;

        let mut taunt_mutations = Vec::new();

        // 1. Debit la mise (sauf daily bonus) en utilisant wallet_uc dans la tx.
        if !cmd.is_daily && mise > 0 {
            let dm = self.wallet_uc
                .debit_tx(&mut tx, &cmd.guild_id, &cmd.user_id, mise, "slot_bet",
                    &format!("Mise slot-machine"))
                .await?;
            taunt_mutations.push((cmd.user_id.clone(), dm));
        }

        // 2. Alimente le pool jackpot (% mise, payant ou daily — daily contribue
        //    via daily_bonus_mise).
        let jackpot_contribution = compute_jackpot_contribution(mise, cfg.jackpot_pool_share_pct);
        let pool_after_contribution = if jackpot_contribution > 0 {
            self.repo.add_to_jackpot_pool_in_tx(
                &mut tx, &cmd.guild_id, jackpot_contribution, cfg.jackpot_starting_pool,
            ).await?
        } else {
            self.repo.get_jackpot_pool(&cmd.guild_id).await?
                .map(|p| p.current_pool)
                .unwrap_or(cfg.jackpot_starting_pool)
        };

        // 3. Calcule le payout final. En cas de Jackpot, le pool est ajoute.
        let payout = compute_payout(mise, &outcome, pool_after_contribution);
        let multiplier = match &outcome {
            SpinOutcome::ThreeOfAKind { multiplier, .. } => *multiplier,
            SpinOutcome::Jackpot { multiplier } => *multiplier,
            SpinOutcome::RefundTwoOfAKind => 1.0,
            SpinOutcome::Loss => 0.0,
        };
        let is_jackpot = matches!(outcome, SpinOutcome::Jackpot { .. });

        // 4. Si Jackpot : claim et reset le pool.
        let mut jackpot_pool_after = pool_after_contribution;
        if is_jackpot {
            self.repo.claim_jackpot_pool_in_tx(
                &mut tx, &cmd.guild_id, &cmd.user_id,
                pool_after_contribution, cfg.jackpot_starting_pool,
            ).await?;
            jackpot_pool_after = cfg.jackpot_starting_pool;
        }

        // 5. Credit du payout (si > 0).
        if payout > 0 {
            let cm = self.wallet_uc
                .credit_tx(&mut tx, &cmd.guild_id, &cmd.user_id, payout, "slot_payout",
                    &format!("Gain slot-machine ({}x)", multiplier))
                .await?;
            taunt_mutations.push((cmd.user_id.clone(), cm));
        }

        // 6. Log du spin.
        let spin = SlotSpin {
            id: Uuid::new_v4(),
            guild_id: cmd.guild_id.clone(),
            user_id: cmd.user_id.clone(),
            username: cmd.username.clone(),
            mise,
            symbols: symbol_strings,
            payout,
            multiplier,
            is_jackpot,
            is_free: cmd.is_daily,
            created_at: Utc::now(),
        };
        self.repo.log_spin_in_tx(&mut tx, &spin).await?;

        // 7. Mark daily claimed si applicable.
        if cmd.is_daily {
            self.repo.mark_daily_claimed_in_tx(&mut tx, &cmd.guild_id, &cmd.user_id).await?;
        }

        tx.commit().await.map_err(|e| DomainError::Internal(format!("commit tx slot: {e}")))?;

        // 8. Post-commit : taunts faillite/jackpot eco.
        let mut triggered_taunts: Vec<TauntEvent> = Vec::new();
        for (user_id, mutation) in &taunt_mutations {
            let evs = self.wallet_uc.post_commit_taunts(&cmd.guild_id, user_id, mutation).await;
            triggered_taunts.extend(evs);
        }

        // 9. Solde apres operation.
        let balance_after = self.wallet_uc.get_balance(&cmd.guild_id, &cmd.user_id).await?;

        Ok(SpinResult { spin, jackpot_pool_after, balance_after, triggered_taunts })
    }
}

#[async_trait]
impl ManageSlotUseCase for ManageSlotService {
    async fn spin(&self, cmd: SpinCommand) -> Result<SpinResult, DomainError> {
        let cfg = self.load_config(&cmd.guild_id).await?;

        // Validation mise.
        if cmd.mise < cfg.min_bet || cmd.mise > cfg.max_bet {
            return Err(DomainError::ValidationError(format!(
                "Mise hors borne (autorise : {} - {})", cfg.min_bet, cfg.max_bet
            )));
        }

        // Cooldown.
        if cfg.cooldown_secs > 0 {
            if let Some(last) = self.repo.last_spin_at(&cmd.guild_id, &cmd.user_id).await? {
                let elapsed = (Utc::now() - last).num_seconds();
                if elapsed < cfg.cooldown_secs as i64 {
                    let remaining = cfg.cooldown_secs as i64 - elapsed;
                    return Err(DomainError::ValidationError(format!(
                        "Cooldown actif : encore {} secondes", remaining
                    )));
                }
            }
        }

        // Init pool jackpot si premiere fois.
        self.repo.init_jackpot_pool_if_absent(&cmd.guild_id, cfg.jackpot_starting_pool).await?;

        let mut payable = cmd.clone();
        payable.is_daily = false;
        self.run_spin(&payable, &cfg).await
    }

    async fn claim_daily_bonus(&self, cmd: SpinCommand) -> Result<SpinResult, DomainError> {
        let cfg = self.load_config(&cmd.guild_id).await?;

        if !cfg.daily_bonus_enabled {
            return Err(DomainError::ValidationError("Daily bonus desactive sur ce serveur".into()));
        }

        if self.repo.has_claimed_daily_today(&cmd.guild_id, &cmd.user_id).await? {
            return Err(DomainError::ValidationError("Daily bonus deja reclame aujourd hui".into()));
        }

        // Init pool jackpot si premiere fois.
        self.repo.init_jackpot_pool_if_absent(&cmd.guild_id, cfg.jackpot_starting_pool).await?;

        let mut daily = cmd.clone();
        daily.is_daily = true;
        self.run_spin(&daily, &cfg).await
    }

    async fn get_jackpot_pool(&self, guild_id: &str) -> Result<i64, DomainError> {
        Ok(self.repo.get_jackpot_pool(guild_id).await?
            .map(|p| p.current_pool)
            .unwrap_or(0))
    }

    async fn recent_spins(&self, guild_id: &str, limit: i64) -> Result<Vec<SlotSpin>, DomainError> {
        self.repo.recent_spins(guild_id, limit).await
    }

    async fn top_winners(
        &self,
        guild_id: &str,
        days: i64,
        limit: i64,
    ) -> Result<Vec<SlotTopWinner>, DomainError> {
        self.repo.top_winners(guild_id, days, limit).await
    }
}

fn parse_bool(s: &str, default: bool) -> bool {
    match s.to_lowercase().as_str() {
        "true" | "1" | "on" | "yes" => true,
        "false" | "0" | "off" | "no" => false,
        _ => default,
    }
}

#[cfg(test)]
#[path = "tests/manage_slot.rs"]
mod tests;
