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
use tonic::Request;
use tonic::Status;
use tracing::error;
use tracing::info;
use sentinel_proto::automod::v1::automod_service_server::AutomodServiceServer;
use sentinel_proto::blackjack::v1::blackjack_service_server::BlackjackServiceServer;
use sentinel_proto::community::v1::community_service_server::CommunityServiceServer;
use sentinel_proto::export::v1::export_service_server::ExportServiceServer;
use sentinel_proto::coude::v1::coude_bets_service_server::CoudeBetsServiceServer;
use sentinel_proto::coude::v1::coude_combats_service_server::CoudeCombatsServiceServer;
use sentinel_proto::coude::v1::coude_economy_service_server::CoudeEconomyServiceServer;
use sentinel_proto::coude::v1::coude_inventory_service_server::CoudeInventoryServiceServer;
use sentinel_proto::coude::v1::coude_player_service_server::CoudePlayerServiceServer;
use sentinel_proto::coude::v1::coude_social_service_server::CoudeSocialServiceServer;
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

use crate::adapters::inbound::grpc::ai::automod::AutomodGrpc;
use crate::adapters::inbound::grpc::casino::blackjack::BlackjackGrpc;
use crate::adapters::inbound::grpc::community::sponsorships::CommunityGrpc;
use crate::adapters::inbound::grpc::coude::bets::BetsGrpc;
use crate::adapters::inbound::grpc::coude::combats::CombatsGrpc;
use crate::adapters::inbound::grpc::coude::economy::EconomyGrpc;
use crate::adapters::inbound::grpc::coude::inventory::InventoryGrpc;
use crate::adapters::inbound::grpc::coude::players::PlayerGrpc;
use crate::adapters::inbound::grpc::coude::social::SocialGrpc;
use crate::adapters::inbound::grpc::ai::images::ImagesGrpc;
use crate::adapters::inbound::grpc::community::members::MembersGrpc;
use crate::adapters::inbound::grpc::moderation::actions::ModerationGrpc;
use crate::adapters::inbound::grpc::community::progression::ProgressionGrpc;
use crate::adapters::inbound::grpc::community::roles::RolePanelsGrpc;
use crate::adapters::inbound::grpc::audit::security::SecurityGrpc;
use crate::adapters::inbound::grpc::audit::stats::StatsGrpc;
use crate::adapters::inbound::grpc::system::tickets::TicketsGrpc;
use crate::adapters::inbound::grpc::community::voice::VoiceChannelsGrpc;
use crate::adapters::inbound::grpc::system::export::ExportGrpc;
use crate::adapters::inbound::grpc::system::welcome::WelcomeGrpc;
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
    };
    let moderation = ModerationGrpc {
        moderation_uc: state.moderation_uc.clone(),
    };
    let blackjack = BlackjackGrpc {
        svc: state.blackjack_svc.clone(),
        wallet_repo: state.wallet_repo.clone(),
        bot_config_repo: state.bot_config_repo.clone(),
        table_repo: state.blackjack_table_repo.clone(),
        broadcaster: state.broadcaster.clone(),
    };
    let coude = PlayerGrpc {
        players_uc: state.coude_players_uc.clone(),
        wallet_uc: state.wallet_uc.clone(),
    };
    // Phase 7A.opt F.1 — 5 services coude supplementaires.
    let coude_combats = CombatsGrpc {
        uc: state.coude_combats_uc.clone(),
        resolve_batch_uc: state.resolve_betting_batch_uc.clone(),
        expire_batch_uc: state.expire_combats_batch_uc.clone(),
        resolve_now_uc: state.resolve_combat_now_uc.clone(),
    };
    let coude_bets = BetsGrpc {
        uc: state.coude_bets_uc.clone(),
    };
    let coude_economy = EconomyGrpc {
        uc: state.coude_economy_uc.clone(),
    };
    let coude_inventory = InventoryGrpc {
        uc: state.coude_inventory_uc.clone(),
        steal_protections_uc: state.coude_steal_protections_uc.clone(),
        steal_boosts_uc: state.coude_steal_boosts_uc.clone(),
    };
    let coude_social = SocialGrpc {
        uc: state.coude_social_uc.clone(),
        catalog_uc: state.coude_catalog_uc.clone(),
        taunts_uc: state.coude_taunts_uc.clone(),
        heist_uc: state.coude_heist_uc.clone(),
        cashbox_uc: state.coude_cashbox_uc.clone(),
    };
    let roles = RolePanelsGrpc {
        uc: state.role_panels_uc.clone(),
        discord_role_repo: state.discord_role_repo.clone(),
    };
    let welcome = WelcomeGrpc {
        uc: state.welcome_config_uc.clone(),
    };
    let export = ExportGrpc {
        uc: state.export_uc.clone(),
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
    // Phase 7A.opt F.1 — 5 services coude supplementaires.
    let coude_combats_svc = svc!(CoudeCombatsServiceServer, coude_combats);
    let coude_bets_svc = svc!(CoudeBetsServiceServer, coude_bets);
    let coude_economy_svc = svc!(CoudeEconomyServiceServer, coude_economy);
    let coude_inventory_svc = svc!(CoudeInventoryServiceServer, coude_inventory);
    let coude_social_svc = svc!(CoudeSocialServiceServer, coude_social);
    let roles_svc = svc!(RolePanelsServiceServer, roles);
    let members_svc = svc!(MembersServiceServer, members);
    let security_svc = svc!(SecurityServiceServer, security);
    let automod_svc = svc!(AutomodServiceServer, automod);
    let voice_svc = svc!(VoiceChannelsServiceServer, voice);
    let images_svc = svc!(ImagesServiceServer, images);
    // Phase 7A.opt F.3/F.4 — nouveaux services.
    let welcome_svc = svc!(WelcomeServiceServer, welcome);
    let export_svc = svc!(ExportServiceServer, export);
    let community_svc = svc!(CommunityServiceServer, community);

    // tonic-health : expose `grpc.health.v1.Health` + marque chaque service
    // comme SERVING. Permet `grpc_health_probe -addr=:50051` dans le healthcheck.
    let (health_reporter, health_service) = tonic_health::server::health_reporter();
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
        .set_serving::<CoudePlayerServiceServer<PlayerGrpc>>()
        .await;
    health_reporter
        .set_serving::<CoudeCombatsServiceServer<CombatsGrpc>>()
        .await;
    health_reporter
        .set_serving::<CoudeBetsServiceServer<BetsGrpc>>()
        .await;
    health_reporter
        .set_serving::<CoudeEconomyServiceServer<EconomyGrpc>>()
        .await;
    health_reporter
        .set_serving::<CoudeInventoryServiceServer<InventoryGrpc>>()
        .await;
    health_reporter
        .set_serving::<CoudeSocialServiceServer<SocialGrpc>>()
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
        .set_serving::<ExportServiceServer<ExportGrpc>>()
        .await;
    health_reporter
        .set_serving::<CommunityServiceServer<CommunityGrpc>>()
        .await;

    // mTLS optionnel : active si GRPC_TLS_DIR defini en env. Sinon plain HTTP/2
    // (mode dev / migration progressive). Le serveur exige un cert client signe
    // par notre CA interne -> empeche un attaquant qui sniffe le bridge Docker
    // de voler le Bearer token API_KEY.
    //
    // tls_config(self, ...) consomme self -> on doit construire le builder
    // final en une expression avant de chainer add_service.
    let mut server_builder = match sentinel_proto::tls::tls_dir() {
        Some(dir) => match sentinel_proto::tls::server_tls_config(&dir) {
            Ok(cfg) => match Server::builder().tls_config(cfg) {
                Ok(b) => {
                    info!(dir = %dir.display(), "gRPC mTLS active (server + client cert verification)");
                    b
                }
                Err(e) => {
                    error!(error = %e, "Echec config TLS serveur, fallback plain HTTP/2");
                    Server::builder()
                }
            },
            Err(e) => {
                error!(error = %e, "Echec lecture certs TLS, fallback plain HTTP/2");
                Server::builder()
            }
        },
        None => {
            info!("gRPC plain HTTP/2 (GRPC_TLS_DIR non defini)");
            Server::builder()
        }
    };

    info!(addr = %bind, "Sentinel gRPC pret (compression Gzip + health)");

    if let Err(e) = server_builder
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
        .add_service(export_svc)
        .add_service(community_svc)
        .add_service(coude_combats_svc)
        .add_service(coude_bets_svc)
        .add_service(coude_economy_svc)
        .add_service(coude_inventory_svc)
        .add_service(coude_social_svc)
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


#[cfg(test)]
#[path = "tests/server.rs"]
mod tests;
