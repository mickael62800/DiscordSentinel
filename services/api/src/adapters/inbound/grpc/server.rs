//! Demarrage du serveur tonic en parallele d'Axum.
//!
//! Phase 7A optimisations :
//! - **Compression Gzip** activee en envoi et reception sur les 12 services
//!   (gain ~40-70% bande passante sur images, ~30% sur leaderboards, CPU <5%).
//! - **tonic-health** expose un service `grpc.health.v1.Health` qui permet au
//!   healthcheck Docker de verifier chaque service individuellement via
//!   `grpc_health_probe -addr=:50051`.
//!
//! Auth : si `api_key` est non vide, un interceptor verifie l'en-tete de
//! metadata `authorization: Bearer <api_key>` sur chaque appel. Sinon le
//! serveur tourne ouvert (mode dev) — meme logique qu'`adapters/inbound/http`.

use std::net::SocketAddr;
use std::sync::Arc;

use tonic::codec::CompressionEncoding;
use tonic::metadata::MetadataValue;
use tonic::service::interceptor::InterceptedService;
use tonic::transport::Server;
use tonic::{Request, Status};
use tracing::{error, info};

use sentinel_proto::automod::v1::automod_service_server::AutomodServiceServer;
use sentinel_proto::blackjack::v1::blackjack_service_server::BlackjackServiceServer;
use sentinel_proto::community::v1::community_service_server::CommunityServiceServer;
use sentinel_proto::coude::v1::coude_player_service_server::CoudePlayerServiceServer;
use sentinel_proto::images::v1::images_service_server::ImagesServiceServer;
use sentinel_proto::members::v1::members_service_server::MembersServiceServer;
use sentinel_proto::moderation::v1::moderation_service_server::ModerationServiceServer;
use sentinel_proto::progression::v1::progression_service_server::ProgressionServiceServer;
use sentinel_proto::roles::v1::role_panels_service_server::RolePanelsServiceServer;
use sentinel_proto::security::v1::security_service_server::SecurityServiceServer;
use sentinel_proto::stats::v1::stats_service_server::StatsServiceServer;
use sentinel_proto::tickets::v1::tickets_service_server::TicketsServiceServer;
use sentinel_proto::voice::v1::voice_channels_service_server::VoiceChannelsServiceServer;
use sentinel_proto::welcome::v1::welcome_service_server::WelcomeServiceServer;

use crate::adapters::inbound::grpc::automod::AutomodGrpc;
use crate::adapters::inbound::grpc::blackjack::BlackjackGrpc;
use crate::adapters::inbound::grpc::community::CommunityGrpc;
use crate::adapters::inbound::grpc::coude::CoudePlayerGrpc;
use crate::adapters::inbound::grpc::images::ImagesGrpc;
use crate::adapters::inbound::grpc::members::MembersGrpc;
use crate::adapters::inbound::grpc::moderation::ModerationGrpc;
use crate::adapters::inbound::grpc::progression::ProgressionGrpc;
use crate::adapters::inbound::grpc::roles::RolePanelsGrpc;
use crate::adapters::inbound::grpc::security::SecurityGrpc;
use crate::adapters::inbound::grpc::stats::StatsGrpc;
use crate::adapters::inbound::grpc::tickets::TicketsGrpc;
use crate::adapters::inbound::grpc::voice::VoiceChannelsGrpc;
use crate::adapters::inbound::grpc::welcome::WelcomeGrpc;
use crate::adapters::inbound::http::state::AppState;

/// Lance le serveur gRPC. A spawn dans une task tokio depuis `main.rs`.
pub async fn serve_grpc(state: AppState, bind: SocketAddr) {
    let api_key = state.api_key.clone();

    let progression = ProgressionGrpc {
        levels_uc: state.levels_uc.clone(),
        broadcaster: state.broadcaster.clone(),
    };
    let stats = StatsGrpc {
        stats_uc: state.stats_uc.clone(),
        broadcaster: state.broadcaster.clone(),
    };
    let tickets = TicketsGrpc {
        tickets_uc: state.tickets_uc.clone(),
        pg_pool: state.pg_pool.clone(),
    };
    let moderation = ModerationGrpc {
        moderation_uc: state.moderation_uc.clone(),
    };
    let blackjack = BlackjackGrpc {
        svc: state.blackjack_svc.clone(),
        wallet_repo: state.wallet_repo.clone(),
        bot_config_repo: state.bot_config_repo.clone(),
        broadcaster: state.broadcaster.clone(),
    };
    let coude = CoudePlayerGrpc {
        players_uc: state.coude_players_uc.clone(),
    };
    let roles = RolePanelsGrpc {
        uc: state.role_panels_uc.clone(),
        discord_role_repo: state.discord_role_repo.clone(),
    };
    let welcome = WelcomeGrpc {
        pg_pool: state.pg_pool.clone(),
    };
    let community = CommunityGrpc {
        pg_pool: state.pg_pool.clone(),
    };
    let members = MembersGrpc {
        uc: state.members_uc.clone(),
    };
    let security = SecurityGrpc {
        uc: state.security_uc.clone(),
    };
    let automod = AutomodGrpc {
        uc: state.analyze_uc.clone(),
    };
    let voice = VoiceChannelsGrpc {
        uc: state.voice_channels_uc.clone(),
    };
    let images = ImagesGrpc {
        uc: state.analyze_image_uc.clone(),
    };

    // Helper local : compression Gzip (send/accept) puis wrap dans l'auth
    // interceptor. Les methodes `send_compressed`/`accept_compressed` sont sur
    // le ServiceServer ; l'interceptor vient par-dessus via InterceptedService.
    macro_rules! svc {
        ($ServerType:ident, $impl:expr) => {{
            let inner = $ServerType::new($impl)
                .send_compressed(CompressionEncoding::Gzip)
                .accept_compressed(CompressionEncoding::Gzip);
            InterceptedService::new(inner, build_auth_interceptor(api_key.clone()))
        }};
    }

    let progression_svc = svc!(ProgressionServiceServer, progression);
    let stats_svc = svc!(StatsServiceServer, stats);
    let tickets_svc = svc!(TicketsServiceServer, tickets);
    let moderation_svc = svc!(ModerationServiceServer, moderation);
    let blackjack_svc = svc!(BlackjackServiceServer, blackjack);
    let coude_svc = svc!(CoudePlayerServiceServer, coude);
    let roles_svc = svc!(RolePanelsServiceServer, roles);
    let members_svc = svc!(MembersServiceServer, members);
    let security_svc = svc!(SecurityServiceServer, security);
    let automod_svc = svc!(AutomodServiceServer, automod);
    let voice_svc = svc!(VoiceChannelsServiceServer, voice);
    let images_svc = svc!(ImagesServiceServer, images);
    // Phase 7A.opt F.3/F.4 — nouveaux services.
    let welcome_svc = svc!(WelcomeServiceServer, welcome);
    let community_svc = svc!(CommunityServiceServer, community);

    // tonic-health : expose `grpc.health.v1.Health` + marque chaque service
    // comme SERVING. Permet `grpc_health_probe -addr=:50051` dans le healthcheck.
    let (mut health_reporter, health_service) = tonic_health::server::health_reporter();
    health_reporter
        .set_serving::<ProgressionServiceServer<ProgressionGrpc>>()
        .await;
    health_reporter
        .set_serving::<StatsServiceServer<StatsGrpc>>()
        .await;
    health_reporter
        .set_serving::<TicketsServiceServer<TicketsGrpc>>()
        .await;
    health_reporter
        .set_serving::<ModerationServiceServer<ModerationGrpc>>()
        .await;
    health_reporter
        .set_serving::<BlackjackServiceServer<BlackjackGrpc>>()
        .await;
    health_reporter
        .set_serving::<CoudePlayerServiceServer<CoudePlayerGrpc>>()
        .await;
    health_reporter
        .set_serving::<RolePanelsServiceServer<RolePanelsGrpc>>()
        .await;
    health_reporter
        .set_serving::<MembersServiceServer<MembersGrpc>>()
        .await;
    health_reporter
        .set_serving::<SecurityServiceServer<SecurityGrpc>>()
        .await;
    health_reporter
        .set_serving::<AutomodServiceServer<AutomodGrpc>>()
        .await;
    health_reporter
        .set_serving::<VoiceChannelsServiceServer<VoiceChannelsGrpc>>()
        .await;
    health_reporter
        .set_serving::<ImagesServiceServer<ImagesGrpc>>()
        .await;
    health_reporter
        .set_serving::<WelcomeServiceServer<WelcomeGrpc>>()
        .await;
    health_reporter
        .set_serving::<CommunityServiceServer<CommunityGrpc>>()
        .await;

    info!(addr = %bind, "Sentinel gRPC pret (compression Gzip + health)");

    if let Err(e) = Server::builder()
        .add_service(health_service)
        .add_service(progression_svc)
        .add_service(stats_svc)
        .add_service(tickets_svc)
        .add_service(moderation_svc)
        .add_service(blackjack_svc)
        .add_service(coude_svc)
        .add_service(roles_svc)
        .add_service(members_svc)
        .add_service(security_svc)
        .add_service(automod_svc)
        .add_service(voice_svc)
        .add_service(images_svc)
        .add_service(welcome_svc)
        .add_service(community_svc)
        .serve(bind)
        .await
    {
        error!(error = %e, "Erreur serveur gRPC");
    }
}

fn build_auth_interceptor(
    api_key: String,
) -> impl Fn(Request<()>) -> Result<Request<()>, Status> + Clone {
    let expected: Option<Arc<MetadataValue<tonic::metadata::Ascii>>> = if api_key.is_empty() {
        None
    } else {
        match format!("Bearer {api_key}").parse::<MetadataValue<_>>() {
            Ok(v) => Some(Arc::new(v)),
            Err(_) => {
                error!("API_KEY contient des caracteres invalides pour un header gRPC; auth desactivee");
                None
            }
        }
    };

    move |req: Request<()>| {
        let Some(expected) = expected.as_ref() else {
            return Ok(req);
        };
        match req.metadata().get("authorization") {
            Some(token) if token == expected.as_ref() => Ok(req),
            _ => Err(Status::unauthenticated("API key invalide ou manquante")),
        }
    }
}
