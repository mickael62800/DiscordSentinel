//! Client gRPC partage entre les bots Sentinel (Phase 7A).
//!
//! Coexiste avec `BaseApiClient` (reqwest/HTTP) pendant la migration
//! bot-par-bot. Chaque bot stocke un `SentinelGrpcClient` dans son TypeMap
//! Serenity (cf. `GrpcClientKey`) en plus de l'`ApiClientKey` HTTP existant.
//!
//! Caracteristiques :
//! - Connexion HTTP/2 unique persistante (multiplexage natif tonic).
//! - Reconnexion lazy + retry transport gere par hyper en interne.
//! - Auth par metadata `authorization: Bearer <api_key>` injectee a chaque
//!   appel via interceptor (meme schema que cote serveur).
//! - Circuit breaker simple (cf. `circuit_breaker.rs`) pour degrader
//!   gracieusement quand l'API est down.
//!
//! ## Comportement si l'API tombe
//!
//! - **Reads** (`get_user_level`, `get_leaderboard`) : retournent
//!   `Err(GrpcCallError::Unavailable)` apres N echecs consecutifs, le circuit
//!   breaker s'ouvre pendant `cooldown` puis tente une requete (half-open).
//!   Les commandes slash doivent traduire ca en message « API indisponible,
//!   reessayez dans quelques instants » au lieu de planter.
//! - **Writes critiques** (`add_xp`) : meme comportement, le bot peut soit
//!   ignorer (XP perdu, acceptable), soit pousser dans Redis Streams pour
//!   replay differe (cf. `event_bus`).
//! - **Fire-and-forget** (record_messages, record_voice via HTTP legacy
//!   pour l'instant) : restent sur HTTP tant que la migration n'est pas
//!   terminee, geres par `BaseApiClient::post_fire_and_forget`.

use std::sync::Arc;
use std::time::Duration;

use serenity::prelude::TypeMapKey;
use tonic::codec::CompressionEncoding;
use tonic::metadata::MetadataValue;
use tonic::service::interceptor::InterceptedService;
use tonic::service::Interceptor;
use tonic::transport::{Channel, Endpoint};
use tonic::{Request, Status};
use tracing::{error, info, warn};

use sentinel_proto::ai_dataset::v1::ai_dataset_service_client::AiDatasetServiceClient;
use sentinel_proto::automod::v1::automod_service_client::AutomodServiceClient;
use sentinel_proto::blackjack::v1::blackjack_service_client::BlackjackServiceClient;
use sentinel_proto::coude::v1::coude_bets_service_client::CoudeBetsServiceClient;
use sentinel_proto::coude::v1::coude_combats_service_client::CoudeCombatsServiceClient;
use sentinel_proto::coude::v1::coude_economy_service_client::CoudeEconomyServiceClient;
use sentinel_proto::coude::v1::coude_inventory_service_client::CoudeInventoryServiceClient;
use sentinel_proto::coude::v1::coude_player_service_client::CoudePlayerServiceClient;
use sentinel_proto::coude::v1::coude_social_service_client::CoudeSocialServiceClient;
use sentinel_proto::members::v1::members_service_client::MembersServiceClient;
use sentinel_proto::moderation::v1::moderation_service_client::ModerationServiceClient;
use sentinel_proto::progression::v1::progression_service_client::ProgressionServiceClient;
use sentinel_proto::roles::v1::role_panels_service_client::RolePanelsServiceClient;
use sentinel_proto::security::v1::security_service_client::SecurityServiceClient;
use sentinel_proto::stats::v1::stats_service_client::StatsServiceClient;
use sentinel_proto::tamagotchi::v1::tamagotchi_service_client::TamagotchiServiceClient;
use sentinel_proto::tickets::v1::tickets_service_client::TicketsServiceClient;
use sentinel_proto::community::v1::community_service_client::CommunityServiceClient;
use sentinel_proto::voice::v1::voice_channels_service_client::VoiceChannelsServiceClient;
use sentinel_proto::welcome::v1::welcome_service_client::WelcomeServiceClient;

use super::circuit_breaker::CircuitBreaker;

/// Erreurs renvoyees par les appels gRPC du client partage.
#[derive(Debug, thiserror::Error)]
pub enum GrpcCallError {
    #[error("API indisponible (circuit breaker ouvert)")]
    Unavailable,
    #[error("appel gRPC echoue : {0}")]
    Status(#[from] Status),
    #[error("erreur transport : {0}")]
    Transport(#[from] tonic::transport::Error),
}

/// Format de secours reutilisable pour convertir un `GrpcCallError` en String.
/// La plupart des api_clients de modules s'en contentent ; blackjack a une
/// version custom qui nettoie les messages pour l'affichage utilisateur.
pub fn grpc_err_to_string(e: GrpcCallError) -> String {
    match e {
        GrpcCallError::Unavailable => "API indisponible (circuit breaker ouvert)".to_string(),
        GrpcCallError::Status(s) => format!("gRPC {:?}: {}", s.code(), s.message()),
        GrpcCallError::Transport(t) => format!("transport gRPC: {t}"),
    }
}

/// Client gRPC partage. Cloneable a moindre cout (Channel = Arc en interne).
#[derive(Clone)]
pub struct SentinelGrpcClient {
    channel: Channel,
    interceptor: AuthInterceptor,
    breaker: Arc<CircuitBreaker>,
}

impl SentinelGrpcClient {
    /// Construit un client a partir des variables d'environnement :
    /// - `GRPC_API_URL` (defaut : `http://127.0.0.1:50051`)
    /// - `API_KEY` (optionnelle, injectee dans `authorization`)
    pub async fn from_env() -> Result<Self, tonic::transport::Error> {
        let url = std::env::var("GRPC_API_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:50051".to_string());
        let api_key = std::env::var("API_KEY").unwrap_or_default();
        Self::connect(&url, &api_key).await
    }

    /// Construit un client en pointant explicitement une URL gRPC.
    pub async fn connect(url: &str, api_key: &str) -> Result<Self, tonic::transport::Error> {
        // Si mTLS active, force https:// dans l'URL. tonic exige https
        // pour declencher le handshake TLS lors du connect.
        let effective_url = if sentinel_proto::tls::tls_dir().is_some() {
            if let Some(rest) = url.strip_prefix("http://") {
                format!("https://{rest}")
            } else if !url.starts_with("https://") {
                format!("https://{url}")
            } else {
                url.to_string()
            }
        } else {
            url.to_string()
        };

        let endpoint = Endpoint::from_shared(effective_url)?
            // Phase 7A — tunings raisonnables. Le multiplexage HTTP/2 evite
            // de multiplier les connexions ; un seul Channel suffit pour tous
            // les RPC concurrents d'un meme bot.
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(30))
            .tcp_keepalive(Some(Duration::from_secs(60)))
            .http2_keep_alive_interval(Duration::from_secs(30))
            .keep_alive_timeout(Duration::from_secs(10))
            .keep_alive_while_idle(true);

        // mTLS optionnel : active si GRPC_TLS_DIR defini en env.
        // Domaine SAN du cert serveur = "api" (cf. gen-grpc-certs.sh).
        let endpoint = match sentinel_proto::tls::tls_dir() {
            Some(dir) => {
                let domain = url
                    .strip_prefix("http://")
                    .or_else(|| url.strip_prefix("https://"))
                    .unwrap_or(url)
                    .split(':')
                    .next()
                    .unwrap_or("api");
                match sentinel_proto::tls::client_tls_config(&dir, domain) {
                    Ok(tls) => match endpoint.clone().tls_config(tls) {
                        Ok(e) => {
                            info!(domain = %domain, "gRPC client TLS active (mTLS)");
                            e
                        }
                        Err(e) => {
                            info!(error = %e, "Echec config TLS client gRPC, fallback plain");
                            endpoint
                        }
                    },
                    Err(e) => {
                        info!(error = %e, "Echec lecture certs TLS gRPC, fallback plain");
                        endpoint
                    }
                }
            }
            None => endpoint,
        };

        let channel = endpoint.connect_lazy();
        info!(url = %url, "SentinelGrpcClient initialise (lazy connect)");

        Ok(Self {
            channel,
            interceptor: AuthInterceptor::new(api_key),
            breaker: Arc::new(CircuitBreaker::new(5, Duration::from_secs(10))),
        })
    }

    // ── Helpers de service ──
    //
    // Phase 7A optimisations : chaque client annonce l'envoi et l'acceptation
    // de la compression Gzip. Le serveur (cf. `sentinel-api/src/adapters/inbound/grpc/server.rs`)
    // accepte les deux, donc les deux bouts negocient gzip automatiquement.

    /// Retourne un client `ProgressionService` pret a l'emploi.
    pub fn progression(
        &self,
    ) -> ProgressionServiceClient<InterceptedService<Channel, AuthInterceptor>> {
        ProgressionServiceClient::with_interceptor(self.channel.clone(), self.interceptor.clone())
            .send_compressed(CompressionEncoding::Gzip)
            .accept_compressed(CompressionEncoding::Gzip)
    }

    /// Retourne un client `StatsService` pret a l'emploi.
    pub fn stats(
        &self,
    ) -> StatsServiceClient<InterceptedService<Channel, AuthInterceptor>> {
        StatsServiceClient::with_interceptor(self.channel.clone(), self.interceptor.clone())
            .send_compressed(CompressionEncoding::Gzip)
            .accept_compressed(CompressionEncoding::Gzip)
    }

    /// Retourne un client `TicketsService` pret a l'emploi.
    pub fn tickets(
        &self,
    ) -> TicketsServiceClient<InterceptedService<Channel, AuthInterceptor>> {
        TicketsServiceClient::with_interceptor(self.channel.clone(), self.interceptor.clone())
            .send_compressed(CompressionEncoding::Gzip)
            .accept_compressed(CompressionEncoding::Gzip)
    }

    /// Retourne un client `ModerationService` pret a l'emploi.
    pub fn moderation(
        &self,
    ) -> ModerationServiceClient<InterceptedService<Channel, AuthInterceptor>> {
        ModerationServiceClient::with_interceptor(self.channel.clone(), self.interceptor.clone())
            .send_compressed(CompressionEncoding::Gzip)
            .accept_compressed(CompressionEncoding::Gzip)
    }

    /// Retourne un client `BlackjackService` pret a l'emploi.
    pub fn blackjack(
        &self,
    ) -> BlackjackServiceClient<InterceptedService<Channel, AuthInterceptor>> {
        BlackjackServiceClient::with_interceptor(self.channel.clone(), self.interceptor.clone())
            .send_compressed(CompressionEncoding::Gzip)
            .accept_compressed(CompressionEncoding::Gzip)
    }

    /// Retourne un client `CoudePlayerService` pret a l'emploi.
    pub fn coude_players(
        &self,
    ) -> CoudePlayerServiceClient<InterceptedService<Channel, AuthInterceptor>> {
        CoudePlayerServiceClient::with_interceptor(self.channel.clone(), self.interceptor.clone())
            .send_compressed(CompressionEncoding::Gzip)
            .accept_compressed(CompressionEncoding::Gzip)
    }

    /// Phase 7A.opt F.1 — Retourne un client `CoudeCombatsService`.
    pub fn coude_combats(
        &self,
    ) -> CoudeCombatsServiceClient<InterceptedService<Channel, AuthInterceptor>> {
        CoudeCombatsServiceClient::with_interceptor(self.channel.clone(), self.interceptor.clone())
            .send_compressed(CompressionEncoding::Gzip)
            .accept_compressed(CompressionEncoding::Gzip)
    }

    /// Phase 7A.opt F.1 — Retourne un client `CoudeBetsService`.
    pub fn coude_bets(
        &self,
    ) -> CoudeBetsServiceClient<InterceptedService<Channel, AuthInterceptor>> {
        CoudeBetsServiceClient::with_interceptor(self.channel.clone(), self.interceptor.clone())
            .send_compressed(CompressionEncoding::Gzip)
            .accept_compressed(CompressionEncoding::Gzip)
    }

    /// Phase 7A.opt F.1 — Retourne un client `CoudeEconomyService`.
    pub fn coude_economy(
        &self,
    ) -> CoudeEconomyServiceClient<InterceptedService<Channel, AuthInterceptor>> {
        CoudeEconomyServiceClient::with_interceptor(self.channel.clone(), self.interceptor.clone())
            .send_compressed(CompressionEncoding::Gzip)
            .accept_compressed(CompressionEncoding::Gzip)
    }

    /// Phase 7A.opt F.1 — Retourne un client `CoudeInventoryService`.
    pub fn coude_inventory(
        &self,
    ) -> CoudeInventoryServiceClient<InterceptedService<Channel, AuthInterceptor>> {
        CoudeInventoryServiceClient::with_interceptor(self.channel.clone(), self.interceptor.clone())
            .send_compressed(CompressionEncoding::Gzip)
            .accept_compressed(CompressionEncoding::Gzip)
    }

    /// Phase 7A.opt F.1 — Retourne un client `CoudeSocialService`.
    pub fn coude_social(
        &self,
    ) -> CoudeSocialServiceClient<InterceptedService<Channel, AuthInterceptor>> {
        CoudeSocialServiceClient::with_interceptor(self.channel.clone(), self.interceptor.clone())
            .send_compressed(CompressionEncoding::Gzip)
            .accept_compressed(CompressionEncoding::Gzip)
    }

    /// Retourne un client `RolePanelsService` pret a l'emploi.
    pub fn role_panels(
        &self,
    ) -> RolePanelsServiceClient<InterceptedService<Channel, AuthInterceptor>> {
        RolePanelsServiceClient::with_interceptor(self.channel.clone(), self.interceptor.clone())
            .send_compressed(CompressionEncoding::Gzip)
            .accept_compressed(CompressionEncoding::Gzip)
    }

    /// Retourne un client `MembersService` pret a l'emploi.
    pub fn members(
        &self,
    ) -> MembersServiceClient<InterceptedService<Channel, AuthInterceptor>> {
        MembersServiceClient::with_interceptor(self.channel.clone(), self.interceptor.clone())
            .send_compressed(CompressionEncoding::Gzip)
            .accept_compressed(CompressionEncoding::Gzip)
    }

    /// Retourne un client `SecurityService` pret a l'emploi.
    pub fn security(
        &self,
    ) -> SecurityServiceClient<InterceptedService<Channel, AuthInterceptor>> {
        SecurityServiceClient::with_interceptor(self.channel.clone(), self.interceptor.clone())
            .send_compressed(CompressionEncoding::Gzip)
            .accept_compressed(CompressionEncoding::Gzip)
    }

    /// Retourne un client `AutomodService` pret a l'emploi.
    pub fn automod(
        &self,
    ) -> AutomodServiceClient<InterceptedService<Channel, AuthInterceptor>> {
        AutomodServiceClient::with_interceptor(self.channel.clone(), self.interceptor.clone())
            .send_compressed(CompressionEncoding::Gzip)
            .accept_compressed(CompressionEncoding::Gzip)
    }

    /// Retourne un client `VoiceChannelsService` pret a l'emploi.
    pub fn voice_channels(
        &self,
    ) -> VoiceChannelsServiceClient<InterceptedService<Channel, AuthInterceptor>> {
        VoiceChannelsServiceClient::with_interceptor(self.channel.clone(), self.interceptor.clone())
            .send_compressed(CompressionEncoding::Gzip)
            .accept_compressed(CompressionEncoding::Gzip)
    }

    /// Phase 7A.opt F.4 — Retourne un client `WelcomeService`.
    pub fn welcome(
        &self,
    ) -> WelcomeServiceClient<InterceptedService<Channel, AuthInterceptor>> {
        WelcomeServiceClient::with_interceptor(self.channel.clone(), self.interceptor.clone())
            .send_compressed(CompressionEncoding::Gzip)
            .accept_compressed(CompressionEncoding::Gzip)
    }

    /// Phase 7A.opt F.3 — Retourne un client `CommunityService`.
    pub fn community(
        &self,
    ) -> CommunityServiceClient<InterceptedService<Channel, AuthInterceptor>> {
        CommunityServiceClient::with_interceptor(self.channel.clone(), self.interceptor.clone())
            .send_compressed(CompressionEncoding::Gzip)
            .accept_compressed(CompressionEncoding::Gzip)
    }

    /// Retourne un client `TamagotchiService` pret a l'emploi.
    pub fn tamagotchi(
        &self,
    ) -> TamagotchiServiceClient<InterceptedService<Channel, AuthInterceptor>> {
        TamagotchiServiceClient::with_interceptor(self.channel.clone(), self.interceptor.clone())
            .send_compressed(CompressionEncoding::Gzip)
            .accept_compressed(CompressionEncoding::Gzip)
    }

    /// Retourne un client `AiDatasetService` pret a l'emploi.
    pub fn ai_dataset(
        &self,
    ) -> AiDatasetServiceClient<InterceptedService<Channel, AuthInterceptor>> {
        AiDatasetServiceClient::with_interceptor(self.channel.clone(), self.interceptor.clone())
            .send_compressed(CompressionEncoding::Gzip)
            .accept_compressed(CompressionEncoding::Gzip)
    }

    /// Wrappe un appel gRPC dans le circuit breaker. A utiliser dans les
    /// wrappers metier des bots pour beneficier de la degradation gracieuse.
    pub async fn guarded<F, Fut, T>(&self, call: F) -> Result<T, GrpcCallError>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T, Status>>,
    {
        if !self.breaker.allow() {
            warn!("Circuit breaker ouvert : appel gRPC court-circuite");
            return Err(GrpcCallError::Unavailable);
        }
        match call().await {
            Ok(v) => {
                self.breaker.record_success();
                Ok(v)
            }
            Err(status) => {
                if matches!(
                    status.code(),
                    tonic::Code::Unavailable
                        | tonic::Code::DeadlineExceeded
                        | tonic::Code::Internal
                ) {
                    self.breaker.record_failure();
                    error!(code = ?status.code(), "Echec gRPC compte par le circuit breaker");
                }
                Err(GrpcCallError::Status(status))
            }
        }
    }
}

/// Cle TypeMap pour stocker le `SentinelGrpcClient` dans le data store de Serenity.
pub struct GrpcClientKey;
impl TypeMapKey for GrpcClientKey {
    type Value = Arc<SentinelGrpcClient>;
}

// ── Interceptor d'auth ──

#[derive(Clone)]
pub struct AuthInterceptor {
    header: Option<MetadataValue<tonic::metadata::Ascii>>,
}

impl AuthInterceptor {
    fn new(api_key: &str) -> Self {
        let header = if api_key.is_empty() {
            None
        } else {
            match format!("Bearer {api_key}").parse::<MetadataValue<_>>() {
                Ok(v) => Some(v),
                Err(_) => {
                    error!("API_KEY invalide pour un header gRPC, auth desactivee cote client");
                    None
                }
            }
        };
        Self { header }
    }
}

impl Interceptor for AuthInterceptor {
    fn call(&mut self, mut req: Request<()>) -> Result<Request<()>, Status> {
        if let Some(h) = &self.header {
            req.metadata_mut().insert("authorization", h.clone());
        }
        Ok(req)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_api_key_yields_no_header() {
        let mut interceptor = AuthInterceptor::new("");
        let req = interceptor.call(Request::new(())).unwrap();
        assert!(req.metadata().get("authorization").is_none());
    }

    #[test]
    fn ascii_api_key_injects_bearer_header() {
        let mut interceptor = AuthInterceptor::new("topsecret");
        let req = interceptor.call(Request::new(())).unwrap();
        let header = req.metadata().get("authorization").expect("header present");
        assert_eq!(header.to_str().unwrap(), "Bearer topsecret");
    }

    #[test]
    fn invalid_api_key_chars_disable_auth_silently() {
        let mut interceptor = AuthInterceptor::new("bad\nkey\0");
        let req = interceptor.call(Request::new(())).unwrap();
        assert!(req.metadata().get("authorization").is_none());
    }

    #[test]
    fn interceptor_clone_preserves_header() {
        let interceptor = AuthInterceptor::new("abc123");
        let mut clone = interceptor.clone();
        let req = clone.call(Request::new(())).unwrap();
        assert_eq!(
            req.metadata().get("authorization").unwrap().to_str().unwrap(),
            "Bearer abc123"
        );
    }

    #[test]
    fn grpc_call_error_status_variant() {
        let status = Status::unavailable("api down");
        let err = GrpcCallError::Status(status);
        match err {
            GrpcCallError::Status(s) => assert_eq!(s.code(), tonic::Code::Unavailable),
            _ => panic!("expected Status variant"),
        }
    }

    #[test]
    fn grpc_call_error_unavailable_display() {
        let err = GrpcCallError::Unavailable;
        let msg = format!("{err}");
        assert!(msg.contains("indisponible") || msg.contains("circuit breaker"));
    }
}
