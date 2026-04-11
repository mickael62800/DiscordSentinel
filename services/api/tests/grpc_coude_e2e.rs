//! Test d'integration end-to-end pour CoudePlayerService gRPC.
//!
//! Demarre un vrai serveur tonic in-process sur 127.0.0.1:0 (port libre),
//! pointe un client tonic dessus, et fait un appel reel `GetOrCreatePlayer`.
//! Le use case est mocke (n'utilise pas de DB) — on valide juste la chaine
//! complete : serialisation proto -> reseau -> handler -> conversion DTO ->
//! reponse proto -> deserialisation client.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use chrono::{TimeZone, Utc};
use tokio::sync::oneshot;
use tonic::transport::{Endpoint, Server};

use sentinel_api::adapters::inbound::grpc::coude::CoudePlayerGrpc;
use sentinel_api::domain::entities::{CombatStat, CoudePlayer, XpProgress};
use sentinel_api::domain::errors::DomainError;
use sentinel_api::ports::inbound::manage_coude_players::ManageCoudePlayersUseCase;
use sentinel_proto::coude::v1 as proto;
use sentinel_proto::coude::v1::coude_player_service_client::CoudePlayerServiceClient;
use sentinel_proto::coude::v1::coude_player_service_server::CoudePlayerServiceServer;

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
    async fn adjust_coins(&self, _: &str, _: &str, _: i64) -> Result<(), DomainError> { unimplemented!() }
    async fn record_coins_earned(&self, _: &str, _: &str, _: i64) -> Result<(), DomainError> { unimplemented!() }
    async fn record_coins_lost(&self, _: &str, _: &str, _: i64) -> Result<(), DomainError> { unimplemented!() }
    async fn update_hp(&self, _: &str, _: &str, _: i32, _: i32) -> Result<(), DomainError> { unimplemented!() }
    async fn full_heal(&self, _: &str, _: &str) -> Result<(), DomainError> { unimplemented!() }
}

// ── Helper : demarre un serveur in-process et retourne (url, shutdown_tx) ──

async fn start_server() -> (String, oneshot::Sender<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let url = format!("http://{addr}");

    let (tx, rx) = oneshot::channel::<()>();
    let svc = CoudePlayerGrpc { players_uc: Arc::new(MockPlayersUc) };
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
