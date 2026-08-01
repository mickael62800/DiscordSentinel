//! Bootstrap : cablage des services nexus-core avec les adapters Postgres.

use std::sync::Arc;

use nexus_core::application::game::manage_game_servers_service::ManageGameServersService;
use nexus_core::application::game::manage_templates_service::ManageGameTemplatesService;
use nexus_core::application::play_wheel_service::PlayWheelService;
use nexus_core::application::wallet_service::WalletService;
use nexus_core::application::coude_service::CoudeService;
use nexus_core::application::coude_inventory_service::CoudeInventoryService;
use nexus_core::application::coude_insurance_service::CoudeInsuranceService;
use nexus_core::application::coude_steal_service::CoudeStealService;
use nexus_core::application::coude_prime_service::CoudePrimeService;
use nexus_core::application::coude_bet_service::CoudeBetService;
use nexus_core::ports::inbound::coude_profile::CoudeProfileUseCase;
use nexus_core::ports::inbound::coude_profile::CoudeCombatUseCase;
use nexus_core::ports::inbound::coude_inventory::CoudeInventoryUseCase;
use nexus_core::ports::inbound::coude_insurance::CoudeInsuranceUseCase;
use nexus_core::ports::inbound::coude_steal::CoudeStealUseCase;
use nexus_core::ports::inbound::coude_prime::CoudePrimeUseCase;
use nexus_core::ports::inbound::coude_bet::CoudeBetUseCase;
use nexus_core::ports::outbound::coude_inventory_repository::CoudeInventoryRepository;
use nexus_core::ports::outbound::coude_insurance_repository::CoudeInsuranceRepository;
use nexus_core::ports::outbound::coude_steal_repository::CoudeStealRepository;
use nexus_core::ports::outbound::coude_prime_repository::CoudePrimeRepository;
use nexus_core::ports::outbound::coude_bet_repository::CoudeBetRepository;
use nexus_core::ports::outbound::coude_repository::CoudeRepository;
use nexus_core::ports::inbound::game::manage_game_servers::ManageGameServersUseCase;
use nexus_core::ports::inbound::game::manage_game_templates::ManageGameTemplatesUseCase;
use nexus_core::ports::inbound::get_wallet::GetWalletUseCase;
use nexus_core::ports::inbound::play_wheel::PlayWheelUseCase;
use nexus_core::ports::inbound::transfer_coins::TransferCoinsUseCase;
use nexus_core::ports::inbound::wallet_history::GetWalletHistoryUseCase;
use nexus_core::ports::inbound::wallet_leaderboard::GetWalletLeaderboardUseCase;
use nexus_core::ports::outbound::casino::game_repository::GameRepository;
use nexus_core::ports::outbound::events::EventPublisher;
use nexus_core::ports::outbound::game::container_runtime::ContainerRuntime;
use nexus_core::ports::outbound::game::game_audit_repository::GameAuditRepository;
use nexus_core::ports::outbound::game::game_server_repository::GameServerRepository;
use nexus_core::ports::outbound::game::game_session_repository::{
    GameSessionRegistrationRepository, GameTemplateSettingsRepository,
};
use nexus_core::ports::outbound::game::game_template_repository::GameTemplateRepository;
use nexus_core::ports::outbound::game::player_session_repository::PlayerSessionRepository;
use nexus_core::ports::outbound::game::port_allocator::PortAllocator;
use nexus_core::ports::outbound::game::rcon_client::RconClient;
use nexus_core::ports::outbound::system::bot_config_repository::BotConfigRepository;
use sqlx::postgres::PgPoolOptions;

use crate::adapters::outbound::game_runtime::docker_runtime::{
    make_docker_client, DockerContainerRuntime,
};
use crate::adapters::outbound::game_runtime::noop_runtime::NoopContainerRuntime;
use crate::adapters::outbound::game_runtime::rcon_minecraft::MinecraftRconClient;
use crate::adapters::outbound::events::noop_publisher::NoopEventPublisher;
use crate::adapters::outbound::events::redis_publisher::RedisEventPublisher;
use crate::adapters::outbound::game_runtime::redis_port_allocator::RedisPortAllocator;
use crate::adapters::outbound::postgres::casino::game_repository::PgGameRepository;
use crate::adapters::outbound::postgres::coude_repository::PgCoudeRepository;
use crate::adapters::outbound::postgres::coude_inventory_repository::PgCoudeInventoryRepository;
use crate::adapters::outbound::postgres::coude_insurance_repository::PgCoudeInsuranceRepository;
use crate::adapters::outbound::postgres::coude_steal_repository::PgCoudeStealRepository;
use crate::adapters::outbound::postgres::coude_prime_repository::PgCoudePrimeRepository;
use crate::adapters::outbound::postgres::coude_bet_repository::PgCoudeBetRepository;
use crate::adapters::outbound::postgres::game::audit_repository::PgGameAuditRepository;
use crate::adapters::outbound::postgres::game::config_repository::PgGameServerConfigRepository;
use crate::adapters::outbound::postgres::game::player_session_repository::PgPlayerSessionRepository;
use crate::adapters::outbound::postgres::game::server_repository::PgGameServerRepository;
use crate::adapters::outbound::postgres::game::session_repository::{
    PgGameSessionRegistrationRepository, PgGameTemplateSettingsRepository,
};
use crate::adapters::outbound::postgres::game::template_repository::PgGameTemplateRepository;
use crate::adapters::outbound::postgres::system::bot_config_repository::PgBotConfigRepository;
use crate::adapters::outbound::postgres::wallet_repository::PgWalletRepository;
use crate::adapters::outbound::postgres::wheel_repository::PgWheelRepository;

#[derive(Clone)]
pub struct AppState {
    pub play_wheel: Arc<dyn PlayWheelUseCase>,
    pub get_wallet: Arc<dyn GetWalletUseCase>,
    pub transfer_coins: Arc<dyn TransferCoinsUseCase>,
    pub wallet_history: Arc<dyn GetWalletHistoryUseCase>,
    pub wallet_leaderboard: Arc<dyn GetWalletLeaderboardUseCase>,
    pub coude_profile: Arc<dyn CoudeProfileUseCase>,
    pub coude_combat: Arc<dyn CoudeCombatUseCase>,
    pub coude_inventory: Arc<dyn CoudeInventoryUseCase>,
    pub coude_insurance: Arc<dyn CoudeInsuranceUseCase>,
    pub coude_steal: Arc<dyn CoudeStealUseCase>,
    pub coude_prime: Arc<dyn CoudePrimeUseCase>,
    pub coude_bet: Arc<dyn CoudeBetUseCase>,
    // ── Game Portal ──
    pub game_servers_uc: Arc<dyn ManageGameServersUseCase>,
    pub game_templates_uc: Arc<dyn ManageGameTemplatesUseCase>,
    /// Adapters exposes pour les endpoints internes /jobs/* (worker) et
    /// quelques handlers qui accedent directement aux repos.
    pub game_server_repo: Arc<dyn GameServerRepository>,
    pub game_template_repo: Arc<dyn GameTemplateRepository>,
    pub game_template_settings_repo: Arc<dyn GameTemplateSettingsRepository>,
    pub game_session_reg_repo: Arc<dyn GameSessionRegistrationRepository>,
    pub game_audit_repo: Arc<dyn GameAuditRepository>,
    pub game_session_repo: Arc<dyn PlayerSessionRepository>,
    pub game_container_runtime: Arc<dyn ContainerRuntime>,
    pub game_rcon_client: Arc<dyn RconClient>,
    pub game_port_allocator: Arc<dyn PortAllocator>,
    pub bot_config_repo: Arc<dyn BotConfigRepository>,
    /// Catalogue des jeux mentionnables (games/panels).
    pub game_repo: Arc<dyn GameRepository>,
    /// Publie les evenements consommes par le bot (salons de session).
    pub events: Arc<dyn EventPublisher>,
    /// Si Some, toutes les routes /api exigent `Authorization: Bearer <key>`.
    pub api_key: Option<String>,
    /// Serveur Discord unique servi par cette installation.
    ///
    /// `None` = verrou desactive. Voir `single_guild` cote HTTP : Nexus
    /// expose sa propre surface, il lui faut donc son propre verrou —
    /// celui de sentinel-api ne le protege pas.
    pub guild_id: Option<String>,
}

/// Connecte le pool Postgres (NEXUS_DATABASE_URL), applique les migrations
/// `nexus-api/migrations/`, et construit l'AppState.
///
/// Env game-portal :
///   - NEXUS_GAME_RUNTIME = docker | noop (defaut : noop). En mode docker,
///     fallback automatique sur noop si le socket Docker est indisponible.
///   - REDIS_URL (defaut redis://127.0.0.1:6379) : allocation atomique des
///     ports via SETNX.
pub async fn build_state() -> Result<AppState, Box<dyn std::error::Error>> {
    let db_url = std::env::var("NEXUS_DATABASE_URL")
        .map_err(|_| "NEXUS_DATABASE_URL manquante (ex: postgres://user:pass@host/nexus)")?;

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;

    // Declare en tete : plusieurs services le lisent pour connaitre
    // l'equilibre du jeu (taux de vol, delais, bornes de transfert).
    let bot_config_repo: Arc<dyn BotConfigRepository> =
        Arc::new(PgBotConfigRepository::new(pool.clone()));

    let wheel_repo = Arc::new(PgWheelRepository::new(pool.clone()));
    let wallet_repo = Arc::new(PgWalletRepository::new(pool.clone()));
    let service = Arc::new(PlayWheelService::new(wheel_repo, wallet_repo.clone(), bot_config_repo.clone()));
    let wallet_service = Arc::new(WalletService::new(wallet_repo, bot_config_repo.clone()));
    let coude_repo: Arc<dyn CoudeRepository> = Arc::new(PgCoudeRepository::new(pool.clone()));
    let coude_profile: Arc<dyn CoudeProfileUseCase> = Arc::new(CoudeService::new(coude_repo));
    let coude_combat: Arc<dyn CoudeCombatUseCase> = Arc::new(CoudeService::new(Arc::new(PgCoudeRepository::new(pool.clone()))));
    let coude_inventory_repo: Arc<dyn CoudeInventoryRepository> = Arc::new(PgCoudeInventoryRepository::new(pool.clone()));
    let coude_inventory: Arc<dyn CoudeInventoryUseCase> = Arc::new(CoudeInventoryService::new(coude_inventory_repo));
    let coude_insurance_repo: Arc<dyn CoudeInsuranceRepository> = Arc::new(PgCoudeInsuranceRepository::new(pool.clone()));
    let coude_insurance: Arc<dyn CoudeInsuranceUseCase> = Arc::new(CoudeInsuranceService::new(coude_insurance_repo, bot_config_repo.clone()));
    let coude_steal_repo: Arc<dyn CoudeStealRepository> = Arc::new(PgCoudeStealRepository::new(pool.clone()));
    let coude_steal: Arc<dyn CoudeStealUseCase> = Arc::new(CoudeStealService::new(coude_steal_repo, bot_config_repo.clone()));
    let coude_prime_repo: Arc<dyn CoudePrimeRepository> = Arc::new(PgCoudePrimeRepository::new(pool.clone()));
    let coude_prime: Arc<dyn CoudePrimeUseCase> = Arc::new(CoudePrimeService::new(coude_prime_repo, bot_config_repo.clone()));
    let coude_bet_repo: Arc<dyn CoudeBetRepository> = Arc::new(PgCoudeBetRepository::new(pool.clone()));
    let coude_bet: Arc<dyn CoudeBetUseCase> = Arc::new(CoudeBetService::new(coude_bet_repo, bot_config_repo.clone()));

    // ── Game Portal : repos Postgres ──
    let game_server_repo: Arc<dyn GameServerRepository> =
        Arc::new(PgGameServerRepository::new(pool.clone()));
    let game_template_repo: Arc<dyn GameTemplateRepository> =
        Arc::new(PgGameTemplateRepository::new(pool.clone()));
    let game_config_repo = Arc::new(PgGameServerConfigRepository::new(pool.clone()));
    let game_audit_repo: Arc<dyn GameAuditRepository> =
        Arc::new(PgGameAuditRepository::new(pool.clone()));
    let game_session_repo: Arc<dyn PlayerSessionRepository> =
        Arc::new(PgPlayerSessionRepository::new(pool.clone()));
    let game_template_settings_repo: Arc<dyn GameTemplateSettingsRepository> =
        Arc::new(PgGameTemplateSettingsRepository::new(pool.clone()));
    let game_session_reg_repo: Arc<dyn GameSessionRegistrationRepository> =
        Arc::new(PgGameSessionRegistrationRepository::new(pool.clone()));
    let game_repo: Arc<dyn GameRepository> = Arc::new(PgGameRepository::new(pool.clone()));

    // ── Game Portal : runtime container (docker | noop) ──
    // NEXUS_GAME_RUNTIME=docker tente le socket Docker ; tout autre valeur
    // (ou l'absence de la variable, ou un socket indisponible) => noop, qui
    // repond Internal sur les operations lifecycle mais laisse le listing
    // et la config fonctionner.
    let runtime_mode = std::env::var("NEXUS_GAME_RUNTIME").unwrap_or_else(|_| "noop".into());
    let container_runtime: Arc<dyn ContainerRuntime> = if runtime_mode == "docker" {
        match make_docker_client() {
            Ok(d) => Arc::new(DockerContainerRuntime::new(d)),
            Err(e) => {
                tracing::warn!(error = %e, "Docker socket indisponible — Game Portal lifecycle inactif (noop)");
                Arc::new(NoopContainerRuntime)
            }
        }
    } else {
        tracing::info!("NEXUS_GAME_RUNTIME={runtime_mode} — runtime container noop");
        Arc::new(NoopContainerRuntime)
    };

    let rcon_client: Arc<dyn RconClient> = Arc::new(MinecraftRconClient::new());

    // Le client redis ne se connecte pas a l'open (lazy) : une URL par defaut
    // ne coute rien tant que l'allocation de port n'est pas sollicitee.
    let redis_url =
        std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".into());
    let redis_client = redis::Client::open(redis_url.as_str())
        .map_err(|e| format!("REDIS_URL invalide ({redis_url}): {e}"))?;
    let port_allocator: Arc<dyn PortAllocator> = Arc::new(RedisPortAllocator::new(redis_client));

    // Bus d'evenements vers le bot. REDIS_URL explicitement definie => stream
    // Redis ; sinon publieur inerte (le bot ne creera pas les salons).
    let events: Arc<dyn EventPublisher> = match std::env::var("REDIS_URL") {
        Ok(url) if !url.is_empty() => match RedisEventPublisher::new(&url) {
            Ok(p) => Arc::new(p),
            Err(e) => {
                tracing::warn!(error = %e, "REDIS_URL invalide — events desactives");
                Arc::new(NoopEventPublisher)
            }
        },
        _ => {
            tracing::warn!(
                "REDIS_URL absente — events desactives : le bot ne creera pas \
                 les salons de session game-portal"
            );
            Arc::new(NoopEventPublisher)
        }
    };

    // ── Game Portal : use cases ──
    let game_servers_uc: Arc<dyn ManageGameServersUseCase> = Arc::new(ManageGameServersService {
        server_repo: game_server_repo.clone(),
        template_repo: game_template_repo.clone(),
        config_repo: game_config_repo,
        audit_repo: game_audit_repo.clone(),
        container_runtime: container_runtime.clone(),
        rcon_client: rcon_client.clone(),
        port_allocator: port_allocator.clone(),
        bot_config: bot_config_repo.clone(),
    });
    let game_templates_uc: Arc<dyn ManageGameTemplatesUseCase> = Arc::new(
        ManageGameTemplatesService::new(game_template_repo.clone(), bot_config_repo.clone()),
    );

    let api_key = std::env::var("NEXUS_API_KEY").ok().filter(|k| !k.is_empty());
    // Meme variable que sentinel-api et que le conteneur web : une seule
    // source de verite pour « de quel serveur parle cette installation ».
    let guild_id = std::env::var("PUBLIC_GUILD_ID")
        .or_else(|_| std::env::var("GUILD_ID"))
        .ok()
        .map(|g| g.trim().to_string())
        .filter(|g| !g.is_empty());
    match &guild_id {
        Some(g) => tracing::info!(guild_id = %g, "mono-serveur : verrou actif"),
        None => tracing::warn!(
            "PUBLIC_GUILD_ID absente — toutes les guildes sont acceptees"
        ),
    }
    if api_key.is_none() {
        tracing::warn!("NEXUS_API_KEY absente — API SANS auth (dev uniquement)");
    }

    Ok(AppState {
        play_wheel: service,
        get_wallet: wallet_service.clone(),
        transfer_coins: wallet_service.clone(),
        wallet_history: wallet_service.clone(),
        wallet_leaderboard: wallet_service,
        coude_profile,
        coude_combat,
        coude_inventory,
        coude_insurance,
        coude_steal,
        coude_prime,
        coude_bet,
        game_servers_uc,
        game_templates_uc,
        game_server_repo,
        game_template_repo,
        game_template_settings_repo,
        game_session_reg_repo,
        game_audit_repo,
        game_session_repo,
        game_container_runtime: container_runtime,
        game_rcon_client: rcon_client,
        game_port_allocator: port_allocator,
        bot_config_repo,
        game_repo,
        events,
        api_key,
        guild_id,
    })
}
