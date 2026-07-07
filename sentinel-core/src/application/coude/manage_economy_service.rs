use std::sync::Arc;

use async_trait::async_trait;

use crate::domain::entities::coude::curse::CurseKind;
use crate::domain::entities::coude::economy::clamp_steal_amount;
use crate::domain::entities::coude::economy::clamp_steal_fail_penalty;
use crate::domain::entities::coude::taunt::TauntEvent;
use crate::domain::errors::DomainError;
use crate::ports::inbound::casino::manage_wallet::ManageWalletUseCase;
use crate::ports::inbound::coude::manage_economy::GiftOutcome;
use crate::ports::inbound::coude::manage_economy::ManageCoudeEconomyUseCase;
use crate::ports::inbound::coude::manage_economy::StealOutcome;
use crate::ports::inbound::coude::manage_taunts::ManageCoudeTauntsUseCase;
use crate::ports::outbound::casino::wallet_repository::WalletRepository;
use crate::ports::outbound::coude::curses_repository::CursesRepository;
use crate::ports::outbound::coude::economy_repository::EconomyRepository;
use crate::ports::outbound::coude::player_repository::PlayerRepository;
use crate::ports::outbound::system::bot_config_repository::BotConfigRepository;
/// Service "economie Coup de Coude".
///
/// # Migration wallet unifie (PoC `/donner`)
///
/// La methode `transfer` ne passe plus par `EconomyRepository` : elle
/// delegue a `ManageWalletUseCase::transfer` (qui centralise le SQL
/// atomique + la detection faillite/jackpot) puis appelle
/// `ManageCoudeTauntsUseCase::on_generous_donor` pour le taunt specifique
/// "don genereux" (qui n'est pas detectable par le service wallet seul).
/// Les `TauntEvent` sont concatenes et retournes pour que la couche
/// transport (gRPC/HTTP) les propage au bot, qui les dispatche en un
/// seul aller-retour.
///
/// Les autres methodes (`steal`, casino, compteurs) continuent d'utiliser
/// le repo economy : elles seront migrees progressivement sur le meme
/// pattern.
pub struct ManageCoudeEconomyService {
    repo: Arc<dyn EconomyRepository>,
    wallet_uc: Arc<dyn ManageWalletUseCase>,
    taunts_uc: Arc<dyn ManageCoudeTauntsUseCase>,
    wallet_repo: Option<Arc<dyn WalletRepository>>,
    curses_repo: Option<Arc<dyn CursesRepository>>,
    player_repo: Option<Arc<dyn PlayerRepository>>,
    bot_config_repo: Option<Arc<dyn BotConfigRepository>>,
}

impl ManageCoudeEconomyService {
    pub fn new(
        repo: Arc<dyn EconomyRepository>,
        wallet_uc: Arc<dyn ManageWalletUseCase>,
        taunts_uc: Arc<dyn ManageCoudeTauntsUseCase>,
    ) -> Self {
        Self {
            repo,
            wallet_uc,
            taunts_uc,
            wallet_repo: None,
            curses_repo: None,
            player_repo: None,
            bot_config_repo: None,
        }
    }

    /// Branche le repo de config bot : le montant des frais "Portefeuille
    /// troué" (LeakyWallet) devient réglable par serveur via `coude-bot`.
    /// Sans repo : valeur par défaut historique (10c).
    pub fn with_bot_config_repo(mut self, repo: Arc<dyn BotConfigRepository>) -> Self {
        self.bot_config_repo = Some(repo);
        self
    }

    /// Lit une cle de config guild entiere (defaut si repo absent/cle absente).
    async fn config_i64(&self, guild_id: &str, key: &str, default: i64) -> i64 {
        match &self.bot_config_repo {
            Some(repo) => {
                crate::application::coude::guild_settings::GuildSettings::load(&**repo, guild_id)
                    .await
                    .get_i64(key, default)
            }
            None => default,
        }
    }

    /// Lit une cle de config guild en ratio de pourcentage (stockee en
    /// entier, ex. 10 => 0.10). Defaut si repo absent/cle absente.
    async fn config_percent_ratio(&self, guild_id: &str, key: &str, default_pct: i64) -> f64 {
        match &self.bot_config_repo {
            Some(repo) => {
                crate::application::coude::guild_settings::GuildSettings::load(&**repo, guild_id)
                    .await
                    .get_percent_ratio(key, default_pct)
            }
            None => default_pct as f64 / 100.0,
        }
    }

    async fn leaky_wallet_fee(&self, guild_id: &str) -> i64 {
        match &self.bot_config_repo {
            Some(repo) => {
                crate::application::coude::guild_settings::load_economy_config(&**repo, guild_id)
                    .await
                    .leaky_wallet_fee_coins
            }
            None => {
                crate::domain::entities::coude::economy_config::CoudeEconomyConfig::default()
                    .leaky_wallet_fee_coins
            }
        }
    }

    /// Branche les repos necessaires au LeakyWallet (cf. COUPE_AMELIORATIONS
    /// 5.1). Si l emetteur d un /donner est sous l effet, 10c sont preleves
    /// en plus en frais (le destinataire recoit `amount` complet).
    pub fn with_leaky_wallet_support(
        mut self,
        wallet_repo: Arc<dyn WalletRepository>,
        curses_repo: Arc<dyn CursesRepository>,
    ) -> Self {
        self.wallet_repo = Some(wallet_repo);
        self.curses_repo = Some(curses_repo);
        self
    }

    /// Branche le repo player pour appliquer le multiplicateur de
    /// "Saison du Vol" (cf. COUPE_AMELIORATIONS 6.3) : gains x1.5
    /// pour le voleur, paye depuis le neant (server-paid bonus).
    pub fn with_player_repo(mut self, player_repo: Arc<dyn PlayerRepository>) -> Self {
        self.player_repo = Some(player_repo);
        self
    }

    async fn has_leaky_wallet(&self, guild_id: &str, user_id: &str) -> bool {
        let Some(repo) = &self.curses_repo else {
            return false;
        };
        matches!(
            repo.get_active_for_target(guild_id, user_id).await,
            Ok(Some(c)) if c.kind == CurseKind::LeakyWallet
        )
    }

    /// Bonus en coins a creer ex-nihilo si la "Saison du Vol" est
    /// active pour le voleur. Retourne 0 si pas de saison ou pas
    /// de player_repo branche.
    async fn season_steal_bonus(&self, guild_id: &str, thief_id: &str, stolen: i64) -> i64 {
        let Some(repo) = &self.player_repo else {
            return 0;
        };
        let Ok(Some(player)) = repo.get(guild_id, thief_id).await else {
            return 0;
        };
        crate::domain::entities::coude::season_theme::compute_season_steal_bonus(
            player.season,
            stolen,
        )
    }
}

fn require_positive(amount: i64) -> Result<(), DomainError> {
    crate::application::validation::validate_positive(amount, "Le montant")
}

#[async_trait]
impl ManageCoudeEconomyUseCase for ManageCoudeEconomyService {
    async fn transfer(
        &self,
        guild_id: &str,
        from_id: &str,
        to_id: &str,
        amount: i64,
    ) -> Result<Vec<TauntEvent>, DomainError> {
        require_positive(amount)?;
        if from_id == to_id {
            return Err(DomainError::ValidationError(
                "Impossible de se transferer des coins a soi-meme".into(),
            ));
        }

        // 1. Mutation atomique via le wallet UC centralise : SELECT FOR
        //    UPDATE + UPDATE debit/credit + INSERT wallet_transactions dans
        //    la meme tx. Erreur propre si solde insuffisant ou destinataire
        //    inexistant (rollback automatique).
        let description = format!("Don entre joueurs ({} -> {})", from_id, to_id);
        let mut taunts = self
            .wallet_uc
            .transfer(
                guild_id,
                from_id,
                to_id,
                amount,
                "coude_donner",
                &description,
            )
            .await?;

        // 1.bis Branchement Leaky Wallet (cf. COUPE_AMELIORATIONS 5.1) :
        //       si l emetteur est maudit, 10c supplementaires sont preleves
        //       en frais (best-effort hors-tx — si le debit echoue, le don
        //       est deja passe : on log et on continue).
        if self.has_leaky_wallet(guild_id, from_id).await {
            if let Some(wallet_repo) = &self.wallet_repo {
                let fee = self.leaky_wallet_fee(guild_id).await;
                if fee > 0 {
                    if let Err(e) = wallet_repo
                        .debit(
                            guild_id,
                            from_id,
                            fee,
                            "curse_leaky_wallet",
                            "Frais Portefeuille troue",
                        )
                        .await
                    {
                        tracing::warn!(
                            error = %e, guild_id, from_id,
                            "leaky wallet : echec prelevement frais (don deja passe)"
                        );
                    }
                }
            }
        }

        // 2. Don genereux : taunt specifique cote emetteur si amount >=
        //    threshold configure (logique interne au service taunts). Ne
        //    peut pas etre detecte par le wallet service car c'est
        //    domaine-specifique Coup de Coude.
        if let Ok(Some(ev)) = self
            .taunts_uc
            .on_generous_donor(guild_id, from_id, amount)
            .await
        {
            taunts.push(ev);
        }

        Ok(taunts)
    }

    async fn gift_coins(
        &self,
        guild_id: &str,
        donor_id: &str,
        target_id: &str,
        amount: i64,
    ) -> Result<GiftOutcome, DomainError> {
        require_positive(amount)?;
        if donor_id == target_id {
            return Err(DomainError::ValidationError(
                "Impossible de se donner a soi-meme".into(),
            ));
        }

        // Invariants economiques lus server-side (config guild, memes cles et
        // defauts que l'ancien bot : `gift_tax_percent`=10, `gift_min_coins_after`=50).
        let tax_rate = self
            .config_percent_ratio(guild_id, "gift_tax_percent", 10)
            .await;
        let min_coins_after = self.config_i64(guild_id, "gift_min_coins_after", 50).await;

        // Regle metier : conserver un solde minimum apres le don (validee
        // cote serveur, plus dans le bot).
        let balance = self.wallet_uc.get_balance(guild_id, donor_id).await?;
        if balance - amount < min_coins_after {
            return Err(DomainError::ValidationError(format!(
                "Tu dois garder au moins {min_coins_after} coins apres le don."
            )));
        }

        // Calcul de la taxe cote serveur (la regle ne vit plus dans le bot).
        let tax = ((amount as f64) * tax_rate).ceil() as i64;
        let received = amount - tax;

        // Transfert atomique de la part recue (reutilise `transfer` : faillite/
        // jackpot/don-genereux detectes comme avant, calcule sur `received` —
        // comportement identique au legacy qui transferait deja `received`).
        let taunts = self
            .transfer(guild_id, donor_id, target_id, received)
            .await?;

        // Debit de la taxe a l'emetteur (best-effort : le don est deja passe).
        if tax > 0 {
            if let Some(wallet_repo) = &self.wallet_repo {
                if let Err(e) = wallet_repo
                    .debit(guild_id, donor_id, tax, "coude_gift_tax", "Taxe sur don")
                    .await
                {
                    tracing::warn!(error = %e, guild_id, donor_id, tax, "Echec debit taxe don (don deja passe)");
                }
            }
        }

        Ok(GiftOutcome {
            received,
            tax,
            taunt_events: taunts,
        })
    }

    async fn steal(
        &self,
        guild_id: &str,
        thief_id: &str,
        victim_id: &str,
        amount: i64,
    ) -> Result<StealOutcome, DomainError> {
        // 1. Validations pre-I/O : on echoue avant get_coins pour eviter
        //    qu'un wallet absent ne masque une self-steal ou un montant <= 0.
        require_positive(amount)?;
        if thief_id == victim_id {
            return Err(DomainError::ValidationError(
                "Impossible de se voler soi-meme".into(),
            ));
        }

        // 2. Lecture solde + clamp pur (domain). On lit hors-tx : si un autre
        //    evenement modifie le solde entre le read et le transfer,
        //    wallet_uc.transfer echouera proprement ("Solde insuffisant").
        let victim_coins = self.repo.get_coins(guild_id, victim_id).await?;
        let stolen = clamp_steal_amount(thief_id, victim_id, amount, victim_coins)?.stolen;

        // 2. Mutation wallet atomique via le service unifie (faillite
        //    cote victime + jackpot cote voleur auto-detectes).
        let description = format!("Vol entre joueurs ({} -> {})", victim_id, thief_id);
        let taunts = self
            .wallet_uc
            .transfer(
                guild_id,
                victim_id,
                thief_id,
                stolen,
                "coude_steal_success",
                &description,
            )
            .await?;

        // 3. Compteurs stats coude_players (side-effect hors wallet).
        self.repo
            .record_steal_stats(guild_id, thief_id, victim_id, stolen)
            .await?;

        // 4. Saison du Vol (cf. COUPE_AMELIORATIONS 6.3) : si le voleur
        //    est en saison Vol, on credit un bonus ex-nihilo (la victime
        //    n est pas davantage videe). Best-effort.
        let season_bonus = self.season_steal_bonus(guild_id, thief_id, stolen).await;
        if season_bonus > 0 {
            if let Some(wallet_repo) = &self.wallet_repo {
                if let Err(e) = wallet_repo
                    .credit(
                        guild_id,
                        thief_id,
                        season_bonus,
                        "season_vol_bonus",
                        "Bonus Saison du Vol",
                    )
                    .await
                {
                    tracing::warn!(error = %e, thief_id, "Echec credit bonus saison vol");
                }
            }
        }

        Ok(StealOutcome {
            stolen: stolen + season_bonus,
            taunt_events: taunts,
        })
    }

    async fn steal_fail_penalty(
        &self,
        guild_id: &str,
        thief_id: &str,
        amount: i64,
    ) -> Result<(i64, Vec<TauntEvent>), DomainError> {
        if amount <= 0 {
            return Ok((0, Vec::new()));
        }

        // Clamp au solde reel (comportement legacy record_coins_lost :
        // GREATEST(0, coins - amount), pas d'erreur si penalite > solde).
        let thief_coins = self.repo.get_coins(guild_id, thief_id).await?;
        let lost = clamp_steal_fail_penalty(amount, thief_coins);
        if lost <= 0 {
            // Pas de debit a faire ; taunts stats deja a jour. Le
            // caller affiche quand meme la penalite "faciale".
            return Ok((0, Vec::new()));
        }

        let mutation = self
            .wallet_uc
            .debit(
                guild_id,
                thief_id,
                lost,
                "coude_steal_fail_penalty",
                "Penalite vol rate",
            )
            .await?;

        self.repo
            .record_steal_fail_stats(guild_id, thief_id, lost)
            .await?;

        Ok((lost, mutation.triggered_taunts))
    }

    async fn record_casino_win(
        &self,
        guild_id: &str,
        user_id: &str,
        gain: i64,
    ) -> Result<(), DomainError> {
        if gain < 0 {
            return Err(DomainError::ValidationError(
                "Le gain ne peut pas etre negatif".into(),
            ));
        }
        // Migration #5 : credit via wallet UC (jackpot auto-detecte) +
        // stats repo. Gain = 0 : log stats uniquement (compte la main
        // dans casino_wins) pour rester coherent avec le comportement
        // legacy qui faisait un UPDATE coude_players meme a gain = 0.
        if gain > 0 {
            let _ = self
                .wallet_uc
                .credit(
                    guild_id,
                    user_id,
                    gain,
                    "coude_casino_win",
                    "Blackjack gagne",
                )
                .await?;
        }
        self.repo
            .record_casino_win_stats(guild_id, user_id, gain)
            .await
    }

    async fn record_casino_loss(
        &self,
        guild_id: &str,
        user_id: &str,
        lost: i64,
    ) -> Result<(), DomainError> {
        if lost < 0 {
            return Err(DomainError::ValidationError(
                "La perte ne peut pas etre negative".into(),
            ));
        }
        // Migration #5 : debit via wallet UC (faillite auto-detectee) +
        // stats repo. Le legacy clampait a 0 via GREATEST(0, coins -
        // lost) ; on reproduit le clamp cote service pour eviter un
        // ValidationError "solde insuffisant" si le debit demande
        // depasse le solde reel.
        if lost > 0 {
            let current = self.wallet_uc.get_balance(guild_id, user_id).await?;
            let effective = lost.min(current);
            if effective > 0 {
                let _ = self
                    .wallet_uc
                    .debit(
                        guild_id,
                        user_id,
                        effective,
                        "coude_casino_loss",
                        "Blackjack perdu",
                    )
                    .await?;
            }
        }
        self.repo
            .record_casino_loss_stats(guild_id, user_id, lost)
            .await
    }

    async fn record_casino_faillite(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<i64, DomainError> {
        // Migration #5 : lire le solde, debit integral via wallet UC
        // (faillite auto-detectee), puis enregistrer la faillite dans
        // les stats. Si le solde est deja a 0, on se contente des stats.
        let current = self.wallet_uc.get_balance(guild_id, user_id).await?;
        if current > 0 {
            let _ = self
                .wallet_uc
                .debit(
                    guild_id,
                    user_id,
                    current,
                    "coude_casino_faillite",
                    "Faillite blackjack",
                )
                .await?;
        }
        self.repo
            .record_casino_faillite_stats(guild_id, user_id, current)
            .await
    }

    async fn count_casino_today(&self, guild_id: &str, user_id: &str) -> Result<i64, DomainError> {
        self.repo.count_casino_today(guild_id, user_id).await
    }

    async fn sum_casino_gains_today(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<i64, DomainError> {
        self.repo.sum_casino_gains_today(guild_id, user_id).await
    }

    async fn count_steal_today(&self, guild_id: &str, user_id: &str) -> Result<i64, DomainError> {
        self.repo.count_steal_today(guild_id, user_id).await
    }

    async fn prank_debit(
        &self,
        guild_id: &str,
        user_id: &str,
        prank_type: &str,
    ) -> Result<crate::ports::inbound::coude::manage_economy::PrankDebitResult, DomainError> {
        use crate::ports::inbound::coude::manage_economy::PrankDebitResult;

        // Cout lu server-side (config guild, defauts historiques du bot).
        let (config_key, default) = match prank_type {
            "braquage" => ("prank_braquage_cost", 100),
            "scoop" => ("prank_scoop_cost", 200),
            "appel" => ("prank_appel_cost", 50),
            _ => {
                return Err(DomainError::ValidationError(
                    "Type de prank inconnu.".into(),
                ))
            }
        };
        let cost = self.config_i64(guild_id, config_key, default).await;

        let balance = self.wallet_uc.get_balance(guild_id, user_id).await?;
        if balance < cost {
            return Ok(PrankDebitResult::InsufficientFunds { cost, balance });
        }

        let mutation = self
            .wallet_uc
            .debit(guild_id, user_id, cost, "coude_prank", "Prank communautaire")
            .await?;
        Ok(PrankDebitResult::Debited {
            cost,
            new_balance: mutation.new_balance,
        })
    }

    async fn apply_cancel_penalty(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<crate::ports::inbound::coude::manage_economy::CancelPenaltyOutcome, DomainError> {
        use crate::ports::inbound::coude::manage_economy::CancelPenaltyOutcome;

        // Pourcentage lu server-side (stocke en entier, defaut 5%).
        let penalty_pct_int = self.config_i64(guild_id, "cancel_penalty", 5).await;
        let penalty_pct = penalty_pct_int as f64 / 100.0;

        let balance = self.wallet_uc.get_balance(guild_id, user_id).await?;
        // Mirror du calcul bot : max(1, coins * pct), clamp au solde reel pour
        // pouvoir reellement debiter (le legacy ne debitait pas -> bug corrige).
        let nominal = (balance as f64 * penalty_pct).max(1.0) as i64;
        let effective = nominal.min(balance).max(0);

        if effective > 0 {
            self.wallet_uc
                .debit(
                    guild_id,
                    user_id,
                    effective,
                    "coude_cancel_penalty",
                    "Penalite annulation combat",
                )
                .await?;
            // Compteur stats total_lost (best-effort, hors wallet) — mirror
            // exact du legacy annuler (`record_coins_lost`).
            if let Some(player_repo) = &self.player_repo {
                if let Err(e) = player_repo
                    .record_coins_lost(guild_id, user_id, effective)
                    .await
                {
                    tracing::warn!(error = %e, user_id, "Echec record total_lost penalite annulation");
                }
            }
        }

        Ok(CancelPenaltyOutcome {
            penalty: effective,
            penalty_percent: penalty_pct_int as i32,
            new_balance: balance - effective,
        })
    }

    async fn apply_refusal_penalty(
        &self,
        guild_id: &str,
        user_id: &str,
        mise: i64,
    ) -> Result<crate::ports::inbound::coude::manage_economy::CancelPenaltyOutcome, DomainError>
    {
        use crate::ports::inbound::coude::manage_economy::CancelPenaltyOutcome;

        // Pourcentage lu server-side (stocke en entier, defaut 20%).
        let penalty_pct_int = self.config_i64(guild_id, "refusal_penalty", 20).await;
        let penalty_pct = penalty_pct_int as f64 / 100.0;

        let balance = self.wallet_uc.get_balance(guild_id, user_id).await?;
        // Mirror du calcul bot : max(1, mise * pct), clamp au solde reel pour
        // pouvoir reellement debiter de facon atomique.
        let nominal = (mise.max(0) as f64 * penalty_pct).max(1.0) as i64;
        let effective = nominal.min(balance).max(0);

        if effective > 0 {
            self.wallet_uc
                .debit(
                    guild_id,
                    user_id,
                    effective,
                    "coude_refusal_penalty",
                    "Penalite refus de combat",
                )
                .await?;
            // Compteur stats total_lost (best-effort, hors wallet).
            if let Some(player_repo) = &self.player_repo {
                if let Err(e) = player_repo
                    .record_coins_lost(guild_id, user_id, effective)
                    .await
                {
                    tracing::warn!(error = %e, user_id, "Echec record total_lost penalite refus");
                }
            }
        }

        Ok(CancelPenaltyOutcome {
            penalty: effective,
            penalty_percent: penalty_pct_int as i32,
            new_balance: balance - effective,
        })
    }
}

#[cfg(test)]
#[path = "tests/manage_economy.rs"]
mod tests;
