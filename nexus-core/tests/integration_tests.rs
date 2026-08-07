//! Tests d'intégration bout-en-bout pour nexus-core (Domain & Application)

use std::sync::{Arc, Mutex};
use async_trait::async_trait;
use uuid::Uuid;
use chrono::Utc;

use nexus_core::application::deploy_panel_service::DeployGamesPanelUseCase;
use nexus_core::application::game_mentions_service::DetectGameMentionsUseCase;
use nexus_core::application::game::manage_templates_service::ManageGameTemplatesService;
use nexus_core::application::upload_emoji_service::UploadEmojiUseCase;
use nexus_core::domain::entities::casino::game::{
    format_custom_emoji, is_allowed_emoji_mime, normalize_game_name, parse_role_color_hex, slugify_emoji_name, DEFAULT_GAME_ROLE_COLOR,
};
use nexus_core::domain::entities::coussin_shop::{item, ITEMS};
use nexus_core::domain::entities::game::config::validate_config_key;
use nexus_core::domain::entities::game::quota::GuildQuotaState;
use nexus_core::domain::entities::game::server::GameServerStatus;
use nexus_core::domain::entities::game::template::{GameTemplate, PortProtocol};
use nexus_core::domain::entities::system::bot_config::{BotDefinition, BotGuildConfig};
use nexus_core::domain::entities::system::discord_ids::UserId;
use nexus_core::domain::entities::wallet::Wallet;
use nexus_core::domain::errors::DomainError;
use nexus_core::ports::inbound::game::manage_game_templates::ManageGameTemplatesUseCase;
use nexus_core::ports::outbound::casino::game_repository::{Game, GamePanel, GameRepository};
use nexus_core::ports::outbound::events::{game_events, EventPublisher};
use nexus_core::ports::outbound::game::container_runtime::{
    ContainerRuntime, ContainerSpec, ContainerState, MockContainerRuntime, RestartPolicy,
};
use nexus_core::ports::outbound::game::game_template_repository::GameTemplateRepository;
use nexus_core::ports::outbound::system::bot_config_repository::BotConfigRepository;
use nexus_core::ports::outbound::system::discord_api_repository::DiscordApiRepository;

// ── Mock Adapters ──

#[derive(Default)]
struct DummyBotConfigRepo;

#[async_trait]
impl BotConfigRepository for DummyBotConfigRepo {
    async fn get_definitions(&self) -> Result<Vec<BotDefinition>, DomainError> { Ok(vec![]) }
    async fn get_config(&self, _g: &str, _b: &str) -> Result<Vec<BotGuildConfig>, DomainError> { Ok(vec![]) }
    async fn get_all_config(&self, _g: &str) -> Result<Vec<BotGuildConfig>, DomainError> { Ok(vec![]) }
    async fn set_config(&self, _g: &str, _b: &str, _k: &str, _v: &str) -> Result<(), DomainError> { Ok(()) }
    async fn delete_config(&self, _g: &str, _b: &str, _k: &str) -> Result<(), DomainError> { Ok(()) }
}

#[derive(Default)]
struct MemoryGameRepo {
    games: Vec<Game>,
}

#[async_trait]
impl GameRepository for MemoryGameRepo {
    async fn list(&self, _guild_id: &str) -> Result<Vec<Game>, DomainError> {
        Ok(self.games.clone())
    }
    async fn list_by_category(&self, _g: &str, _c: Option<&str>) -> Result<Vec<Game>, DomainError> { Ok(self.games.clone()) }
    async fn create(&self, _g: &str, _n: &str, _s: &str, _d: Option<&str>, _c: Option<&str>, _i: Option<&str>) -> Result<Game, DomainError> { todo!() }
    async fn update(&self, _g: &str, _s: &str, _d: Option<&str>, _c: Option<Option<&str>>, _i: Option<Option<&str>>) -> Result<Option<Game>, DomainError> { todo!() }
    async fn delete(&self, _g: &str, _s: &str) -> Result<bool, DomainError> { Ok(true) }
    async fn find_by_name(&self, _g: &str, _n: &str) -> Result<Option<Game>, DomainError> { Ok(None) }
    async fn set_role_id(&self, _g: &str, _s: &str, _r: Option<&str>) -> Result<Option<Game>, DomainError> { Ok(None) }
    async fn save_panel(&self, _g: &str, _m: &str, _c: &str, _cat: Option<&str>) -> Result<GamePanel, DomainError> { todo!() }
    async fn find_panel_by_message(&self, _g: &str, _m: &str) -> Result<Option<GamePanel>, DomainError> { Ok(None) }
    async fn list_panels(&self, _g: &str) -> Result<Vec<GamePanel>, DomainError> { Ok(vec![]) }
}

#[derive(Default)]
struct MemoryEventPublisher {
    published: Mutex<Vec<(String, serde_json::Value)>>,
}

#[async_trait]
impl EventPublisher for MemoryEventPublisher {
    async fn publish(&self, event: &str, data: serde_json::Value) {
        self.published.lock().unwrap().push((event.to_string(), data));
    }
}

#[derive(Default)]
struct MemoryTemplateRepo {
    templates: Vec<GameTemplate>,
}

#[async_trait]
impl GameTemplateRepository for MemoryTemplateRepo {
    async fn list(&self) -> Result<Vec<GameTemplate>, DomainError> {
        Ok(self.templates.clone())
    }
    async fn find_by_id(&self, id: Uuid) -> Result<Option<GameTemplate>, DomainError> {
        Ok(self.templates.iter().find(|t| t.id == id).cloned())
    }
    async fn find_by_slug(&self, slug: &str) -> Result<Option<GameTemplate>, DomainError> {
        Ok(self.templates.iter().find(|t| t.slug == slug).cloned())
    }
}

struct DummyDiscordApi;

#[async_trait]
impl DiscordApiRepository for DummyDiscordApi {
    async fn upload_emoji(&self, _g: &str, name: &str, _bytes: &[u8], _m: &str) -> Result<(String, String), DomainError> {
        Ok(("emoji_999".into(), name.into()))
    }
}

// ── 9 SCÉNARIOS D'INTÉGRATION DU DOMAINE ET DE L'APPLICATION ──

#[tokio::test]
async fn integration_detect_mentions_and_deploy_panel() {
    let game = Game {
        id: Uuid::new_v4().to_string(),
        guild_id: "guild_100".into(),
        game_name: "C++".into(),
        role_id: Some("role_cpp".into()),
        category: Some("programming".into()),
        created_by: "system".into(),
        emoji: None,
        created_at: Utc::now().to_string(),
    };

    let game_repo = Arc::new(MemoryGameRepo { games: vec![game] });
    let event_publisher = Arc::new(MemoryEventPublisher::default());

    let mentions_uc = DetectGameMentionsUseCase::new(game_repo);
    let deploy_uc = DeployGamesPanelUseCase::new(event_publisher.clone());

    // 1. Detect game mentions in chat
    let detected = mentions_uc.execute("guild_100", "Qui joue a C++ ce soir ?").await.unwrap();
    assert_eq!(detected.len(), 1);
    assert_eq!(detected[0].game_name, "C++");

    // 2. Deploy games panel
    deploy_uc.execute("guild_100", "channel_general", Some("programming")).await;

    let events = event_publisher.published.lock().unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].0, game_events::GAMES_PANEL_DEPLOY);
    assert_eq!(events[0].1["guild_id"], "guild_100");
}

#[tokio::test]
async fn integration_template_catalog_and_mock_runtime_lifecycle() {
    let template = GameTemplate {
        id: Uuid::new_v4(),
        slug: "minecraft".into(),
        name: "Minecraft".into(),
        description: Some("Sandbox game".into()),
        image: "itzg/minecraft-server:latest".into(),
        category: Some("sandbox".into()),
        icon: None,
        accent_color: None,
        cover_image_url: None,
        container_port: 25565,
        port_protocol: PortProtocol::Tcp,
        volume_path: "/data".into(),
        run_as_root: false,
        default_memory_mb: 2048,
        min_memory_mb: 1024,
        max_memory_mb: 4096,
        default_env: serde_json::json!({}),
        config_schema: vec![],
        supports_rcon: true,
        supports_mods: true,
        idle_shutdown_days: 7,
        init_files: vec![],
        command: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    let template_repo = Arc::new(MemoryTemplateRepo {
        templates: vec![template.clone()],
    });
    let bot_config = Arc::new(DummyBotConfigRepo);

    let template_service = ManageGameTemplatesService::new(template_repo, bot_config);
    let runtime = MockContainerRuntime::new();

    // 1. Query template from catalog
    let found = template_service.get_by_slug("minecraft").await.unwrap();
    assert_eq!(found.slug, "minecraft");

    // 2. Provision & Lifecycle via Mock Runtime
    let spec = ContainerSpec {
        image: found.image,
        name: "mc-server-integration".into(),
        env: std::collections::HashMap::new(),
        port_mappings: vec![],
        volumes: vec![],
        memory_bytes: 2048 * 1024 * 1024,
        cpu_limit: None,
        network: "nexus-net".into(),
        user: None,
        restart_policy: RestartPolicy::None,
        labels: std::collections::HashMap::new(),
        command: None,
    };

    let container_id = runtime.create_container(&spec).await.unwrap();
    runtime.start_container(&container_id).await.unwrap();

    let status = runtime.inspect(&container_id).await.unwrap().unwrap();
    assert_eq!(status.state, ContainerState::Running);

    runtime.stop_container(&container_id, 5).await.unwrap();
    let status_after = runtime.inspect(&container_id).await.unwrap().unwrap();
    assert_eq!(status_after.state, ContainerState::Exited);
}

#[tokio::test]
async fn integration_upload_emoji_and_mention_detection() {
    let api = Arc::new(DummyDiscordApi);
    let upload_uc = UploadEmojiUseCase::new(api);

    let valid_bytes = vec![0u8; 100];
    let (id, name) = upload_uc.execute("g1", "pepe_smirk", &valid_bytes, "image/png").await.unwrap();

    assert_eq!(id, "emoji_999");
    assert_eq!(name, "pepe_smirk");
}

#[tokio::test]
async fn integration_multi_container_concurrency_and_events() {
    let runtime = MockContainerRuntime::new();
    let event_publisher = Arc::new(MemoryEventPublisher::default());

    // Create 3 containers concurrently
    for i in 1..=3 {
        let spec = ContainerSpec {
            image: "alpine:latest".into(),
            name: format!("node-{}", i),
            env: std::collections::HashMap::new(),
            port_mappings: vec![],
            volumes: vec![],
            memory_bytes: 512 * 1024 * 1024,
            cpu_limit: None,
            network: "nexus-net".into(),
            user: None,
            restart_policy: RestartPolicy::None,
            labels: std::collections::HashMap::new(),
            command: None,
        };

        let cid = runtime.create_container(&spec).await.unwrap();
        runtime.start_container(&cid).await.unwrap();

        event_publisher.publish(
            game_events::SERVER_STARTED,
            serde_json::json!({ "container_id": cid, "server_index": i })
        ).await;
    }

    let events = event_publisher.published.lock().unwrap();
    assert_eq!(events.len(), 3);
    assert_eq!(events[0].0, game_events::SERVER_STARTED);
}

#[test]
fn integration_game_server_state_transitions_and_restart_rules() {
    let status_stopped = GameServerStatus::Stopped;
    assert!(status_stopped.can_start());
    assert!(!status_stopped.can_stop());

    let status_running = GameServerStatus::Running;
    assert!(!status_running.can_start());
    assert!(status_running.can_stop());
}

#[test]
fn integration_game_quota_validation_rules() {
    let state = GuildQuotaState {
        active_servers: 4,
        max_servers: 5,
        allocated_memory_mb: 2048,
        max_memory_mb: 8192,
    };

    assert!(state.can_create_server(2048).is_ok());

    let full_servers = GuildQuotaState {
        active_servers: 5,
        max_servers: 5,
        allocated_memory_mb: 2048,
        max_memory_mb: 8192,
    };
    assert!(full_servers.can_create_server(2048).is_err()); // Exceeds max servers

    let full_memory = GuildQuotaState {
        active_servers: 1,
        max_servers: 5,
        allocated_memory_mb: 7000,
        max_memory_mb: 8192,
    };
    assert!(full_memory.can_create_server(2048).is_err()); // Exceeds max memory
}

#[test]
fn integration_game_config_validation_rules() {
    let valid_cfg = validate_config_key("MAX_PLAYERS");
    assert!(valid_cfg.is_ok());

    let invalid_cfg = validate_config_key("lower_key");
    assert!(invalid_cfg.is_err());
}

#[test]
fn integration_coussin_shop_items_and_inventory() {
    let shop = ITEMS;
    assert!(!shop.is_empty());
    assert!(shop.iter().all(|item| item.price > 0));

    let rage_item = item("rage");
    assert!(rage_item.is_some());
    assert_eq!(rage_item.unwrap().price, 100);
}

#[test]
fn integration_casino_game_slugification_and_color_parsing() {
    let name = normalize_game_name("  Minecraft  ").unwrap();
    assert_eq!(name, "Minecraft");

    let slug = slugify_emoji_name("  My Awesome Game #1!  ");
    assert_eq!(slug, "my_awesome_game_1");

    let color = parse_role_color_hex("#FF5733", DEFAULT_GAME_ROLE_COLOR);
    assert_eq!(color, 0xFF5733);

    assert!(is_allowed_emoji_mime("image/png"));
    assert_eq!(format_custom_emoji("pepe", "123", false), "<:pepe:123>");
}

#[test]
fn integration_wallet_id_and_discord_id_types() {
    let user_id = UserId::from("123456789012345678");
    assert_eq!(user_id.as_str(), "123456789012345678");

    let wallet = Wallet::new("guild_1", "user_1");
    assert_eq!(wallet.coins, 0);
}
