//! Tests d'integration end-to-end pour les 6 services Coude gRPC.
//!
//! Demarre un vrai serveur tonic in-process sur 127.0.0.1:0 (port libre),
//! pointe un client tonic dessus, et fait des appels reels. Les use cases
//! sont mockes (n'utilisent pas de DB) — on valide la chaine complete :
//! serialisation proto -> reseau -> handler -> conversion DTO ->
//! reponse proto -> deserialisation client.
//!
//! Couvre :
//! - CoudePlayerService (Phase 7A)        — get_or_create, get, add_xp
//! - CoudeCombatsService (Phase 7A.opt F.1) — list, get, create
//! - CoudeBetsService (F.1)                 — list_for_combat
//! - CoudeEconomyService (F.1)              — transfer, count_casino_today
//! - CoudeInventoryService (F.1)            — list_inventory, has_item
//! - CoudeSocialService (F.1)               — leaderboard, current_season

#[path = "../../test_helpers.rs"]
mod test_helpers;

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, TimeZone, Utc};
use tokio::sync::oneshot;
use tonic::transport::{Endpoint, Server};
use uuid::Uuid;

use sentinel_api::adapters::inbound::grpc::coude::{
    CoudeBetsGrpc, CoudeCombatsGrpc, CoudeEconomyGrpc, CoudeInventoryGrpc, CoudePlayerGrpc,
    CoudeSocialGrpc,
};
use sentinel_api::domain::entities::{
    CombatResolution, CombatStat, CoudeBet, CoudeCombat, CoudeCurrentSeason, CoudeEvent,
    CoudeInsurance, CoudeInventoryItem, CoudeLeaderboardEntry, CoudePlayer, CoudePrime,
    LeaderboardCategory, NewCoudeBet, NewCoudeCombat, NewCoudePrime, NewDailyChaos, RefundSummary,
    XpProgress,
};
use sentinel_api::domain::errors::DomainError;
use sentinel_api::ports::inbound::{
    ManageCoudeBetsUseCase, ManageCoudeCombatsUseCase, ManageCoudeEconomyUseCase,
    ManageCoudeInventoryUseCase, ManageCoudeSocialUseCase,
};
use sentinel_api::ports::inbound::manage_coude_players::ManageCoudePlayersUseCase;
use sentinel_proto::coude::v1 as proto;
use sentinel_proto::coude::v1::coude_bets_service_client::CoudeBetsServiceClient;
use sentinel_proto::coude::v1::coude_bets_service_server::CoudeBetsServiceServer;
use sentinel_proto::coude::v1::coude_combats_service_client::CoudeCombatsServiceClient;
use sentinel_proto::coude::v1::coude_combats_service_server::CoudeCombatsServiceServer;
use sentinel_proto::coude::v1::coude_economy_service_client::CoudeEconomyServiceClient;
use sentinel_proto::coude::v1::coude_economy_service_server::CoudeEconomyServiceServer;
use sentinel_proto::coude::v1::coude_inventory_service_client::CoudeInventoryServiceClient;
use sentinel_proto::coude::v1::coude_inventory_service_server::CoudeInventoryServiceServer;
use sentinel_proto::coude::v1::coude_player_service_client::CoudePlayerServiceClient;
use sentinel_proto::coude::v1::coude_player_service_server::CoudePlayerServiceServer;
use sentinel_proto::coude::v1::coude_social_service_client::CoudeSocialServiceClient;
use sentinel_proto::coude::v1::coude_social_service_server::CoudeSocialServiceServer;

// ── Mock du use case : n'implemente que ce qui est appele dans les tests ──

struct MockPlayersUc;

#[async_trait]
impl ManageCoudePlayersUseCase for MockPlayersUc {
    async fn get_or_create(
        &self,
        guild_id: String,
        user_id: String,
        username: String,
    ) -> Result<CoudePlayer, DomainError> {
        Ok(CoudePlayer {
            guild_id,
            user_id,
            username,
            coins: 100,
            total_wins: 0, total_losses: 0, total_draws: 0,
            total_earned: 0, total_lost: 0, total_stolen: 0,
            cowardice_count: 0, chaos_events: 0,
            casino_wins: 0, casino_losses: 0,
            level: 1, xp: 0, stat_points: 0, atk: 0, def: 0,
            class: None, title: None, class_changed_at: None,
            hp_current: 100, hp_max: 100,
            hp_last_regen: None, repos_last_used: None,
            season: 1,
            created_at: Utc.with_ymd_and_hms(2026, 4, 11, 12, 0, 0).unwrap(),
            updated_at: Utc.with_ymd_and_hms(2026, 4, 11, 12, 0, 0).unwrap(),
        })
    }

    async fn get(&self, guild_id: &str, user_id: &str) -> Result<CoudePlayer, DomainError> {
        if user_id == "missing" {
            return Err(DomainError::NotFound("joueur introuvable".into()));
        }
        Ok(CoudePlayer {
            guild_id: guild_id.into(),
            user_id: user_id.into(),
            username: "Existing".into(),
            coins: 999,
            total_wins: 5, total_losses: 2, total_draws: 0,
            total_earned: 1500, total_lost: 200, total_stolen: 0,
            cowardice_count: 0, chaos_events: 0,
            casino_wins: 0, casino_losses: 0,
            level: 7, xp: 2000, stat_points: 1, atk: 4, def: 3,
            class: None, title: None, class_changed_at: None,
            hp_current: 80, hp_max: 100,
            hp_last_regen: None, repos_last_used: None,
            season: 1,
            created_at: Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
            updated_at: Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        })
    }

    async fn add_xp(
        &self,
        _guild_id: &str,
        _user_id: &str,
        amount: i64,
    ) -> Result<XpProgress, DomainError> {
        Ok(XpProgress {
            new_xp: amount,
            new_level: 2,
            leveled_up: amount >= 100,
            stat_points_gained: if amount >= 100 { 1 } else { 0 },
        })
    }

    // Methodes non utilisees dans ces tests — panic explicite si appelees.
    async fn list(&self, _: &str) -> Result<Vec<CoudePlayer>, DomainError> { unimplemented!() }
    async fn random_active(&self, _: &str, _: i64) -> Result<Vec<CoudePlayer>, DomainError> { unimplemented!() }
    async fn list_guild_ids(&self) -> Result<Vec<String>, DomainError> { unimplemented!() }
    async fn update_class(&self, _: &str, _: &str, _: &str) -> Result<(), DomainError> { unimplemented!() }
    async fn spend_stat_point(&self, _: &str, _: &str, _: CombatStat) -> Result<CoudePlayer, DomainError> { unimplemented!() }
    async fn reset_stats(&self, _: &str, _: &str, _: i64) -> Result<CoudePlayer, DomainError> { unimplemented!() }
    async fn record_win(&self, _: &str, _: &str, _: i64, _: i64) -> Result<(), DomainError> { unimplemented!() }
    async fn record_loss(&self, _: &str, _: &str, _: i64) -> Result<(), DomainError> { unimplemented!() }
    async fn record_draw(&self, _: &str, _: &str, _: i64) -> Result<(), DomainError> { unimplemented!() }
    async fn increment_cowardice(&self, _: &str, _: &str) -> Result<i32, DomainError> { unimplemented!() }
    async fn increment_chaos(&self, _: &str, _: &str) -> Result<(), DomainError> { unimplemented!() }
    async fn record_coins_earned(&self, _: &str, _: &str, _: i64) -> Result<(), DomainError> { unimplemented!() }
    async fn record_coins_lost(&self, _: &str, _: &str, _: i64) -> Result<(), DomainError> { unimplemented!() }
    async fn update_hp(&self, _: &str, _: &str, _: i32, _: i32) -> Result<(), DomainError> { unimplemented!() }
    async fn full_heal(&self, _: &str, _: &str) -> Result<(), DomainError> { unimplemented!() }
    async fn regen_hp_tick(&self, _: f64, _: f64, _: f64, _: f64) -> Result<u64, DomainError> { Ok(0) }
}

// ── Helper : demarre un serveur in-process et retourne (url, shutdown_tx) ──

async fn start_server() -> (String, oneshot::Sender<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let url = format!("http://{addr}");

    let (tx, rx) = oneshot::channel::<()>();
    let svc = CoudePlayerGrpc {
        players_uc: Arc::new(MockPlayersUc),
        wallet_uc: Arc::new(test_helpers::StubWalletUc),
    };
    let server = CoudePlayerServiceServer::new(svc);

    tokio::spawn(async move {
        Server::builder()
            .add_service(server)
            .serve_with_incoming_shutdown(
                tokio_stream::wrappers::TcpListenerStream::new(listener),
                async {
                    let _ = rx.await;
                },
            )
            .await
            .unwrap();
    });

    // Petit yield pour laisser le serveur prendre l'accept loop.
    tokio::time::sleep(Duration::from_millis(50)).await;
    (url, tx)
}

async fn connect(url: &str) -> CoudePlayerServiceClient<tonic::transport::Channel> {
    let endpoint = Endpoint::from_shared(url.to_string()).unwrap();
    CoudePlayerServiceClient::connect(endpoint).await.unwrap()
}

// ══════════════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn get_or_create_player_round_trip() {
    let (url, shutdown) = start_server().await;
    let mut client = connect(&url).await;

    let resp = client
        .get_or_create_player(proto::GetOrCreatePlayerRequest {
            guild_id: "guild42".into(),
            user_id: "user99".into(),
            username: "alice".into(),
        })
        .await
        .expect("RPC ok");

    let player = resp.into_inner();
    assert_eq!(player.guild_id, "guild42");
    assert_eq!(player.user_id, "user99");
    assert_eq!(player.username, "alice");
    assert_eq!(player.coins, 100);
    assert_eq!(player.level, 1);
    assert_eq!(player.hp_current, 100);
    assert_eq!(player.hp_max, 100);
    assert!(player.class.is_none());

    let _ = shutdown.send(());
}

#[tokio::test]
async fn get_player_returns_existing_state() {
    let (url, shutdown) = start_server().await;
    let mut client = connect(&url).await;

    let resp = client
        .get_player(proto::GetPlayerRequest {
            guild_id: "g1".into(),
            user_id: "veteran".into(),
        })
        .await
        .expect("RPC ok");

    let player = resp.into_inner();
    assert_eq!(player.username, "Existing");
    assert_eq!(player.coins, 999);
    assert_eq!(player.level, 7);
    assert_eq!(player.total_wins, 5);
    assert_eq!(player.hp_current, 80);

    let _ = shutdown.send(());
}

#[tokio::test]
async fn get_player_not_found_propagates_status() {
    let (url, shutdown) = start_server().await;
    let mut client = connect(&url).await;

    let err = client
        .get_player(proto::GetPlayerRequest {
            guild_id: "g".into(),
            user_id: "missing".into(),
        })
        .await
        .expect_err("RPC doit echouer");

    // domain_to_status mappe DomainError::NotFound vers Code::NotFound.
    assert_eq!(err.code(), tonic::Code::NotFound);

    let _ = shutdown.send(());
}

#[tokio::test]
async fn add_xp_returns_progress_with_levelup_flag() {
    let (url, shutdown) = start_server().await;
    let mut client = connect(&url).await;

    // Petite XP : pas de levelup.
    let small = client
        .add_xp(proto::AddXpRequest {
            guild_id: "g".into(),
            user_id: "u".into(),
            amount: 50,
        })
        .await
        .unwrap()
        .into_inner();
    assert_eq!(small.new_xp, 50);
    assert!(!small.leveled_up);
    assert_eq!(small.stat_points_gained, 0);

    // Grosse XP : levelup.
    let big = client
        .add_xp(proto::AddXpRequest {
            guild_id: "g".into(),
            user_id: "u".into(),
            amount: 250,
        })
        .await
        .unwrap()
        .into_inner();
    assert_eq!(big.new_xp, 250);
    assert!(big.leveled_up);
    assert_eq!(big.stat_points_gained, 1);

    let _ = shutdown.send(());
}

// ══════════════════════════════════════════════════════════════════════
// Phase 7A.opt F.1 — 5 services coude additionnels (smoke tests)
// ══════════════════════════════════════════════════════════════════════
//
// Pour chaque service on cree un mock minimal (1-2 methodes implementees,
// le reste en `unimplemented!()`) puis on lance UN appel reel pour valider
// la chaine wiring + serialisation + auth (pas d'auth ici car serveur sans
// interceptor) + reponse. Si le proto change ou la conversion casse, ces
// tests echouent.

fn ts() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 4, 11, 12, 0, 0).unwrap()
}

/// Lance un serveur tonic in-process avec un seul service. Utilise une macro
/// pour eviter les bornes generiques compliquees sur tonic::Service.
macro_rules! spawn_one_service {
    ($svc:expr) => {{
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let url = format!("http://{addr}");
        let (tx, rx) = oneshot::channel::<()>();
        tokio::spawn(async move {
            Server::builder()
                .add_service($svc)
                .serve_with_incoming_shutdown(
                    tokio_stream::wrappers::TcpListenerStream::new(listener),
                    async {
                        let _ = rx.await;
                    },
                )
                .await
                .unwrap();
        });
        tokio::time::sleep(Duration::from_millis(50)).await;
        (url, tx)
    }};
}

// ── Mock CombatsUseCase : list + create implementes ──

struct MockCombatsUc;

#[async_trait]
impl ManageCoudeCombatsUseCase for MockCombatsUc {
    async fn list(&self, guild_id: &str, _: Option<&str>, _: i64) -> Result<Vec<CoudeCombat>, DomainError> {
        Ok(vec![CoudeCombat {
            id: Uuid::nil(),
            guild_id: guild_id.into(),
            channel_id: Some("c".into()),
            attacker_id: "a".into(), attacker_name: "Atk".into(),
            defender_id: "d".into(), defender_name: "Def".into(),
            mise: 100, status: "pending".into(),
            winner_id: None, attacker_roll: None, defender_roll: None,
            chaos_event: None, special_attack: None, defender_special: None,
            coins_transferred: None, result_message: None, message_id: None,
            created_at: ts(), accepted_at: None, resolved_at: None,
        }])
    }
    async fn create(&self, new: NewCoudeCombat) -> Result<CoudeCombat, DomainError> {
        Ok(CoudeCombat {
            id: Uuid::new_v4(),
            guild_id: new.guild_id, channel_id: new.channel_id,
            attacker_id: new.attacker_id, attacker_name: new.attacker_name,
            defender_id: new.defender_id, defender_name: new.defender_name,
            mise: new.mise, status: "pending".into(),
            winner_id: None, attacker_roll: None, defender_roll: None,
            chaos_event: None, special_attack: new.special_attack, defender_special: None,
            coins_transferred: None, result_message: None, message_id: None,
            created_at: ts(), accepted_at: None, resolved_at: None,
        })
    }
    async fn get(&self, _: Uuid) -> Result<CoudeCombat, DomainError> { unimplemented!() }
    async fn get_pending_for_attacker(&self, _: &str, _: &str) -> Result<Option<CoudeCombat>, DomainError> { unimplemented!() }
    async fn get_pending_for_defender(&self, _: &str, _: &str) -> Result<Option<CoudeCombat>, DomainError> { unimplemented!() }
    async fn list_expired_pending(&self) -> Result<Vec<CoudeCombat>, DomainError> { unimplemented!() }
    async fn get_betting_for_participant(&self, _: &str, _: &str) -> Result<Option<CoudeCombat>, DomainError> { unimplemented!() }
    async fn cancel(&self, _: Uuid) -> Result<(), DomainError> { unimplemented!() }
    async fn resolve(&self, _: Uuid, _: CombatResolution) -> Result<(), DomainError> { unimplemented!() }
    async fn set_betting(&self, _: Uuid, _: &str) -> Result<bool, DomainError> { unimplemented!() }
    async fn expire(&self, _: Uuid) -> Result<(), DomainError> { unimplemented!() }
    async fn set_defender_special(&self, _: Uuid, _: &str) -> Result<(), DomainError> { unimplemented!() }
}

#[tokio::test]
async fn coude_combats_list_and_create_round_trip() {
    let svc = CoudeCombatsServiceServer::new(CoudeCombatsGrpc { uc: Arc::new(MockCombatsUc), resolve_batch_uc: Arc::new(test_helpers::StubResolveBettingBatch), expire_batch_uc: Arc::new(test_helpers::StubExpireCombatsBatch), resolve_now_uc: Arc::new(test_helpers::StubResolveCombatNow) });
    let (url, shutdown) = spawn_one_service!(svc);
    let mut client = CoudeCombatsServiceClient::connect(Endpoint::from_shared(url).unwrap())
        .await.unwrap();

    let list = client.list(proto::ListCombatsRequest {
        guild_id: "g".into(), status: None, limit: 10,
    }).await.unwrap().into_inner();
    assert_eq!(list.combats.len(), 1);
    assert_eq!(list.combats[0].mise, 100);

    let created = client.create(proto::CreateCombatRequest {
        guild_id: "g".into(), channel_id: Some("c".into()),
        attacker_id: "a".into(), attacker_name: "A".into(),
        defender_id: "d".into(), defender_name: "D".into(),
        mise: 250, special_attack: None,
    }).await.unwrap().into_inner();
    assert_eq!(created.mise, 250);
    assert_eq!(created.status, "pending");

    let _ = shutdown.send(());
}

// ── Mock BetsUseCase : list_for_combat ──

struct MockBetsUc;

#[async_trait]
impl ManageCoudeBetsUseCase for MockBetsUc {
    async fn list_for_combat(&self, combat_id: Uuid) -> Result<Vec<CoudeBet>, DomainError> {
        Ok(vec![CoudeBet {
            id: Uuid::from_u128(1), guild_id: "g".into(), combat_id,
            bettor_id: "u".into(), bettor_name: "Joe".into(),
            backed_id: "a".into(), amount: 100, won: None, payout: None,
        }])
    }
    async fn place(
        &self,
        _: NewCoudeBet,
    ) -> Result<sentinel_api::ports::inbound::PlaceBetOutcome, DomainError> {
        unimplemented!()
    }
    async fn resolve(
        &self,
        _: Uuid,
        _: Option<String>,
    ) -> Result<sentinel_api::ports::inbound::ResolveBetsOutcome, DomainError> {
        unimplemented!()
    }
    async fn refund(&self, _: Uuid) -> Result<RefundSummary, DomainError> { unimplemented!() }
}

#[tokio::test]
async fn coude_bets_list_for_combat_round_trip() {
    let svc = CoudeBetsServiceServer::new(CoudeBetsGrpc { uc: Arc::new(MockBetsUc) });
    let (url, shutdown) = spawn_one_service!(svc);
    let mut client = CoudeBetsServiceClient::connect(Endpoint::from_shared(url).unwrap())
        .await.unwrap();

    let combat_id = Uuid::new_v4();
    let bets = client.list_for_combat(proto::ListForCombatRequest {
        combat_id: combat_id.to_string(),
    }).await.unwrap().into_inner();
    assert_eq!(bets.bets.len(), 1);
    assert_eq!(bets.bets[0].amount, 100);
    assert_eq!(bets.bets[0].combat_id, combat_id.to_string());

    let _ = shutdown.send(());
}

// ── Mock EconomyUseCase : transfer + count_casino_today ──

struct MockEconomyUc;

#[async_trait]
impl ManageCoudeEconomyUseCase for MockEconomyUc {
    async fn transfer(
        &self,
        _: &str,
        from: &str,
        to: &str,
        amount: i64,
    ) -> Result<Vec<sentinel_api::domain::entities::TauntEvent>, DomainError> {
        if from == to || amount <= 0 {
            return Err(DomainError::ValidationError("transfer invalide".into()));
        }
        Ok(vec![])
    }
    async fn count_casino_today(&self, _: &str, _: &str) -> Result<i64, DomainError> { Ok(7) }
    async fn steal(&self, _: &str, _: &str, _: &str, _: i64) -> Result<sentinel_api::ports::inbound::manage_coude_economy::StealOutcome, DomainError> { unimplemented!() }
    async fn steal_fail_penalty(&self, _: &str, _: &str, _: i64) -> Result<(i64, Vec<sentinel_api::domain::entities::TauntEvent>), DomainError> { unimplemented!() }
    async fn record_casino_win(&self, _: &str, _: &str, _: i64) -> Result<(), DomainError> { unimplemented!() }
    async fn record_casino_loss(&self, _: &str, _: &str, _: i64) -> Result<(), DomainError> { unimplemented!() }
    async fn record_casino_faillite(&self, _: &str, _: &str) -> Result<i64, DomainError> { unimplemented!() }
    async fn sum_casino_gains_today(&self, _: &str, _: &str) -> Result<i64, DomainError> { unimplemented!() }
    async fn count_steal_today(&self, _: &str, _: &str) -> Result<i64, DomainError> { unimplemented!() }
}

#[tokio::test]
async fn coude_economy_transfer_and_count_round_trip() {
    let svc = CoudeEconomyServiceServer::new(CoudeEconomyGrpc { uc: Arc::new(MockEconomyUc) });
    let (url, shutdown) = spawn_one_service!(svc);
    let mut client = CoudeEconomyServiceClient::connect(Endpoint::from_shared(url).unwrap())
        .await.unwrap();

    // Transfer ok
    client.transfer(proto::TransferRequest {
        guild_id: "g".into(), from_id: "a".into(), to_id: "b".into(), amount: 100,
    }).await.unwrap();

    // Transfer invalide -> propage Validation -> Code::InvalidArgument
    let err = client.transfer(proto::TransferRequest {
        guild_id: "g".into(), from_id: "a".into(), to_id: "a".into(), amount: 50,
    }).await.unwrap_err();
    assert_eq!(err.code(), tonic::Code::InvalidArgument);

    // Count casino today
    let count = client.count_casino_today(proto::UserInGuildRequest {
        guild_id: "g".into(), user_id: "u".into(),
    }).await.unwrap().into_inner();
    assert_eq!(count.value, 7);

    let _ = shutdown.send(());
}

// ── Mock InventoryUseCase : list + has_item ──

struct MockInventoryUc;

#[async_trait]
impl ManageCoudeInventoryUseCase for MockInventoryUc {
    async fn list_inventory(&self, guild_id: &str, user_id: &str) -> Result<Vec<CoudeInventoryItem>, DomainError> {
        Ok(vec![
            CoudeInventoryItem { guild_id: guild_id.into(), user_id: user_id.into(), item_key: "potion".into(), quantity: 3 },
            CoudeInventoryItem { guild_id: guild_id.into(), user_id: user_id.into(), item_key: "shield".into(), quantity: 1 },
        ])
    }
    async fn has_item(&self, _: &str, _: &str, item_key: &str) -> Result<bool, DomainError> {
        Ok(item_key == "potion")
    }
    async fn add_item(&self, _: &str, _: &str, _: &str) -> Result<(), DomainError> { unimplemented!() }
    async fn use_item(&self, _: &str, _: &str, _: &str) -> Result<bool, DomainError> { unimplemented!() }
    async fn create_prime(&self, _: NewCoudePrime) -> Result<CoudePrime, DomainError> { unimplemented!() }
    async fn list_active_primes(&self, _: &str, _: &str) -> Result<Vec<CoudePrime>, DomainError> { unimplemented!() }
    async fn claim_primes(&self, _: &str, _: &str, _: &str, _: &str) -> Result<i64, DomainError> { unimplemented!() }
    async fn buy_insurance(&self, _: &str, _: &str, _: bool, _: i64) -> Result<bool, DomainError> { unimplemented!() }
    async fn get_active_insurance(&self, _: &str, _: &str) -> Result<Option<CoudeInsurance>, DomainError> { unimplemented!() }
    async fn expire_insurance(&self, _: Uuid) -> Result<(), DomainError> { unimplemented!() }
}

#[tokio::test]
async fn coude_inventory_list_and_has_item_round_trip() {
    let svc = CoudeInventoryServiceServer::new(CoudeInventoryGrpc { uc: Arc::new(MockInventoryUc), steal_protections_uc: Arc::new(test_helpers::StubCoudeStealProtections), steal_boosts_uc: Arc::new(test_helpers::StubCoudeStealBoosts) });
    let (url, shutdown) = spawn_one_service!(svc);
    let mut client = CoudeInventoryServiceClient::connect(Endpoint::from_shared(url).unwrap())
        .await.unwrap();

    let inv = client.list_inventory(proto::UserInGuildRequest {
        guild_id: "g".into(), user_id: "u".into(),
    }).await.unwrap().into_inner();
    assert_eq!(inv.items.len(), 2);
    assert_eq!(inv.items[0].item_key, "potion");
    assert_eq!(inv.items[0].quantity, 3);

    let has_potion = client.has_item(proto::HasItemRequest {
        guild_id: "g".into(), user_id: "u".into(), item_key: "potion".into(),
    }).await.unwrap().into_inner();
    assert!(has_potion.value);

    let has_sword = client.has_item(proto::HasItemRequest {
        guild_id: "g".into(), user_id: "u".into(), item_key: "sword".into(),
    }).await.unwrap().into_inner();
    assert!(!has_sword.value);

    let _ = shutdown.send(());
}

// ── Mock SocialUseCase : leaderboard + current_season ──

struct MockSocialUc;

#[async_trait]
impl ManageCoudeSocialUseCase for MockSocialUc {
    async fn leaderboard(&self, _: &str, _: LeaderboardCategory, limit: i64) -> Result<Vec<CoudeLeaderboardEntry>, DomainError> {
        Ok((0..limit.min(3)).map(|i| CoudeLeaderboardEntry {
            user_id: format!("u{i}"),
            username: format!("Player{i}"),
            value: 1000 - i * 100,
        }).collect())
    }
    async fn trigger_daily_chaos(&self, _: &str) -> Result<Option<sentinel_api::domain::entities::DailyChaosOutcome>, DomainError> { Ok(None) }
    async fn current_season(&self, _: &str) -> Result<CoudeCurrentSeason, DomainError> {
        Ok(CoudeCurrentSeason {
            season_number: 5,
            started_at: ts(),
            ends_at: ts(),
            days_remaining: 30,
        })
    }
    async fn check_cooldown(&self, _: &str, _: &str, _: &str) -> Result<Option<DateTime<Utc>>, DomainError> { unimplemented!() }
    async fn set_cooldown(&self, _: &str, _: &str, _: &str, _: i64) -> Result<(), DomainError> { unimplemented!() }
    async fn list_active_events(&self, _: &str) -> Result<Vec<CoudeEvent>, DomainError> { unimplemented!() }
    async fn log_daily_chaos(&self, _: NewDailyChaos) -> Result<(), DomainError> { unimplemented!() }
}

#[tokio::test]
async fn coude_social_leaderboard_and_season_round_trip() {
    let svc = CoudeSocialServiceServer::new(CoudeSocialGrpc { uc: Arc::new(MockSocialUc), cashbox_uc: Arc::new(test_helpers::StubCoudeCashbox), catalog_uc: Arc::new(test_helpers::StubCoudeCatalog), heist_uc: Arc::new(test_helpers::StubCoudeHeist), taunts_uc: Arc::new(test_helpers::StubCoudeTaunts) });
    let (url, shutdown) = spawn_one_service!(svc);
    let mut client = CoudeSocialServiceClient::connect(Endpoint::from_shared(url).unwrap())
        .await.unwrap();

    let lb = client.leaderboard(proto::LeaderboardRequest {
        guild_id: "g".into(),
        category: proto::LeaderboardCategory::Richest as i32,
        limit: 5,
    }).await.unwrap().into_inner();
    assert_eq!(lb.entries.len(), 3);
    assert_eq!(lb.entries[0].value, 1000);
    assert_eq!(lb.entries[2].value, 800);

    let season = client.current_season(proto::CurrentSeasonRequest {
        guild_id: "g".into(),
    }).await.unwrap().into_inner();
    assert_eq!(season.season_number, 5);
    assert_eq!(season.days_remaining, 30);

    let _ = shutdown.send(());
}
