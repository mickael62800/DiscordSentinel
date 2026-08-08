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
use tonic::transport::Channel;
use tonic::transport::Endpoint;
use tonic::Request;

use sentinel_proto::community::v1 as com_proto;
use sentinel_proto::community::v1::community_service_client::CommunityServiceClient;
use sentinel_proto::tickets::v1 as tickets_proto;
use sentinel_proto::tickets::v1::tickets_service_client::TicketsServiceClient;
use sentinel_proto::welcome::v1 as welcome_proto;
use sentinel_proto::welcome::v1::welcome_service_client::WelcomeServiceClient;

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
        .unwrap_or_else(|e| {
            panic!("Connexion gRPC echouee vers {url} : {e}. Lance `docker compose up -d api`.")
        })
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
async fn live_missing_auth_token_is_unauthenticated() {
    let channel = connect().await;
    let mut client = TicketsServiceClient::new(channel);

    // Pas d'auth() - request sans header authorization
    let req = Request::new(tickets_proto::ListTicketsRequest {
        status: None,
        priority: None,
        search: None,
        author_id: None,
        limit: 5,
        offset: 0,
    });
    let err = client.list_tickets(req).await.expect_err("doit echouer");
    assert_eq!(err.code(), tonic::Code::Unauthenticated);
}

// ══════════════════════════════════════════════════════════════════════
// Tests LIVE pour services dependants de la DB (Tickets/Welcome/
// Community/Blackjack). Ils ne sont pas testables in-process via mocks
// car les handlers utilisent sqlx::PgPool ou des structs concretes.
// ══════════════════════════════════════════════════════════════════════

#[tokio::test]
#[ignore = "necessite la stack Docker (api + postgres) — lancer avec --ignored"]
async fn live_tickets_list_smoke() {
    let channel = connect().await;
    let mut client = TicketsServiceClient::new(channel);

    let req = auth(Request::new(tickets_proto::ListTicketsRequest {
        status: None,
        priority: None,
        search: None,
        author_id: None,
        limit: 5,
        offset: 0,
    }));
    // Smoke : on attend juste un Ok (pas de panic, pas de status d'erreur).
    let resp = client.list_tickets(req).await.expect("list_tickets reussi");
    let list = resp.into_inner();
    // La DB peut etre vide en environnement de test — on valide juste le type.
    assert!(list.tickets.len() <= 5);
}

#[tokio::test]
#[ignore = "necessite la stack Docker (api + postgres) — lancer avec --ignored"]
async fn live_welcome_get_config_returns_default_or_existing() {
    let channel = connect().await;
    let mut client = WelcomeServiceClient::new(channel);

    let req = auth(Request::new(welcome_proto::GetConfigRequest {
        guild_id: unique_id(),
    }));
    // GetConfig doit toujours reussir (renvoie une config par defaut si
    // pas en DB).
    let cfg = client
        .get_config(req)
        .await
        .expect("get_config reussi")
        .into_inner();
    // La config existe forcement (champ welcome_enabled doit etre lisible).
    let _ = cfg.welcome_enabled;
}

#[tokio::test]
#[ignore = "necessite la stack Docker (api + postgres) — lancer avec --ignored"]
async fn live_community_list_sponsorships_and_temp_roles() {
    let channel = connect().await;
    let mut client = CommunityServiceClient::new(channel);

    let guild = unique_id();

    let sponsors = client
        .list_sponsorships(auth(Request::new(com_proto::ListSponsorshipsRequest {
            guild_id: guild.clone(),
        })))
        .await
        .expect("list_sponsorships reussi")
        .into_inner();
    assert!(sponsors.sponsorships.is_empty(), "guild aleatoire = vide");

    let temp_roles = client
        .list_temp_roles(auth(Request::new(com_proto::ListTempRolesRequest {
            guild_id: guild,
        })))
        .await
        .expect("list_temp_roles reussi")
        .into_inner();
    assert!(temp_roles.roles.is_empty(), "guild aleatoire = vide");
}
