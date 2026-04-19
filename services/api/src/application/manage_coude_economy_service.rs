use std::sync::Arc;

use async_trait::async_trait;

use crate::domain::entities::TauntEvent;
use crate::domain::errors::DomainError;
use crate::ports::inbound::manage_coude_economy::{
    ManageCoudeEconomyUseCase, StealOutcome,
};
use crate::ports::inbound::manage_coude_taunts::ManageCoudeTauntsUseCase;
use crate::ports::inbound::manage_wallet::ManageWalletUseCase;
use crate::ports::outbound::CoudeEconomyRepository;

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
}

impl ManageCoudeEconomyService {
    pub fn new(
        repo: Arc<dyn CoudeEconomyRepository>,
        wallet_uc: Arc<dyn ManageWalletUseCase>,
        taunts_uc: Arc<dyn ManageCoudeTauntsUseCase>,
    ) -> Self {
        Self { repo, wallet_uc, taunts_uc }
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

        Ok(StealOutcome {
            stolen,
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
mod tests {
    use super::*;
    use crate::domain::entities::{CoudeTauntsConfig, StreakKind, TauntEvent};
    use async_trait::async_trait;
    use sqlx::{Postgres, Transaction};
    use std::sync::Mutex;

    // ── Mock CoudeEconomyRepository ──
    struct MockEconomyRepo {
        coins: Mutex<std::collections::HashMap<String, i64>>,
        stats_calls: Mutex<Vec<(String, String, String, i64)>>,
        fail_stats_calls: Mutex<Vec<(String, String, i64)>>,
        casino_win_stats: Mutex<Vec<(String, String, i64)>>,
        casino_loss_stats: Mutex<Vec<(String, String, i64)>>,
        casino_faillite_stats: Mutex<Vec<(String, String, i64)>>,
    }
    impl MockEconomyRepo {
        fn new() -> Self {
            Self {
                coins: Mutex::new(std::collections::HashMap::new()),
                stats_calls: Mutex::new(Vec::new()),
                fail_stats_calls: Mutex::new(Vec::new()),
                casino_win_stats: Mutex::new(Vec::new()),
                casino_loss_stats: Mutex::new(Vec::new()),
                casino_faillite_stats: Mutex::new(Vec::new()),
            }
        }
        fn set_coins(&self, guild_id: &str, user_id: &str, coins: i64) {
            self.coins
                .lock()
                .unwrap()
                .insert(format!("{}:{}", guild_id, user_id), coins);
        }
    }
    #[async_trait]
    impl CoudeEconomyRepository for MockEconomyRepo {
        async fn record_steal_stats(
            &self,
            g: &str,
            thief: &str,
            victim: &str,
            amount: i64,
        ) -> Result<(), DomainError> {
            self.stats_calls
                .lock()
                .unwrap()
                .push((g.into(), thief.into(), victim.into(), amount));
            Ok(())
        }
        async fn record_steal_fail_stats(
            &self,
            g: &str,
            thief: &str,
            amount: i64,
        ) -> Result<(), DomainError> {
            self.fail_stats_calls
                .lock()
                .unwrap()
                .push((g.into(), thief.into(), amount));
            Ok(())
        }
        async fn get_coins(&self, g: &str, u: &str) -> Result<i64, DomainError> {
            self.coins
                .lock()
                .unwrap()
                .get(&format!("{}:{}", g, u))
                .copied()
                .ok_or_else(|| DomainError::NotFound("Wallet introuvable".into()))
        }
        async fn record_casino_win_stats(
            &self,
            g: &str,
            u: &str,
            gain: i64,
        ) -> Result<(), DomainError> {
            self.casino_win_stats
                .lock()
                .unwrap()
                .push((g.into(), u.into(), gain));
            Ok(())
        }
        async fn record_casino_loss_stats(
            &self,
            g: &str,
            u: &str,
            lost: i64,
        ) -> Result<(), DomainError> {
            self.casino_loss_stats
                .lock()
                .unwrap()
                .push((g.into(), u.into(), lost));
            Ok(())
        }
        async fn record_casino_faillite_stats(
            &self,
            g: &str,
            u: &str,
            cleared: i64,
        ) -> Result<i64, DomainError> {
            self.casino_faillite_stats
                .lock()
                .unwrap()
                .push((g.into(), u.into(), cleared));
            Ok(cleared)
        }
        async fn count_casino_today(&self, _: &str, _: &str) -> Result<i64, DomainError> {
            unimplemented!()
        }
        async fn sum_casino_gains_today(&self, _: &str, _: &str) -> Result<i64, DomainError> {
            unimplemented!()
        }
        async fn count_steal_today(&self, _: &str, _: &str) -> Result<i64, DomainError> {
            unimplemented!()
        }
    }

    fn fake_taunt(kind: StreakKind, user: &str) -> TauntEvent {
        TauntEvent {
            channel_id: "chan".into(),
            target_user_id: user.into(),
            message: format!("taunt {}", kind.as_str()),
            nickname_suffix: String::new(),
            streak_kind: kind.as_str(),
            streak_value: 1,
        }
    }

    // ── Mock WalletUC : renvoie la liste de taunts passee au constructeur ──
    struct MockWalletUc {
        returned: Vec<TauntEvent>,
        calls: Mutex<Vec<(String, String, String, i64, String)>>,
        debit_calls: Mutex<Vec<(String, String, i64, String)>>,
        debit_returned: Vec<TauntEvent>,
        credit_calls: Mutex<Vec<(String, String, i64, String)>>,
        credit_returned: Vec<TauntEvent>,
        balances: Mutex<std::collections::HashMap<String, i64>>,
        should_fail: bool,
    }
    impl MockWalletUc {
        fn set_balance(&self, guild_id: &str, user_id: &str, coins: i64) {
            self.balances
                .lock()
                .unwrap()
                .insert(format!("{}:{}", guild_id, user_id), coins);
        }
    }
    #[async_trait]
    impl ManageWalletUseCase for MockWalletUc {
        async fn credit(
            &self,
            guild_id: &str,
            user: &str,
            amount: i64,
            source: &str,
            _desc: &str,
        ) -> Result<crate::ports::inbound::manage_wallet::WalletMutation, DomainError> {
            if self.should_fail {
                return Err(DomainError::ValidationError("wallet fail".into()));
            }
            self.credit_calls.lock().unwrap().push((
                guild_id.into(), user.into(), amount, source.into(),
            ));
            let key = format!("{}:{}", guild_id, user);
            let mut map = self.balances.lock().unwrap();
            let prev = *map.get(&key).unwrap_or(&0);
            let new_balance = prev + amount;
            map.insert(key, new_balance);
            Ok(crate::ports::inbound::manage_wallet::WalletMutation {
                new_balance,
                previous_balance: prev,
                triggered_taunts: self.credit_returned.clone(),
            })
        }
        async fn debit(
            &self,
            guild_id: &str,
            user: &str,
            amount: i64,
            source: &str,
            _desc: &str,
        ) -> Result<crate::ports::inbound::manage_wallet::WalletMutation, DomainError> {
            if self.should_fail {
                return Err(DomainError::ValidationError("Solde insuffisant".into()));
            }
            self.debit_calls.lock().unwrap().push((
                guild_id.into(), user.into(), amount, source.into(),
            ));
            Ok(crate::ports::inbound::manage_wallet::WalletMutation {
                new_balance: 0,
                previous_balance: amount,
                triggered_taunts: self.debit_returned.clone(),
            })
        }
        async fn transfer(
            &self,
            guild_id: &str,
            from: &str,
            to: &str,
            amount: i64,
            source: &str,
            _desc: &str,
        ) -> Result<Vec<TauntEvent>, DomainError> {
            if self.should_fail {
                return Err(DomainError::ValidationError("Solde insuffisant".into()));
            }
            self.calls.lock().unwrap().push((
                guild_id.into(), from.into(), to.into(), amount, source.into(),
            ));
            Ok(self.returned.clone())
        }
        async fn get_balance(&self, guild_id: &str, user_id: &str) -> Result<i64, DomainError> {
            Ok(*self
                .balances
                .lock()
                .unwrap()
                .get(&format!("{}:{}", guild_id, user_id))
                .unwrap_or(&0))
        }
        async fn credit_tx(
            &self, _: &mut Transaction<'_, Postgres>, _: &str, _: &str, _: i64, _: &str, _: &str,
        ) -> Result<crate::ports::inbound::manage_wallet::TxWalletMutation, DomainError> {
            unimplemented!()
        }
        async fn debit_tx(
            &self, _: &mut Transaction<'_, Postgres>, _: &str, _: &str, _: i64, _: &str, _: &str,
        ) -> Result<crate::ports::inbound::manage_wallet::TxWalletMutation, DomainError> {
            unimplemented!()
        }
        async fn post_commit_taunts(
            &self,
            _: &str,
            _: &str,
            _: &crate::ports::inbound::manage_wallet::TxWalletMutation,
        ) -> Vec<TauntEvent> {
            vec![]
        }
    }

    // ── Mock TauntsUC : renvoie un donor taunt si amount >= threshold ──
    struct MockTauntsUc {
        donor_threshold: i64,
        donor_calls: Mutex<Vec<(String, String, i64)>>,
    }
    #[async_trait]
    impl ManageCoudeTauntsUseCase for MockTauntsUc {
        async fn on_player_won(&self, _: &str, _: &str) -> Result<Option<TauntEvent>, DomainError> { Ok(None) }
        async fn on_player_lost(&self, _: &str, _: &str) -> Result<Option<TauntEvent>, DomainError> { Ok(None) }
        async fn on_player_drew(&self, _: &str, _: &str) -> Result<(), DomainError> { Ok(()) }
        async fn on_player_stolen_from(&self, _: &str, _: &str) -> Result<Option<TauntEvent>, DomainError> { Ok(None) }
        async fn on_player_defended_steal(&self, _: &str, _: &str) -> Result<(), DomainError> { Ok(()) }
        async fn on_bj_natural(&self, _: &str, _: &str) -> Result<Option<TauntEvent>, DomainError> { Ok(None) }
        async fn on_bj_hand_won(&self, _: &str, _: &str) -> Result<Option<TauntEvent>, DomainError> { Ok(None) }
        async fn on_bj_hand_bust(&self, _: &str, _: &str) -> Result<Option<TauntEvent>, DomainError> { Ok(None) }
        async fn on_bankruptcy(&self, _: &str, _: &str) -> Result<Option<TauntEvent>, DomainError> { Ok(None) }
        async fn on_jackpot(&self, _: &str, _: &str, _: i64) -> Result<Option<TauntEvent>, DomainError> { Ok(None) }
        async fn on_generous_donor(
            &self,
            guild_id: &str,
            user_id: &str,
            amount: i64,
        ) -> Result<Option<TauntEvent>, DomainError> {
            self.donor_calls.lock().unwrap().push((guild_id.into(), user_id.into(), amount));
            if amount >= self.donor_threshold {
                Ok(Some(fake_taunt(StreakKind::EcoGenerousDonor, user_id)))
            } else {
                Ok(None)
            }
        }
        async fn get_config(&self, _: &str) -> Result<CoudeTauntsConfig, DomainError> {
            Ok(CoudeTauntsConfig { guild_id: "g".into(), channel_id: None, enabled: false })
        }
        async fn set_channel(&self, _: &str, _: Option<&str>) -> Result<(), DomainError> { Ok(()) }
        async fn set_enabled(&self, _: &str, _: bool) -> Result<(), DomainError> { Ok(()) }
        async fn set_opt_out(&self, _: &str, _: &str, _: bool) -> Result<(), DomainError> { Ok(()) }
        async fn is_opted_out(&self, _: &str, _: &str) -> Result<bool, DomainError> { Ok(false) }
        async fn list_opt_outs(&self, _: &str) -> Result<Vec<String>, DomainError> { Ok(vec![]) }
    }

    fn build_service(
        wallet_taunts: Vec<TauntEvent>,
        wallet_fail: bool,
        donor_threshold: i64,
    ) -> (
        ManageCoudeEconomyService,
        Arc<MockEconomyRepo>,
        Arc<MockWalletUc>,
        Arc<MockTauntsUc>,
    ) {
        build_service_with_debit_taunts(wallet_taunts, vec![], wallet_fail, donor_threshold)
    }

    fn build_service_with_debit_taunts(
        wallet_taunts: Vec<TauntEvent>,
        debit_taunts: Vec<TauntEvent>,
        wallet_fail: bool,
        donor_threshold: i64,
    ) -> (
        ManageCoudeEconomyService,
        Arc<MockEconomyRepo>,
        Arc<MockWalletUc>,
        Arc<MockTauntsUc>,
    ) {
        let repo = Arc::new(MockEconomyRepo::new());
        let wallet = Arc::new(MockWalletUc {
            returned: wallet_taunts,
            calls: Mutex::new(Vec::new()),
            debit_calls: Mutex::new(Vec::new()),
            debit_returned: debit_taunts,
            credit_calls: Mutex::new(Vec::new()),
            credit_returned: Vec::new(),
            balances: Mutex::new(std::collections::HashMap::new()),
            should_fail: wallet_fail,
        });
        let taunts = Arc::new(MockTauntsUc {
            donor_threshold,
            donor_calls: Mutex::new(Vec::new()),
        });
        let svc = ManageCoudeEconomyService::new(repo.clone(), wallet.clone(), taunts.clone());
        (svc, repo, wallet, taunts)
    }

    #[tokio::test]
    async fn transfer_delegates_to_wallet_uc_and_concats_donor_taunt() {
        let wallet_taunts = vec![fake_taunt(StreakKind::EcoBankruptcy, "alice")];
        let (svc, _repo, wallet, taunts) = build_service(wallet_taunts, false, 1_000);

        let out = svc.transfer("g1", "alice", "bob", 5_000).await.unwrap();

        // Wallet a ete appele une fois avec les bons args.
        let calls = wallet.calls.lock().unwrap().clone();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "g1");
        assert_eq!(calls[0].1, "alice");
        assert_eq!(calls[0].2, "bob");
        assert_eq!(calls[0].3, 5_000);
        assert_eq!(calls[0].4, "coude_donner");

        // Taunts : faillite (wallet) + donor (car 5000 >= 1000).
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].streak_kind, StreakKind::EcoBankruptcy.as_str());
        assert_eq!(out[1].streak_kind, StreakKind::EcoGenerousDonor.as_str());

        // donor appele avec amount brut.
        let donor_calls = taunts.donor_calls.lock().unwrap().clone();
        assert_eq!(donor_calls.len(), 1);
        assert_eq!(donor_calls[0].2, 5_000);
    }

    #[tokio::test]
    async fn transfer_below_donor_threshold_does_not_trigger_donor_taunt() {
        let (svc, _repo, _wallet, _taunts) = build_service(vec![], false, 10_000);
        let out = svc.transfer("g1", "alice", "bob", 500).await.unwrap();
        assert!(out.is_empty());
    }

    #[tokio::test]
    async fn transfer_rejects_self_transfer_before_calling_wallet() {
        let (svc, _repo, wallet, _) = build_service(vec![], false, 1);
        let err = svc.transfer("g1", "alice", "alice", 100).await.unwrap_err();
        assert!(matches!(err, DomainError::ValidationError(_)));
        assert!(wallet.calls.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn transfer_rejects_non_positive_amount() {
        let (svc, _repo, wallet, _) = build_service(vec![], false, 1);
        assert!(svc.transfer("g1", "a", "b", 0).await.is_err());
        assert!(svc.transfer("g1", "a", "b", -10).await.is_err());
        assert!(wallet.calls.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn transfer_propagates_wallet_error() {
        let (svc, _repo, _, _) = build_service(vec![], true, 1);
        let err = svc.transfer("g1", "alice", "bob", 100).await.unwrap_err();
        assert!(matches!(err, DomainError::ValidationError(_)));
    }

    // ── Steal tests (migration wallet unifie) ──

    #[tokio::test]
    async fn steal_success_delegates_to_wallet_transfer() {
        // Victime a 5000 coins, voleur tente 1000. Wallet renverra un
        // taunt jackpot (simule).
        let wallet_taunts = vec![fake_taunt(StreakKind::EcoJackpot, "thief")];
        let (svc, repo, wallet, _taunts) = build_service(wallet_taunts, false, 9_999_999);
        repo.set_coins("g1", "victim", 5000);

        let outcome = svc.steal("g1", "thief", "victim", 1000).await.unwrap();

        assert_eq!(outcome.stolen, 1000);
        assert_eq!(outcome.taunt_events.len(), 1);
        assert_eq!(outcome.taunt_events[0].streak_kind, StreakKind::EcoJackpot.as_str());

        // Wallet.transfer appele avec (victim -> thief, 1000, coude_steal_success).
        let calls = wallet.calls.lock().unwrap().clone();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "g1");
        assert_eq!(calls[0].1, "victim");
        assert_eq!(calls[0].2, "thief");
        assert_eq!(calls[0].3, 1000);
        assert_eq!(calls[0].4, "coude_steal_success");

        // Stats counters appeles avec (thief, victim, 1000).
        let stats = repo.stats_calls.lock().unwrap().clone();
        assert_eq!(stats.len(), 1);
        assert_eq!(stats[0].1, "thief");
        assert_eq!(stats[0].2, "victim");
        assert_eq!(stats[0].3, 1000);
    }

    #[tokio::test]
    async fn steal_clamps_to_victim_balance() {
        // Voleur demande 5000 mais victime n'en a que 800 : stolen = 800.
        let (svc, repo, wallet, _) = build_service(vec![], false, 9_999_999);
        repo.set_coins("g1", "victim", 800);

        let outcome = svc.steal("g1", "thief", "victim", 5000).await.unwrap();
        assert_eq!(outcome.stolen, 800);

        let calls = wallet.calls.lock().unwrap().clone();
        assert_eq!(calls[0].3, 800);
    }

    #[tokio::test]
    async fn steal_rejects_when_victim_has_nothing() {
        let (svc, repo, wallet, _) = build_service(vec![], false, 1);
        repo.set_coins("g1", "victim", 0);
        let err = svc.steal("g1", "thief", "victim", 100).await.unwrap_err();
        assert!(matches!(err, DomainError::ValidationError(_)));
        assert!(wallet.calls.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn steal_rejects_self_steal() {
        let (svc, _repo, wallet, _) = build_service(vec![], false, 1);
        let err = svc.steal("g1", "alice", "alice", 100).await.unwrap_err();
        assert!(matches!(err, DomainError::ValidationError(_)));
        assert!(wallet.calls.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn steal_rejects_non_positive_amount() {
        let (svc, repo, wallet, _) = build_service(vec![], false, 1);
        repo.set_coins("g1", "victim", 1000);
        assert!(svc.steal("g1", "thief", "victim", 0).await.is_err());
        assert!(svc.steal("g1", "thief", "victim", -10).await.is_err());
        assert!(wallet.calls.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn steal_fail_penalty_delegates_to_wallet_debit() {
        // Voleur a 2000 coins, penalite 500 : debit 500, faillite non
        // declenchee par le mock.
        let (svc, repo, wallet, _) =
            build_service_with_debit_taunts(vec![], vec![], false, 1);
        repo.set_coins("g1", "thief", 2000);

        let (lost, taunts) = svc.steal_fail_penalty("g1", "thief", 500).await.unwrap();

        assert_eq!(lost, 500);
        assert!(taunts.is_empty());

        let debit_calls = wallet.debit_calls.lock().unwrap().clone();
        assert_eq!(debit_calls.len(), 1);
        assert_eq!(debit_calls[0].0, "g1");
        assert_eq!(debit_calls[0].1, "thief");
        assert_eq!(debit_calls[0].2, 500);
        assert_eq!(debit_calls[0].3, "coude_steal_fail_penalty");

        // Fail stats counter (thief, 500).
        let stats = repo.fail_stats_calls.lock().unwrap().clone();
        assert_eq!(stats.len(), 1);
        assert_eq!(stats[0].1, "thief");
        assert_eq!(stats[0].2, 500);
    }

    #[tokio::test]
    async fn steal_fail_penalty_clamps_to_thief_balance() {
        // Voleur a 300 mais penalite demandee = 1000 : debite seulement 300.
        let (svc, repo, wallet, _) = build_service(vec![], false, 1);
        repo.set_coins("g1", "thief", 300);

        let (lost, _taunts) = svc.steal_fail_penalty("g1", "thief", 1000).await.unwrap();
        assert_eq!(lost, 300);

        let debit_calls = wallet.debit_calls.lock().unwrap().clone();
        assert_eq!(debit_calls[0].2, 300);
    }

    #[tokio::test]
    async fn steal_fail_penalty_noop_when_thief_has_nothing() {
        // Voleur a 0 coin : pas de debit, pas d'erreur (comportement
        // legacy record_coins_lost).
        let (svc, repo, wallet, _) = build_service(vec![], false, 1);
        repo.set_coins("g1", "thief", 0);

        let (lost, taunts) = svc.steal_fail_penalty("g1", "thief", 500).await.unwrap();
        assert_eq!(lost, 0);
        assert!(taunts.is_empty());
        assert!(wallet.debit_calls.lock().unwrap().is_empty());
        assert!(repo.fail_stats_calls.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn steal_fail_penalty_propagates_bankruptcy_taunt() {
        // Wallet debit declenche une faillite cote voleur.
        let (svc, repo, _wallet, _) = build_service_with_debit_taunts(
            vec![],
            vec![fake_taunt(StreakKind::EcoBankruptcy, "thief")],
            false,
            1,
        );
        repo.set_coins("g1", "thief", 1000);

        let (lost, taunts) = svc.steal_fail_penalty("g1", "thief", 1000).await.unwrap();
        assert_eq!(lost, 1000);
        assert_eq!(taunts.len(), 1);
        assert_eq!(taunts[0].streak_kind, StreakKind::EcoBankruptcy.as_str());
    }

    // ── Casino tests (migration #5) ──

    #[tokio::test]
    async fn casino_win_delegates_to_wallet_credit() {
        // Gain > 0 : credit via wallet_uc + stats repo.
        let (svc, repo, wallet, _) = build_service(vec![], false, 1);
        svc.record_casino_win("g1", "alice", 1500).await.unwrap();

        // wallet.credit appele une fois avec les bons args.
        let credit_calls = wallet.credit_calls.lock().unwrap().clone();
        assert_eq!(credit_calls.len(), 1);
        assert_eq!(credit_calls[0].0, "g1");
        assert_eq!(credit_calls[0].1, "alice");
        assert_eq!(credit_calls[0].2, 1500);
        assert_eq!(credit_calls[0].3, "coude_casino_win");

        // Stats repo appele avec (g1, alice, 1500).
        let stats = repo.casino_win_stats.lock().unwrap().clone();
        assert_eq!(stats.len(), 1);
        assert_eq!(stats[0], ("g1".into(), "alice".into(), 1500));
    }

    #[tokio::test]
    async fn casino_loss_delegates_to_wallet_debit_clamped_to_balance() {
        // Solde 800, lost demande 1500 : debit clamp a 800, stats
        // restent sur 1500 (legacy conservait la perte "faciale").
        let (svc, repo, wallet, _) = build_service(vec![], false, 1);
        wallet.set_balance("g1", "alice", 800);

        svc.record_casino_loss("g1", "alice", 1500).await.unwrap();

        let debit_calls = wallet.debit_calls.lock().unwrap().clone();
        assert_eq!(debit_calls.len(), 1);
        assert_eq!(debit_calls[0].2, 800);
        assert_eq!(debit_calls[0].3, "coude_casino_loss");

        let stats = repo.casino_loss_stats.lock().unwrap().clone();
        assert_eq!(stats.len(), 1);
        assert_eq!(stats[0], ("g1".into(), "alice".into(), 1500));
    }

    #[tokio::test]
    async fn casino_faillite_debits_full_balance_and_records_stats() {
        // Solde 2000 : faillite debite les 2000, stats = 2000.
        let (svc, repo, wallet, _) = build_service(vec![], false, 1);
        wallet.set_balance("g1", "alice", 2000);

        let total_lost = svc.record_casino_faillite("g1", "alice").await.unwrap();
        assert_eq!(total_lost, 2000);

        let debit_calls = wallet.debit_calls.lock().unwrap().clone();
        assert_eq!(debit_calls.len(), 1);
        assert_eq!(debit_calls[0].2, 2000);
        assert_eq!(debit_calls[0].3, "coude_casino_faillite");

        let stats = repo.casino_faillite_stats.lock().unwrap().clone();
        assert_eq!(stats.len(), 1);
        assert_eq!(stats[0], ("g1".into(), "alice".into(), 2000));
    }

    #[tokio::test]
    async fn casino_faillite_on_empty_wallet_only_records_stats() {
        // Solde 0 : pas de debit, stats quand meme enregistres avec
        // cleared = 0 (incremente casino_losses).
        let (svc, repo, wallet, _) = build_service(vec![], false, 1);
        wallet.set_balance("g1", "alice", 0);

        let total_lost = svc.record_casino_faillite("g1", "alice").await.unwrap();
        assert_eq!(total_lost, 0);

        assert!(wallet.debit_calls.lock().unwrap().is_empty());
        let stats = repo.casino_faillite_stats.lock().unwrap().clone();
        assert_eq!(stats.len(), 1);
        assert_eq!(stats[0].2, 0);
    }
}
