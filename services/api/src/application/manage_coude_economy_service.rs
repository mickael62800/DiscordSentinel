use std::sync::Arc;

use async_trait::async_trait;

use crate::domain::entities::TauntEvent;
use crate::domain::errors::DomainError;
use crate::ports::inbound::manage_coude_economy::ManageCoudeEconomyUseCase;
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
    ) -> Result<i64, DomainError> {
        require_positive(amount)?;
        if thief_id == victim_id {
            return Err(DomainError::ValidationError(
                "Impossible de se voler soi-meme".into(),
            ));
        }
        let stolen = self.repo.steal(guild_id, thief_id, victim_id, amount).await?;
        if stolen <= 0 {
            return Err(DomainError::ValidationError(
                "La victime n'a pas de coins a voler".into(),
            ));
        }
        Ok(stolen)
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
        self.repo.record_casino_win(guild_id, user_id, gain).await
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
        self.repo.record_casino_loss(guild_id, user_id, lost).await
    }

    async fn record_casino_faillite(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<i64, DomainError> {
        self.repo.record_casino_faillite(guild_id, user_id).await
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

    // ── Mock CoudeEconomyRepository : steal/casino uniquement (pas utilise ici) ──
    struct MockEconomyRepo;
    #[async_trait]
    impl CoudeEconomyRepository for MockEconomyRepo {
        async fn steal(&self, _: &str, _: &str, _: &str, _: i64) -> Result<i64, DomainError> {
            unimplemented!()
        }
        async fn record_casino_win(&self, _: &str, _: &str, _: i64) -> Result<(), DomainError> {
            unimplemented!()
        }
        async fn record_casino_loss(&self, _: &str, _: &str, _: i64) -> Result<(), DomainError> {
            unimplemented!()
        }
        async fn record_casino_faillite(&self, _: &str, _: &str) -> Result<i64, DomainError> {
            unimplemented!()
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
        should_fail: bool,
    }
    #[async_trait]
    impl ManageWalletUseCase for MockWalletUc {
        async fn credit(
            &self, _: &str, _: &str, _: i64, _: &str, _: &str,
        ) -> Result<crate::ports::inbound::manage_wallet::WalletMutation, DomainError> {
            unimplemented!()
        }
        async fn debit(
            &self, _: &str, _: &str, _: i64, _: &str, _: &str,
        ) -> Result<crate::ports::inbound::manage_wallet::WalletMutation, DomainError> {
            unimplemented!()
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
        async fn get_balance(&self, _: &str, _: &str) -> Result<i64, DomainError> {
            unimplemented!()
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
    ) -> (ManageCoudeEconomyService, Arc<MockWalletUc>, Arc<MockTauntsUc>) {
        let repo = Arc::new(MockEconomyRepo);
        let wallet = Arc::new(MockWalletUc {
            returned: wallet_taunts,
            calls: Mutex::new(Vec::new()),
            should_fail: wallet_fail,
        });
        let taunts = Arc::new(MockTauntsUc {
            donor_threshold,
            donor_calls: Mutex::new(Vec::new()),
        });
        let svc = ManageCoudeEconomyService::new(repo, wallet.clone(), taunts.clone());
        (svc, wallet, taunts)
    }

    #[tokio::test]
    async fn transfer_delegates_to_wallet_uc_and_concats_donor_taunt() {
        let wallet_taunts = vec![fake_taunt(StreakKind::EcoBankruptcy, "alice")];
        let (svc, wallet, taunts) = build_service(wallet_taunts, false, 1_000);

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
        let (svc, _wallet, _taunts) = build_service(vec![], false, 10_000);
        let out = svc.transfer("g1", "alice", "bob", 500).await.unwrap();
        assert!(out.is_empty());
    }

    #[tokio::test]
    async fn transfer_rejects_self_transfer_before_calling_wallet() {
        let (svc, wallet, _) = build_service(vec![], false, 1);
        let err = svc.transfer("g1", "alice", "alice", 100).await.unwrap_err();
        assert!(matches!(err, DomainError::ValidationError(_)));
        assert!(wallet.calls.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn transfer_rejects_non_positive_amount() {
        let (svc, wallet, _) = build_service(vec![], false, 1);
        assert!(svc.transfer("g1", "a", "b", 0).await.is_err());
        assert!(svc.transfer("g1", "a", "b", -10).await.is_err());
        assert!(wallet.calls.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn transfer_propagates_wallet_error() {
        let (svc, _, _) = build_service(vec![], true, 1);
        let err = svc.transfer("g1", "alice", "bob", 100).await.unwrap_err();
        assert!(matches!(err, DomainError::ValidationError(_)));
    }
}
