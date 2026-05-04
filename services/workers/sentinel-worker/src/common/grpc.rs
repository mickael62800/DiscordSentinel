//! Helpers de connection gRPC vers l'API Sentinel.
//!
//! Centralise le boilerplate `Endpoint::from_shared + connect_timeout +
//! connect + with_interceptor(Authorization Bearer)` duplique dans
//! `coude-worker/{hp_regen,expire_combats,daily_chaos}` et
//! `export-worker/drain_export_jobs`.
//!
//! Lit `GRPC_API_URL` (default `http://127.0.0.1:50051`) et `API_KEY`
//! depuis l'environnement.

use std::time::Duration;

use tonic::metadata::MetadataValue;
use tonic::transport::{Channel, Endpoint};
use tonic::Request;

const DEFAULT_GRPC_URL: &str = "http://127.0.0.1:50051";
const DEFAULT_CONNECT_TIMEOUT_SECS: u64 = 5;
const DEFAULT_REQUEST_TIMEOUT_SECS: u64 = 30;

/// Connecte un Channel gRPC vers `GRPC_API_URL` avec timeouts par defaut
/// (connect 5s, request 30s). Active mTLS si `GRPC_TLS_DIR` defini en env.
/// Retourne une `String` d'erreur prete a remonter au scheduler.
pub async fn connect() -> Result<Channel, String> {
    let url = std::env::var("GRPC_API_URL").unwrap_or_else(|_| DEFAULT_GRPC_URL.to_string());

    // Si mTLS active, force https:// dans l'URL. tonic exige https pour
    // declencher le handshake TLS lors du connect.
    let effective_url = if sentinel_proto::tls::tls_dir().is_some() {
        if let Some(rest) = url.strip_prefix("http://") {
            format!("https://{rest}")
        } else if !url.starts_with("https://") {
            format!("https://{url}")
        } else {
            url.clone()
        }
    } else {
        url.clone()
    };

    let endpoint = Endpoint::from_shared(effective_url.clone())
        .map_err(|e| format!("invalid GRPC_API_URL {url}: {e}"))?
        .connect_timeout(Duration::from_secs(DEFAULT_CONNECT_TIMEOUT_SECS))
        .timeout(Duration::from_secs(DEFAULT_REQUEST_TIMEOUT_SECS));

    // mTLS optionnel : active si GRPC_TLS_DIR defini.
    // tls_config(self, ...) consomme self -> on construit la chaine en
    // une seule expression.
    let endpoint = match sentinel_proto::tls::tls_dir() {
        Some(dir) => {
            let domain = url
                .strip_prefix("http://")
                .or_else(|| url.strip_prefix("https://"))
                .unwrap_or(&url)
                .split(':')
                .next()
                .unwrap_or("api");
            let tls = sentinel_proto::tls::client_tls_config(&dir, domain)
                .map_err(|e| format!("read TLS certs: {e}"))?;
            endpoint
                .tls_config(tls)
                .map_err(|e| format!("tls_config gRPC: {e}"))?
        }
        None => endpoint,
    };

    endpoint
        .connect()
        .await
        .map_err(|e| format!("connect gRPC {url}: {e}"))
}

/// Construit un interceptor `tonic` qui ajoute `Authorization: Bearer
/// {API_KEY}` a chaque requete sortante. Utilisable avec
/// `MyServiceClient::with_interceptor(channel, common::grpc::bearer_interceptor()?)`.
///
/// Si `API_KEY` est vide ou invalide en metadata, l'interceptor laisse
/// passer la requete sans auth (l'API repondra 401).
pub fn bearer_interceptor() -> Result<impl Fn(Request<()>) -> Result<Request<()>, tonic::Status> + Clone, String> {
    let api_key = std::env::var("API_KEY").unwrap_or_default();
    let auth: Option<MetadataValue<_>> = if api_key.is_empty() {
        None
    } else {
        Some(
            format!("Bearer {api_key}")
                .parse()
                .map_err(|e| format!("invalid api_key: {e}"))?,
        )
    };
    Ok(move |mut req: Request<()>| -> Result<Request<()>, tonic::Status> {
        if let Some(ref v) = auth {
            req.metadata_mut().insert("authorization", v.clone());
        }
        Ok(req)
    })
}

/// Helper pour ajouter le header Bearer a une requete tonic specifique
/// (alternative a `with_interceptor` quand on construit le client a la volee
/// avec `Channel::clone`).
pub fn with_bearer<T>(req: &mut Request<T>) -> Result<(), String> {
    let api_key = std::env::var("API_KEY").unwrap_or_default();
    if api_key.is_empty() {
        return Ok(());
    }
    let v: MetadataValue<_> = format!("Bearer {api_key}")
        .parse()
        .map_err(|e| format!("invalid api_key: {e}"))?;
    req.metadata_mut().insert("authorization", v);
    Ok(())
}
