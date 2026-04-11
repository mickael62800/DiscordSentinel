//! Tests d'integration REELS contre l'API gRPC qui tourne dans Docker.
//!
//! Pre-requis : `docker compose up -d api postgres redis pgbouncer`.
//! Le serveur tonic ecoute sur `localhost:50051`, l'API key par defaut est
//! lue depuis `GRPC_LIVE_API_KEY` (sinon `PHWHQOHDQFNHEGQHDEYFUWFHKUGTFKBY`,
//! la valeur de dev par defaut).
//!
//! Marques `#[ignore]` pour ne pas casser `cargo test` en environnement sans
//! Docker. Lancement explicite :
//!
//! ```bash
//! cargo test -p sentinel-api --test grpc_live_integration -- --ignored --nocapture
//! ```
//!
//! Ces tests valident la chaine COMPLETE :
//! 1. Client gRPC -> reseau -> serveur tonic Docker
//! 2. AuthInterceptor (header `authorization: Bearer <key>`)
//! 3. Handler grpc -> use case reel -> repo Postgres
//! 4. Reponse proto serialisee -> deserialisation client
//!
//! Toutes les ecritures utilisent un guild_id aleatoire (UUID-derive) pour
//! eviter de polluer les donnees existantes. Pas de cleanup explicite : les
//! lignes orphelines tombent au prochain run de `cleanup-worker`.

use std::time::Duration;

use tonic::metadata::MetadataValue;
use tonic::transport::{Channel, Endpoint};
use tonic::Request;

use sentinel_proto::coude::v1 as proto;
use sentinel_proto::coude::v1::coude_player_service_client::CoudePlayerServiceClient;

const DEFAULT_API_URL: &str = "http://127.0.0.1:50051";
const DEFAULT_API_KEY: &str = "PHWHQOHDQFNHEGQHDEYFUWFHKUGTFKBY";

fn api_url() -> String {
    std::env::var("GRPC_LIVE_API_URL").unwrap_or_else(|_| DEFAULT_API_URL.to_string())
}

fn api_key() -> String {
    std::env::var("GRPC_LIVE_API_KEY").unwrap_or_else(|_| DEFAULT_API_KEY.to_string())
}

fn unique_id() -> String {
    // Snowflake-like : 18 chiffres derives d'un UUID v4. Evite les collisions
    // avec les guild_ids existants.
    let raw = uuid::Uuid::new_v4().as_u128();
    format!("{}", raw % 1_000_000_000_000_000_000_u128)
}

async fn connect() -> Channel {
    let url = api_url();
    Endpoint::from_shared(url.clone())
        .expect("URL gRPC valide")
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(15))
        .connect()
        .await
        .unwrap_or_else(|e| panic!("Connexion gRPC echouee vers {url} : {e}. Lance `docker compose up -d api`."))
}

fn auth<T>(mut req: Request<T>) -> Request<T> {
    let token: MetadataValue<_> = format!("Bearer {}", api_key())
        .parse()
        .expect("API_KEY ASCII");
    req.metadata_mut().insert("authorization", token);
    req
}

// ══════════════════════════════════════════════════════════════════════
// Smoke : connexion gRPC + auth + RPC reel sur la vraie DB
// ══════════════════════════════════════════════════════════════════════

#[tokio::test]
#[ignore = "necessite la stack Docker (api + postgres) — lancer avec --ignored"]
async fn live_get_or_create_then_get_player() {
    let channel = connect().await;
    let mut client = CoudePlayerServiceClient::new(channel);

    let guild_id = unique_id();
    let user_id = unique_id();

    // 1. GetOrCreate cree le joueur (premiere fois)
    let req = auth(Request::new(proto::GetOrCreatePlayerRequest {
        guild_id: guild_id.clone(),
        user_id: user_id.clone(),
        username: "live_test_user".into(),
    }));
    let created = client
        .get_or_create_player(req)
        .await
        .expect("get_or_create reussi")
        .into_inner();

    assert_eq!(created.guild_id, guild_id);
    assert_eq!(created.user_id, user_id);
    assert_eq!(created.username, "live_test_user");
    assert_eq!(created.level, 1, "nouveau joueur niveau 1");
    assert_eq!(created.xp, 0);
    assert!(created.coins >= 0);

    // 2. Get verifie que la creation a bien persiste en DB
    let req = auth(Request::new(proto::GetPlayerRequest {
        guild_id: guild_id.clone(),
        user_id: user_id.clone(),
    }));
    let fetched = client
        .get_player(req)
        .await
        .expect("get reussi apres create")
        .into_inner();

    assert_eq!(fetched.guild_id, guild_id);
    assert_eq!(fetched.user_id, user_id);
    assert_eq!(fetched.username, "live_test_user");
    // L'horodatage created_at doit etre du jour meme.
    assert!(
        fetched.created_at.starts_with(&chrono::Utc::now().format("%Y-%m-%d").to_string()),
        "created_at attendu aujourd'hui, recu {}",
        fetched.created_at
    );

    // 3. GetOrCreate de nouveau retourne le meme joueur (pas de duplicate)
    let req = auth(Request::new(proto::GetOrCreatePlayerRequest {
        guild_id: guild_id.clone(),
        user_id: user_id.clone(),
        username: "different_name".into(),
    }));
    let again = client
        .get_or_create_player(req)
        .await
        .unwrap()
        .into_inner();
    assert_eq!(again.user_id, user_id, "doit etre le meme joueur");
}

#[tokio::test]
#[ignore = "necessite la stack Docker (api + postgres) — lancer avec --ignored"]
async fn live_add_xp_persists_progression() {
    let channel = connect().await;
    let mut client = CoudePlayerServiceClient::new(channel);

    let guild_id = unique_id();
    let user_id = unique_id();

    // Cree le joueur
    let req = auth(Request::new(proto::GetOrCreatePlayerRequest {
        guild_id: guild_id.clone(),
        user_id: user_id.clone(),
        username: "xp_test".into(),
    }));
    let initial = client.get_or_create_player(req).await.unwrap().into_inner();
    let initial_xp = initial.xp;

    // Ajoute 75 XP
    let req = auth(Request::new(proto::AddXpRequest {
        guild_id: guild_id.clone(),
        user_id: user_id.clone(),
        amount: 75,
    }));
    let progress = client.add_xp(req).await.unwrap().into_inner();
    assert_eq!(progress.new_xp, initial_xp + 75);

    // Re-fetch confirme la persistance
    let req = auth(Request::new(proto::GetPlayerRequest {
        guild_id: guild_id.clone(),
        user_id: user_id.clone(),
    }));
    let after = client.get_player(req).await.unwrap().into_inner();
    assert_eq!(after.xp, initial_xp + 75);
}

#[tokio::test]
#[ignore = "necessite la stack Docker (api + postgres) — lancer avec --ignored"]
async fn live_get_unknown_player_returns_not_found() {
    let channel = connect().await;
    let mut client = CoudePlayerServiceClient::new(channel);

    let req = auth(Request::new(proto::GetPlayerRequest {
        guild_id: unique_id(),  // guild qui n'existe pas
        user_id: unique_id(),
    }));
    let err = client.get_player(req).await.expect_err("doit echouer");

    // domain_to_status mappe DomainError::NotFound vers Code::NotFound.
    assert_eq!(err.code(), tonic::Code::NotFound);
}

#[tokio::test]
#[ignore = "necessite la stack Docker (api + postgres) — lancer avec --ignored"]
async fn live_missing_auth_token_is_unauthenticated() {
    let channel = connect().await;
    let mut client = CoudePlayerServiceClient::new(channel);

    // Pas d'auth() - request sans header authorization
    let req = Request::new(proto::GetPlayerRequest {
        guild_id: "1".into(),
        user_id: "1".into(),
    });
    let err = client.get_player(req).await.expect_err("doit echouer");
    assert_eq!(err.code(), tonic::Code::Unauthenticated);
}
