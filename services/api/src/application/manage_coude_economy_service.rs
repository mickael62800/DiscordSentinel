use std::sync::Arc;

use async_trait::async_trait;

use crate::domain::entities::{CurseKind, TauntEvent, LEAKY_WALLET_FEE_COINS};
use crate::domain::errors::DomainError;
use crate::ports::inbound::manage_coude_economy::{
    ManageCoudeEconomyUseCase, StealOutcome,
};
use crate::ports::inbound::manage_coude_taunts::ManageCoudeTauntsUseCase;
use crate::ports::inbound::manage_wallet::ManageWalletUseCase;
use crate::ports::outbound::{
    CoudeCursesRepository, CoudeEconomyRepository, CoudePlayerRepository, WalletRepository,
};

/// Service "economie Coup de Coude".
///
/// # Migration wallet unifie (PoC `/donner`)
///
/// La methode `transfer` ne passe plus par `CoudeEconomyRepository` : elle
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
    repo: Arc<dyn CoudeEconomyRepository>,
    wallet_uc: Arc<dyn ManageWalletUseCase>,
    taunts_uc: Arc<dyn ManageCoudeTauntsUseCase>,
    wallet_repo: Option<Arc<dyn WalletRepository>>,
    curses_repo: Option<Arc<dyn CoudeCursesRepository>>,
    player_repo: Option<Arc<dyn CoudePlayerRepository>>,
}

impl ManageCoudeEconomyService {
    pub fn new(
        repo: Arc<dyn CoudeEconomyRepository>,
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
        }
    }

    /// Branche les repos necessaires au LeakyWallet (cf. COUPE_AMELIORATIONS
    /// 5.1). Si l emetteur d un /donner est sous l effet, 10c sont preleves
    /// en plus en frais (le destinataire recoit `amount` complet).
    pub fn with_leaky_wallet_support(
        mut self,
        wallet_repo: Arc<dyn WalletRepository>,
        curses_repo: Arc<dyn CoudeCursesRepository>,
    ) -> Self {
        self.wallet_repo = Some(wallet_repo);
        self.curses_repo = Some(curses_repo);
        self
    }

    /// Branche le repo player pour appliquer le multiplicateur de
    /// "Saison du Vol" (cf. COUPE_AMELIORATIONS 6.3) : gains x1.5
    /// pour le voleur, paye depuis le neant (server-paid bonus).
    pub fn with_player_repo(mut self, player_repo: Arc<dyn CoudePlayerRepository>) -> Self {
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
        let Some(repo) = &self.player_repo else { return 0; };
        let Ok(Some(player)) = repo.get(guild_id, thief_id).await else { return 0; };
        use crate::domain::entities::theme_for_season;
        let mult = theme_for_season(player.season).steal_gain_multiplier;
        if mult <= 1.0 || stolen <= 0 {
            return 0;
        }
        ((stolen as f64) * (mult - 1.0)) as i64
    }
}

fn require_positive(amount: i64) -> Result<(), DomainError> {
    if amount <= 0 {
        Err(DomainError::ValidationError(
            "Le montant doit etre positif".into(),
        ))
    } else {
        Ok(())
    }
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
            .transfer(guild_id, from_id, to_id, amount, "coude_donner", &description)
            .await?;

        // 1.bis Branchement Leaky Wallet (cf. COUPE_AMELIORATIONS 5.1) :
        //       si l emetteur est maudit, 10c supplementaires sont preleves
        //       en frais (best-effort hors-tx — si le debit echoue, le don
        //       est deja passe : on log et on continue).
        if self.has_leaky_wallet(guild_id, from_id).await {
            if let Some(wallet_repo) = &self.wallet_repo {
                if let Err(e) = wallet_repo
                    .debit(
                        guild_id,
                        from_id,
                        LEAKY_WALLET_FEE_COINS,
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

        // 2. Don genereux : taunt specifique cote emetteur si amount >=
        //    threshold configure (logique interne au service taunts). Ne
        //    peut pas etre detecte par le wallet service car c'est
        //    domaine-specifique Coup de Coude.
        if let Ok(Some(ev)) = self.taunts_uc.on_generous_donor(guild_id, from_id, amount).await {
            taunts.push(ev);
        }

        Ok(taunts)
    }

    async fn steal(
        &self,
        guild_id: &str,
        thief_id: &str,
        victim_id: &str,
        amount: i64,
    ) -> Result<StealOutcome, DomainError> {
        require_positive(amount)?;
        if thief_id == victim_id {
            return Err(DomainError::ValidationError(
                "Impossible de se voler soi-meme".into(),
            ));
        }

        // 1. Lire le solde victime + clamp : on ne peut pas voler plus
        //    que ce qu'elle possede (pas de creation de coins). On lit
        //    hors-tx : si un autre evenement modifie le solde entre le
        //    read et le transfer, wallet_uc.transfer echouera
        //    proprement (ValidationError "Solde insuffisant").
        let victim_coins = self.repo.get_coins(guild_id, victim_id).await?;
        let stolen = amount.min(victim_coins);
        if stolen <= 0 {
            return Err(DomainError::ValidationError(
                "La victime n'a pas de coins a voler".into(),
            ));
        }

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
        let lost = amount.min(thief_coins);
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
                .credit(guild_id, user_id, gain, "coude_casino_win", "Blackjack gagne")
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

    async fn count_casino_today(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<i64, DomainError> {
        self.repo.count_casino_today(guild_id, user_id).await
    }

    async fn sum_casino_gains_today(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<i64, DomainError> {
        self.repo.sum_casino_gains_today(guild_id, user_id).await
    }

    async fn count_steal_today(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<i64, DomainError> {
        self.repo.count_steal_today(guild_id, user_id).await
    }
}


#[cfg(test)]
#[path = "tests/manage_coude_economy.rs"]
mod tests;
