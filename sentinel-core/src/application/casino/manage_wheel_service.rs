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

use crate::domain::entities::casino::wheel::is_memorable_case;
use crate::domain::entities::casino::wheel::spin_with_rng_curses_cfg as wheel_spin_with_rng_curses_cfg;
use crate::domain::entities::casino::wheel::WheelConfig;
use crate::domain::entities::casino::wheel::WheelSpin;
use crate::domain::entities::casino::wheel::WheelTopWinner;
use crate::domain::entities::casino::wheel::WHEEL_CASES;
use crate::domain::entities::coude::curse::CurseKind;
use crate::domain::entities::coude::taunt::TauntEvent;
use crate::domain::errors::DomainError;
use crate::ports::inbound::casino::manage_wallet::ManageWalletUseCase;
use crate::ports::inbound::casino::manage_wheel::ManageWheelUseCase;
use crate::ports::inbound::casino::manage_wheel::WheelSpinCommand;
use crate::ports::inbound::casino::manage_wheel::WheelSpinResult;
use crate::ports::outbound::casino::wheel_repository::WheelRepository;
use crate::ports::outbound::coude::curses_repository::CursesRepository;
use crate::ports::outbound::system::bot_config_repository::BotConfigRepository;
use crate::ports::uow::UnitOfWork;
const MODULE_BOT_NAME: &str = "wheel-bot";
pub struct ManageWheelService {
    repo: Arc<dyn WheelRepository>,
    wallet_uc: Arc<dyn ManageWalletUseCase>,
    curses_repo: Option<Arc<dyn CursesRepository>>,
    bot_config_repo: Option<Arc<dyn BotConfigRepository>>,
    uow: Arc<dyn UnitOfWork>,
}

impl ManageWheelService {
    pub fn new(
        repo: Arc<dyn WheelRepository>,
        wallet_uc: Arc<dyn ManageWalletUseCase>,
        uow: Arc<dyn UnitOfWork>,
    ) -> Self {
        Self {
            repo,
            wallet_uc,
            curses_repo: None,
            bot_config_repo: None,
            uow,
        }
    }

    /// Branche le repo des maledictions pour activer "Heartbreak"
    /// (cf. COUPE_AMELIORATIONS 5.1) : le spinner maudit ne peut pas
    /// tomber sur la licorne. Optionnel pour preserver les call-sites
    /// de test et eviter une regression silencieuse.
    pub fn with_curses_repo(mut self, repo: Arc<dyn CursesRepository>) -> Self {
        self.curses_repo = Some(repo);
        self
    }

    /// Branche le repo de config pour rendre les payouts/poids des cases
    /// editables par serveur (`wheel-bot`). Optionnel : sans lui, la Roue
    /// utilise les payouts/poids par defaut (`WheelConfig::default`).
    pub fn with_bot_config_repo(mut self, repo: Arc<dyn BotConfigRepository>) -> Self {
        self.bot_config_repo = Some(repo);
        self
    }

    /// Charge la config `wheel-bot` de la guild et la decode en `WheelConfig`.
    /// Chaque cle absente retombe sur le defaut de la case ; garde-fous
    /// (clamp payout ±50000, somme des poids > 0) appliques via `normalized()`.
    async fn load_config(&self, guild_id: &str) -> WheelConfig {
        let Some(repo) = &self.bot_config_repo else {
            return WheelConfig::default();
        };
        let entries = match repo.get_config(guild_id, MODULE_BOT_NAME).await {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!(
                    event_type = "wheel.config_load_failed",
                    guild_id = %guild_id,
                    error = %e,
                    "Echec lecture config wheel-bot, utilisation des defauts"
                );
                return WheelConfig::default();
            }
        };

        let mut cfg = WheelConfig::default();
        for entry in &entries {
            // Cles : wheel_<segment>_payout / wheel_<segment>_weight
            let Some(rest) = entry.config_key.strip_prefix("wheel_") else {
                continue;
            };
            if let Some(seg_key) = rest.strip_suffix("_payout") {
                if let Some(idx) = WHEEL_CASES.iter().position(|c| c.key == seg_key) {
                    if let Ok(v) = entry.config_value.parse::<i64>() {
                        cfg.segments[idx].payout = v;
                    }
                }
            } else if let Some(seg_key) = rest.strip_suffix("_weight") {
                if let Some(idx) = WHEEL_CASES.iter().position(|c| c.key == seg_key) {
                    if let Ok(v) = entry.config_value.parse::<u32>() {
                        cfg.segments[idx].weight = v;
                    }
                }
            }
        }

        cfg.normalized()
    }
}

#[async_trait]
impl ManageWheelUseCase for ManageWheelService {
    async fn spin(&self, cmd: WheelSpinCommand) -> Result<WheelSpinResult, DomainError> {
        // 1. Verif daily.
        if self
            .repo
            .has_claimed_today(&cmd.guild_id, &cmd.user_id)
            .await?
        {
            return Err(DomainError::ValidationError(
                "Tu as deja tire la Roue du Destin aujourd hui.".into(),
            ));
        }

        // 2. Detection malediction "Heartbreak" — bloque la licorne pour
        //    cette tirage. Echec silencieux : si le repo casse, on spin
        //    quand meme normalement (le user ne perdra rien de plus).
        let block_licorne = if let Some(curses_repo) = &self.curses_repo {
            matches!(
                curses_repo
                    .get_active_for_target(&cmd.guild_id, &cmd.user_id)
                    .await,
                Ok(Some(c)) if c.kind == CurseKind::Heartbreak
            )
        } else {
            false
        };

        // 3. Spin RNG (entropie OS). Config payouts/poids editable par serveur.
        let config = self.load_config(&cmd.guild_id).await;
        let mut rng = rand::rngs::StdRng::from_entropy();
        let outcome = wheel_spin_with_rng_curses_cfg(&mut rng, block_licorne, &config);
        let payout = outcome.case.payout;

        // 3. Tx atomique.
        let mut tx = self.uow.begin().await?;

        let mut taunt_mutations = Vec::new();

        // Claim atomique du tirage du jour AVANT tout credit. ON CONFLICT DO
        // NOTHING : seule la premiere tx concurrente obtient `true` et continue
        // vers le payout. Une tx perdante recoit `false`, abandonne (rollback
        // implicite au drop) et ne paie RIEN. Le `has_claimed_today` en amont
        // reste un fast-path non-atomique.
        let claimed = self
            .repo
            .mark_claimed_in_tx(&mut *tx, &cmd.guild_id, &cmd.user_id)
            .await?;
        if !claimed {
            return Err(DomainError::ValidationError(
                "Tu as deja tire la Roue du Destin aujourd hui.".into(),
            ));
        }

        // Wallet : credit ou debit selon le signe du payout.
        if payout > 0 {
            let m = self
                .wallet_uc
                .credit_tx(
                    &mut *tx,
                    &cmd.guild_id,
                    &cmd.user_id,
                    payout,
                    "wheel_payout",
                    &format!("Roue du Destin : {}", outcome.case.label),
                )
                .await?;
            taunt_mutations.push((cmd.user_id.clone(), m));
        } else if payout < 0 {
            // Clamp : on ne peut pas debiter plus que le solde.
            let balance = self
                .wallet_uc
                .get_balance(&cmd.guild_id, &cmd.user_id)
                .await?;
            let actual_debit = (-payout).min(balance);
            if actual_debit > 0 {
                let m = self
                    .wallet_uc
                    .debit_tx(
                        &mut *tx,
                        &cmd.guild_id,
                        &cmd.user_id,
                        actual_debit,
                        "wheel_loss",
                        &format!("Roue du Destin : {}", outcome.case.label),
                    )
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
        self.repo.log_spin_in_tx(&mut *tx, &spin).await?;

        // Note : le mark daily est effectue en tete de tx (claim atomique).

        self.uow.commit(tx).await?;

        // 4. Post-commit taunts.
        let mut triggered_taunts: Vec<TauntEvent> = Vec::new();
        for (uid, mutation) in &taunt_mutations {
            let evs = self
                .wallet_uc
                .post_commit_taunts(&cmd.guild_id, uid, mutation)
                .await;
            triggered_taunts.extend(evs);
        }

        let balance_after = self
            .wallet_uc
            .get_balance(&cmd.guild_id, &cmd.user_id)
            .await?;
        let is_memorable = is_memorable_case(outcome.case.key);

        Ok(WheelSpinResult {
            spin,
            case: outcome.case,
            balance_after,
            is_memorable,
            triggered_taunts,
        })
    }

    async fn recent_spins(
        &self,
        guild_id: &str,
        limit: i64,
    ) -> Result<Vec<WheelSpin>, DomainError> {
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
