//! Impl du systeme de braquage (Phase 10).
//!
//! Orchestre :
//! - cooldown hebdo (coude_heist_repo.last_attempt)
//! - prison (coude_heist_repo.get_prison / send_to_prison)
//! - inventaire d'outils (coude_inventory_uc.list_inventory)
//! - caisse communautaire (cashbox_repo.claim_all_for_redistribution
//!   adapte, ou lecture + deposit negative — on choisit un claim
//!   partiel custom ici)
//! - wallet du joueur (wallet_repo.credit/debit)
//! - consommation des outils (inventory_uc.use_item)

#[cfg(test)]
#[path = "tests/manage_heist.rs"]
mod tests;

use std::sync::Arc;

use async_trait::async_trait;
use chrono::Duration as ChronoDuration;
use chrono::Utc;
use rand::Rng;
use tracing::info;

use crate::domain::entities::coude::balance::BalanceParams;
use crate::domain::entities::coude::heist::compute_success_chance;
use crate::domain::entities::coude::heist::HeistOutcome;
use crate::domain::entities::coude::heist::HEIST_TOOLS;
use crate::domain::errors::DomainError;
use crate::ports::inbound::coude::manage_heist::HeistCooldownStatus;
use crate::ports::inbound::coude::manage_heist::ManageCoudeHeistUseCase;
use crate::ports::inbound::coude::manage_heist::PrisonStatusInfo;
use crate::ports::inbound::coude::manage_inventory::ManageCoudeInventoryUseCase;
use crate::ports::outbound::casino::wallet_repository::WalletRepository;
use crate::ports::outbound::coude::cashbox_repository::CashboxRepository;
use crate::ports::outbound::coude::heist_repository::HeistRepository;
use crate::ports::outbound::coude::player_repository::PlayerRepository;
use crate::ports::outbound::coude::social_repository::SocialRepository;
use crate::ports::outbound::system::bot_config_repository::BotConfigRepository;

/// Clé du verrou de cooldown atomique pour le braquage (table
/// `coude_cooldowns` via `SocialRepository`). Réutilise l'infra anti-TOCTOU
/// de tout-ou-rien : `try_claim_cooldown` garantit qu'un seul `/braquage`
/// concurrent remporte le claim, les autres échouent AVANT toute mutation
/// caisse/wallet (anti double-drain + double-gain #2).
const HEIST_COOLDOWN_KEY: &str = "heist";

pub struct ManageCoudeHeistService {
    heist_repo: Arc<dyn HeistRepository>,
    cashbox_repo: Arc<dyn CashboxRepository>,
    inventory_uc: Arc<dyn ManageCoudeInventoryUseCase>,
    wallet_repo: Arc<dyn WalletRepository>,
    bot_config_repo: Arc<dyn BotConfigRepository>,
    social_repo: Arc<dyn SocialRepository>,
    player_repo: Option<Arc<dyn PlayerRepository>>,
}

impl ManageCoudeHeistService {
    pub fn new(
        heist_repo: Arc<dyn HeistRepository>,
        cashbox_repo: Arc<dyn CashboxRepository>,
        inventory_uc: Arc<dyn ManageCoudeInventoryUseCase>,
        wallet_repo: Arc<dyn WalletRepository>,
        bot_config_repo: Arc<dyn BotConfigRepository>,
        social_repo: Arc<dyn SocialRepository>,
    ) -> Self {
        Self {
            heist_repo,
            cashbox_repo,
            inventory_uc,
            wallet_repo,
            bot_config_repo,
            social_repo,
            player_repo: None,
        }
    }

    /// Branche le repo player (cf. COUPE_AMELIORATIONS 6.3) pour
    /// appliquer le multiplicateur de cooldown de la "Saison du
    /// Braquage".
    pub fn with_player_repo(mut self, repo: Arc<dyn PlayerRepository>) -> Self {
        self.player_repo = Some(repo);
        self
    }

    /// Calcule le cooldown effectif (en jours) en appliquant le
    /// multiplicateur de saison thematique si dispo.
    async fn effective_cooldown_days(
        &self,
        guild_id: &str,
        user_id: &str,
        base_cooldown_days: i64,
    ) -> i64 {
        let Some(repo) = &self.player_repo else {
            return base_cooldown_days;
        };
        let Ok(Some(player)) = repo.get(guild_id, user_id).await else {
            return base_cooldown_days;
        };
        crate::domain::entities::coude::season_theme::apply_season_braquage_cooldown(
            player.season,
            base_cooldown_days,
        )
    }

    async fn load_balance(&self, guild_id: &str) -> BalanceParams {
        crate::application::coude::guild_settings::load_balance_params(
            &*self.bot_config_repo,
            guild_id,
        )
        .await
    }

    async fn load_economy(
        &self,
        guild_id: &str,
    ) -> crate::domain::entities::coude::economy_config::CoudeEconomyConfig {
        crate::application::coude::guild_settings::load_economy_config(
            &*self.bot_config_repo,
            guild_id,
        )
        .await
    }
}

#[async_trait]
impl ManageCoudeHeistUseCase for ManageCoudeHeistService {
    async fn get_cooldown_status(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<HeistCooldownStatus, DomainError> {
        let last = self.heist_repo.last_attempt(guild_id, user_id).await?;
        let Some(last) = last else {
            return Ok(HeistCooldownStatus {
                ready: true,
                next_attempt_at: None,
                last_success: None,
            });
        };
        let params = self.load_balance(guild_id).await;
        let base_cooldown_days = params.heist_cooldown_days.max(1) as i64;
        let cooldown_days = self
            .effective_cooldown_days(guild_id, user_id, base_cooldown_days)
            .await;
        let next = last.attempted_at + ChronoDuration::days(cooldown_days);
        let ready = Utc::now() >= next;
        Ok(HeistCooldownStatus {
            ready,
            next_attempt_at: Some(next),
            last_success: Some(last.success),
        })
    }

    async fn get_prison_status(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<PrisonStatusInfo, DomainError> {
        let state = self.heist_repo.get_prison(guild_id, user_id).await?;
        let Some(state) = state else {
            return Ok(PrisonStatusInfo {
                in_prison: false,
                released_at: None,
                reason: None,
            });
        };
        let in_prison = state.released_at > Utc::now();
        Ok(PrisonStatusInfo {
            in_prison,
            released_at: Some(state.released_at),
            reason: Some(state.reason),
        })
    }

    async fn attempt_heist(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<HeistOutcome, DomainError> {
        // 1. Check prison
        let prison = self.get_prison_status(guild_id, user_id).await?;
        if prison.in_prison {
            return Err(DomainError::Forbidden(
                "Tu es en prison ! Impossible de braquer avant liberation.".into(),
            ));
        }

        // 2. Check cooldown configurable (default 7 jours, cf. BalanceParams).
        //    Le multiplicateur de "Saison du Braquage" est applique dans
        //    get_cooldown_status (cf. COUPE_AMELIORATIONS 6.3).
        let params_early = self.load_balance(guild_id).await;
        let cooldown = self.get_cooldown_status(guild_id, user_id).await?;
        if !cooldown.ready {
            let effective_days = self
                .effective_cooldown_days(guild_id, user_id, params_early.heist_cooldown_days as i64)
                .await;
            return Err(DomainError::Forbidden(format!(
                "Cooldown {} jours non ecoule.",
                effective_days
            )));
        }

        // 3. Charger la caisse : on veut juste lire le solde pour savoir
        //    s'il vaut le coup de tenter (sinon abort). On ne vide pas
        //    tout : meme sur succes, on ne prend qu'un % aleatoire.
        let cashbox = self.cashbox_repo.get_or_create(guild_id).await?;
        if cashbox.balance <= 0 {
            return Err(DomainError::Forbidden(
                "La caisse est vide, inutile de tenter.".into(),
            ));
        }

        // 3.b CLAIM ATOMIQUE du cooldown hebdo AVANT toute mutation
        //     caisse/wallet (fix #2 — anti double-drain + double-gain). Deux
        //     `/braquage` concurrents passent tous deux les checks read-only
        //     ci-dessus (prison, cooldown via last_attempt, caisse non vide),
        //     mais un seul remporte ce claim ; l'autre échoue ici sans toucher
        //     à l'argent. Le TTL = fenêtre de cooldown effective, cohérent avec
        //     `record_attempt` (qui reste la source de vérité persistante lue
        //     par `get_cooldown_status`).
        let base_cooldown_days = params_early.heist_cooldown_days.max(1) as i64;
        let effective_days = self
            .effective_cooldown_days(guild_id, user_id, base_cooldown_days)
            .await;
        let ttl_secs = effective_days.saturating_mul(86_400).max(1);
        let claimed = self
            .social_repo
            .try_claim_cooldown(guild_id, user_id, HEIST_COOLDOWN_KEY, ttl_secs)
            .await?;
        if !claimed {
            return Err(DomainError::Forbidden(format!(
                "Cooldown {} jours non ecoule.",
                effective_days
            )));
        }

        // À partir d'ici le verrou est posé : tout échec de la section protégée
        // libère le claim (best-effort) pour ne pas pénaliser un braquage qui
        // n'a rien payé (mirror tout-ou-rien : release-on-failure).
        let result = self
            .run_heist_locked(guild_id, user_id, cashbox.balance, &params_early)
            .await;
        if result.is_err() {
            if let Err(e) = self
                .social_repo
                .clear_cooldown(guild_id, user_id, HEIST_COOLDOWN_KEY)
                .await
            {
                tracing::warn!(
                    error = %e,
                    guild_id,
                    user_id,
                    "Echec liberation cooldown heist apres echec mutation"
                );
            }
        }
        result
    }
}

impl ManageCoudeHeistService {
    /// Section du braquage exécutée APRÈS l'obtention du verrou de cooldown
    /// atomique. Toute erreur ici déclenche la libération du claim côté
    /// `attempt_heist`. `cashbox_total_before` est le solde caisse lu lors du
    /// check read-only (avant claim), exposé tel quel dans `HeistOutcome`.
    ///
    /// # Atomicité caisse ↔ wallet
    ///
    /// `cashbox.withdraw` + `wallet.credit` restent deux transactions
    /// distinctes (le `CashboxRepository` n'expose pas de withdraw in-tx et
    /// `WalletRepository.credit` n'est pas tx-aware sur le même pool). Le claim
    /// atomique élimine la **duplication** (double-drain / double-gain), qui
    /// était le bug de mint/double-spend. Le résidu — withdraw OK puis credit
    /// KO ⇒ coins retirés de la caisse mais non crédités — est *déflationniste*
    /// (jamais de mint), borné par l'ordre withdraw→credit et rattrapé par le
    /// CHECK `coins >= 0`. Une vraie UoW (DbTx partagé withdraw+credit_tx) est
    /// la suite propre mais hors scope conservateur ici.
    async fn run_heist_locked(
        &self,
        guild_id: &str,
        user_id: &str,
        cashbox_total_before: i64,
        params_early: &BalanceParams,
    ) -> Result<HeistOutcome, DomainError> {
        // 4. Lister les outils de braquage actifs dans l'inventaire.
        //    Chaque tool key present (quantity > 0) compte une fois.
        let inventory = self.inventory_uc.list_inventory(guild_id, user_id).await?;
        let tool_keys: Vec<String> = inventory
            .iter()
            .filter(|i| i.quantity > 0)
            .map(|i| i.item_key.clone())
            .filter(|k| HEIST_TOOLS.iter().any(|t| t.key == k.as_str()))
            .collect();

        // Config ECONOMY réglable par serveur (base/plafond de réussite +
        // bornes de gain du braquage). Domaine PUR : passée en donnée.
        let econ = self.load_economy(guild_id).await;

        // 5. Calcule la chance effective (domain pur).
        let chance = compute_success_chance(&tool_keys, &econ);

        // 6. Roll + decide. On scope ThreadRng pour rester Send.
        //    `econ` garantit gain_min <= gain_max (cf. sanitize).
        let (success, gain_percent) = {
            let mut rng = rand::thread_rng();
            let roll: u32 = rng.gen_range(1..=100);
            let success = roll <= chance;
            let gain: u32 = rng.gen_range(econ.heist_gain_min_pct..=econ.heist_gain_max_pct);
            (success, gain)
        };

        // 7. Consomme une fraction aleatoire des outils utilises (Phase 132) :
        //    * succes → `braquage_tools_consumed_success_pct` (%)
        //    * echec  → `braquage_tools_consumed_fail_pct`    (%)
        //    Selection aleatoire parmi les outils actifs. Si l'erreur
        //    survient en cours de consommation, on arrete et on remonte
        //    l'erreur (meme semantique qu'avant).
        let balance = self.load_balance(guild_id).await;
        let consumed_pct = if success {
            balance.braquage_tools_consumed_success_pct
        } else {
            balance.braquage_tools_consumed_fail_pct
        };
        let tools_to_consume: Vec<String> = {
            use rand::seq::SliceRandom;
            let total = tool_keys.len();
            let count = ((total as u64 * consumed_pct) / 100) as usize;
            let count = count.min(total);
            let mut rng = rand::thread_rng();
            let mut shuffled: Vec<String> = tool_keys.clone();
            shuffled.shuffle(&mut rng);
            shuffled.into_iter().take(count).collect()
        };
        for key in &tools_to_consume {
            self.inventory_uc.use_item(guild_id, user_id, key).await?;
        }

        // 8. Calcule le montant vole (capture instantane, la caisse peut
        //    bouger entre get_or_create et le prelevement — acceptable :
        //    on prend gain % du solde courant).
        let amount_stolen: i64 = if success {
            let refreshed = self.cashbox_repo.get_or_create(guild_id).await?;
            let balance = refreshed.balance.max(0);
            // Arithmetique i128 pour eviter perte de precision f64 sur
            // gros soldes (> 2^53) et tout risque d'overflow i64 sur
            // balance * gain_percent.
            ((balance as i128) * (gain_percent as i128) / 100) as i64
        } else {
            0
        };

        if success && amount_stolen > 0 {
            // Prelever de la caisse : on utilise un deposit negatif ? Non,
            // la signature refuse amount <= 0. On passe par une methode
            // dedie ? Pour rester minimal, on fait directement une UPDATE
            // via le repo : mais on n'a pas de `withdraw` methode. On
            // recourre donc a un hack temporaire : on fait un claim_all
            // puis on re-deposit la difference. C'est atomique cote DB
            // mais moche. Alternative propre : ajouter une methode
            // withdraw au CashboxRepository. Faisons-le.
            self.cashbox_repo.withdraw(guild_id, amount_stolen).await?;

            // Credit le wallet du joueur
            self.wallet_repo
                .credit(
                    guild_id,
                    user_id,
                    amount_stolen,
                    "coude_heist_success",
                    "Braquage de la caisse reussi",
                )
                .await?;
        }

        // 9. Log la tentative (pour le cooldown)
        let _ = self
            .heist_repo
            .record_attempt(
                guild_id,
                user_id,
                success,
                amount_stolen,
                chance as i32,
                &tool_keys,
            )
            .await?;

        // 10. Si echec → prison (duree configurable, default 24h).
        let prison_released_at = if !success {
            let prison_hours = params_early.heist_prison_hours.max(1) as i64;
            let released = Utc::now() + ChronoDuration::hours(prison_hours);
            self.heist_repo
                .send_to_prison(guild_id, user_id, released, "heist_failed")
                .await?;
            Some(released)
        } else {
            None
        };

        info!(
            guild_id,
            user_id,
            success,
            chance,
            amount_stolen,
            tools = tool_keys.len(),
            "Heist attempt resolved"
        );

        Ok(HeistOutcome {
            success,
            chance_percent: chance,
            cashbox_total_before,
            amount_stolen,
            tools_consumed: tools_to_consume,
            prison_released_at,
        })
    }
}
