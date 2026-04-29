//! Bootstrap : construction de l'etat applicatif (connexions infra + DI).
//!
//! Extrait de `main.rs` pour garder ce dernier concentre sur bind/serve.
//! Chaque fonction publique represente une phase de l'initialisation :
//! - `connect_pg` : pool PostgreSQL avec compat pgbouncer transaction pooling.
//! - `connect_redis` : client Redis + purge cache + liveness check.
//! - `build_inference` : services ONNX (vision + text + rate limiter).
//! - `build_broadcaster` : EventBroadcaster connecte a Redis pub/sub.
//! - `build_app_state` : assemble tous les repos/services dans l'AppState.

use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use sqlx::postgres::PgConnectOptions;
use sqlx::postgres::PgPoolOptions;
use tracing::error;
use tracing::info;
use crate::adapters::inbound::http::state::AppState;
use crate::adapters::inbound::ws::broadcaster::EventBroadcaster;
use crate::adapters::outbound::batching::batch_writer::BatchWriterConfig;
use crate::adapters::outbound::batching::audit_log_batcher::BatchedPgAuditLogRepository;
use crate::adapters::outbound::batching::log_batcher::BatchedPgLogRepository;
use crate::adapters::outbound::job_client::JobClient;
use crate::adapters::outbound::postgres::audit::analytics_repository::PgAnalyticsRepository;
use crate::adapters::outbound::postgres::casino::blackjack_repository::PgBlackjackRepository;
use crate::adapters::outbound::postgres::casino::blackjack_table_repository::PgBlackjackTableRepository;
use crate::adapters::outbound::postgres::system::bot_config_repository::PgBotConfigRepository;
use crate::adapters::outbound::postgres::community::conduct_repository::PgConductRepository;
use crate::adapters::outbound::postgres::coude::bet_repository::PgBetRepository;
use crate::adapters::outbound::postgres::coude::cashbox_repository::PgCashboxRepository;
use crate::adapters::outbound::postgres::coude::combat_repository::PgCombatRepository;
use crate::adapters::outbound::postgres::coude::bounty_repository::PgBountyRepository;
use crate::adapters::outbound::postgres::coude::coalition_repository::PgCoalitionRepository;
use crate::adapters::outbound::postgres::coude::curses_repository::PgCursesRepository;
use crate::adapters::outbound::postgres::coude::economy_repository::PgEconomyRepository;
use crate::adapters::outbound::postgres::coude::flavor_templates_repository::PgFlavorTemplatesRepository;
use crate::adapters::outbound::postgres::coude::heist_repository::PgHeistRepository;
use crate::adapters::outbound::postgres::coude::inventory_repository::PgInventoryRepository;
use crate::adapters::outbound::postgres::coude::refusal_count_repository::PgRefusalCountRepository;
use crate::adapters::outbound::postgres::coude::safety_net_repository::PgSafetyNetRepository;
use crate::adapters::outbound::postgres::coude::tout_ou_rien_repository::PgToutOuRienRepository;
use crate::adapters::outbound::postgres::coude::ultimate_repository::PgUltimateRepository;
use crate::adapters::outbound::postgres::coude::vendetta_repository::PgVendettaRepository;
use crate::adapters::outbound::postgres::coude::player_repository::PgPlayerRepository;
use crate::adapters::outbound::postgres::coude::social_repository::PgSocialRepository;
use crate::adapters::outbound::postgres::coude::steal_boost_repository::PgStealBoostRepository;
use crate::adapters::outbound::postgres::coude::steal_protection_repository::PgStealProtectionRepository;
use crate::adapters::outbound::postgres::coude::taunts_repository::PgTauntsRepository;
use crate::adapters::outbound::postgres::community::daily_activity_repository::PgDailyActivityRepository;
use crate::adapters::outbound::postgres::community::discord_role_repository::PgDiscordRoleRepository;
use crate::adapters::outbound::postgres::moderation::evidence_repository::PgEvidenceRepository;
use crate::adapters::outbound::postgres::casino::game_repository::PgGameRepository;
use crate::adapters::outbound::postgres::system::guild_repository::PgGuildRepository;
use crate::adapters::outbound::postgres::moderation::infraction_repository::PgInfractionRepository;
use crate::adapters::outbound::postgres::community::level_repository::PgLevelRepository;
use crate::adapters::outbound::postgres::community::member_repository::PgMemberRepository;
use crate::adapters::outbound::postgres::moderation::moderation_repository::PgModerationRepository;
use crate::adapters::outbound::postgres::audit::modstats_repository::PgModstatsRepository;
use crate::adapters::outbound::postgres::moderation::notes_repository::PgNotesRepository;
use crate::adapters::outbound::postgres::moderation::pending_action_repository::PgPendingActionRepository;
use crate::adapters::outbound::postgres::moderation::reminder_repository::PgReminderRepository;
use crate::adapters::outbound::postgres::moderation::review_repository::PgReviewRepository;
use crate::adapters::outbound::postgres::community::role_panel_repository::PgRolePanelRepository;
use crate::adapters::outbound::postgres::moderation::rule_repository::PgRuleRepository;
use crate::adapters::outbound::postgres::audit::security_event_repository::PgSecurityEventRepository;
use crate::adapters::outbound::postgres::coude::sponsorship_repository::PgSponsorshipRepository;
use crate::adapters::outbound::postgres::audit::stats_repository::PgStatsRepository;
use crate::adapters::outbound::postgres::moderation::strike_repository::PgStrikeRepository;
use crate::adapters::outbound::postgres::community::temp_role_repository::PgTempRoleRepository;
use crate::adapters::outbound::postgres::system::ticket_repository::PgTicketRepository;
use crate::adapters::outbound::postgres::audit::user_activity_repository::PgUserActivityRepository;
use crate::adapters::outbound::postgres::community::voice_channel_repository::PgVoiceChannelRepository;
use crate::adapters::outbound::postgres::casino::wallet_repository::PgWalletRepository;
use crate::adapters::outbound::postgres::audit::watched_user_repository::PgWatchedUserRepository;
use crate::adapters::outbound::postgres::community::welcome_config_repository::PgWelcomeConfigRepository;
use crate::adapters::outbound::redis_cache::RedisCache;
use crate::application::ai::analyze_image_service::AnalyzeImageService;
use crate::application::ai::analyze_message_service::AnalyzeMessageService;
use crate::application::casino::blackjack_service::BlackjackService;
use crate::application::coude::combat::expire_batch::ExpireCombatsBatchService;
use crate::application::system::export_service::ExportService;
use crate::application::audit::manage_audit_logs_service::ManageAuditLogsService;
use crate::application::community::manage_conduct_service::ManageConductService;
use crate::application::coude::bet::manage::ManageCoudeBetsService;
use crate::application::coude::manage_cashbox_service::ManageCoudeCashboxService;
use crate::application::coude::manage_catalog_service::ManageCoudeCatalogService;
use crate::application::coude::combat::manage::ManageCoudeCombatsService;
use crate::application::coude::manage_curses_service::ManageCoudeCursesService;
use crate::application::coude::manage_economy_service::ManageCoudeEconomyService;
use crate::application::coude::manage_heist_service::ManageCoudeHeistService;
use crate::application::coude::manage_inventory_service::ManageCoudeInventoryService;
use crate::application::coude::manage_safety_net_service::ManageCoudeSafetyNetService;
use crate::application::coude::manage_vendetta_service::ManageCoudeVendettaService;
use crate::application::coude::manage_players_service::ManageCoudePlayersService;
use crate::application::coude::manage_social_service::ManageCoudeSocialService;
use crate::application::coude::steal::manage_boosts::ManageCoudeStealBoostsService;
use crate::application::coude::steal::manage_protections::ManageCoudeStealProtectionsService;
use crate::application::coude::manage_taunts_service::ManageCoudeTauntsService;
use crate::application::casino::manage_wallet_service::ManageWalletService;
use crate::application::moderation::manage_infractions_service::ManageInfractionsService;
use crate::application::community::manage_levels_service::ManageLevelsService;
use crate::application::community::manage_members_service::ManageMembersService;
use crate::application::moderation::manage_moderation_service::ManageModerationService;
use crate::application::moderation::manage_notes_service::ManageNotesService;
use crate::application::moderation::manage_reminders_service::ManageRemindersService;
use crate::application::community::manage_role_panels_service::ManageRolePanelsService;
use crate::application::moderation::manage_rules_service::ManageRulesService;
use crate::application::audit::manage_security_service::ManageSecurityService;
use crate::application::audit::manage_stats_service::ManageStatsService;
use crate::application::moderation::manage_strikes_service::ManageStrikesService;
use crate::application::system::manage_tickets_service::ManageTicketsService;
use crate::application::community::voice_channels::ManageVoiceChannelsService;
use crate::application::audit::manage_watched_users_service::ManageWatchedUsersService;
use crate::application::coude::play_tout_ou_rien_service::PlayToutOuRienService;
use crate::application::coude::play_travaux_service::PlayTravauxService;
use crate::application::coude::bet::resolve_batch::ResolveBettingBatchService;
use crate::application::coude::combat::resolve_now::ResolveCombatNowService;
use crate::application::coude::steal::roll::RollStealService;
use crate::config::AppConfig;
use crate::adapters::outbound::discord_api::DiscordApiService;
use crate::adapters::outbound::inference_service::InferenceService;
use crate::adapters::outbound::text_tokenizer::TextTokenizer;
use crate::domain::services::ai::inference_limiter::InferenceRateLimiter;

/// Connecte a PostgreSQL avec pgbouncer transaction pooling compat.
///
/// Phase 7A opt C.1 : compat pgbouncer transaction pooling.
///
/// `.statement_cache_capacity(0)` : pgbouncer en transaction pooling ne
///   garantit pas que deux requetes consecutives arrivent sur la meme
///   backend connection, donc les prepared statements caches par sqlx
///   (via son cache LRU par defaut) peuvent etre invalides silencieusement
///   et cela declenche `query_wait_timeout` (code 08P01). Desactiver le
///   cache resout le probleme — cout CPU marginal.
///
/// `.application_name("sentinel-api")` : permet a pgbouncer/postgres de
///   tracer les connexions par service (visible dans `pg_stat_activity`).
pub async fn connect_pg(config: &AppConfig) -> sqlx::PgPool {
    let connect_opts = PgConnectOptions::from_str(&config.database_url)
        .expect("DATABASE_URL invalide")
        .statement_cache_capacity(0)
        .application_name("sentinel-api");

    PgPoolOptions::new()
        .max_connections(20)
        .min_connections(2)
        .acquire_timeout(Duration::from_secs(10))
        .test_before_acquire(false)
        .connect_with(connect_opts)
        .await
        .expect("Impossible de se connecter a PostgreSQL")
}

/// Ouvre le client Redis + purge cache bot:definitions + check liveness.
///
/// Purger le cache des definitions de bots apres migration : les migrations
/// peuvent modifier les config_schema (ex: 113 = ajout des 4 salons audit),
/// mais le cache Redis bot:definitions a un TTL d'1h. Sans ca, les changements
/// n'apparaissent qu'apres expiration du TTL.
pub async fn connect_redis(config: &AppConfig) -> redis::Client {
    let redis_client =
        redis::Client::open(config.redis_url.as_str()).expect("URL Redis invalide");

    if let Ok(mut conn) = redis_client.get_multiplexed_async_connection().await {
        use redis::AsyncCommands;
        let _: Result<(), _> = conn.del::<_, ()>("bot:definitions").await;
        info!("Cache Redis bot:definitions purge (post-migration)");
    }

    // Verifier la connexion Redis au demarrage
    match redis_client.get_multiplexed_async_connection().await {
        Ok(_) => info!("Redis connecte"),
        Err(e) => error!("Redis indisponible au demarrage: {e} — le cache sera desactive"),
    }

    redis_client
}

/// Construit le service d'inference ONNX (vision + text tokenizer + rate limiter).
pub fn build_inference() -> (
    Arc<InferenceService>,
    Arc<TextTokenizer>,
    Arc<InferenceRateLimiter>,
) {
    let vision_model_path = std::env::var("VISION_MODEL_PATH").ok();
    let text_model_path = std::env::var("TEXT_MODEL_PATH").ok();
    let tokenizer_path = std::env::var("TEXT_TOKENIZER_PATH").ok();
    let text_max_length: usize = std::env::var("TEXT_MAX_LENGTH")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(512);

    let inference = Arc::new(InferenceService::new(
        vision_model_path.as_deref(),
        text_model_path.as_deref(),
    ));
    let tokenizer = Arc::new(TextTokenizer::new(
        tokenizer_path.as_deref(),
        text_max_length,
    ));

    let inference_max_concurrent: usize = std::env::var("INFERENCE_MAX_CONCURRENT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(4);
    let inference_max_per_sec: u64 = std::env::var("INFERENCE_MAX_PER_SEC")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(20);

    let inference_limiter = Arc::new(InferenceRateLimiter::new(
        inference_max_concurrent,
        inference_max_per_sec,
    ));

    info!(
        max_concurrent = inference_max_concurrent,
        max_per_sec = inference_max_per_sec,
        "Inference rate limiter configure"
    );

    (inference, tokenizer, inference_limiter)
}

/// Construit l'EventBroadcaster connecte a Redis pub/sub.
pub fn build_broadcaster(redis_client: redis::Client) -> Arc<EventBroadcaster> {
    let redis_channel =
        std::env::var("REDIS_CHANNEL").unwrap_or_else(|_| "sentinel:events".to_string());
    Arc::new(EventBroadcaster::new().with_redis(redis_client, redis_channel))
}

/// Construit l'etat complet de l'application (tous les repos + services).
/// Consomme le pool et le client Redis (via clones).
pub async fn build_app_state(
    config: &AppConfig,
    pg_pool: sqlx::PgPool,
    redis_client: redis::Client,
) -> AppState {
    // ── Adapters sortants ──
    let rule_repo = Arc::new(PgRuleRepository::new(pg_pool.clone()));
    let infraction_repo = Arc::new(PgInfractionRepository::new(pg_pool.clone()));
    let ticket_repo = Arc::new(PgTicketRepository::new(pg_pool.clone()));
    let security_repo = Arc::new(PgSecurityEventRepository::new(pg_pool.clone()));
    let moderation_repo = Arc::new(PgModerationRepository::new(pg_pool.clone()));
    let stats_repo = Arc::new(PgStatsRepository::new(pg_pool.clone()));
    let voice_channel_repo = Arc::new(PgVoiceChannelRepository::new(pg_pool.clone()));
    let bot_config_repo = Arc::new(PgBotConfigRepository::new(pg_pool.clone()));
    let conduct_repo = Arc::new(PgConductRepository::new(pg_pool.clone()));
    let guild_repo = Arc::new(PgGuildRepository::new(pg_pool.clone()));
    // Phase 5C — Batch writes : BatchedPgLogRepository bufferise les inserts et
    // flush via multi-row INSERT toutes les 500ms ou 100 entries.
    let log_repo = Arc::new(BatchedPgLogRepository::new(
        pg_pool.clone(),
        BatchWriterConfig::default(),
    ));
    let notes_repo = Arc::new(PgNotesRepository::new(pg_pool.clone()));
    let reminder_repo = Arc::new(PgReminderRepository::new(pg_pool.clone()));
    let strike_repo = Arc::new(PgStrikeRepository::new(pg_pool.clone()));
    let cache = Arc::new(
        RedisCache::new(redis_client.clone())
            .await
            .expect("Impossible d'etablir la connexion Redis pour le cache"),
    );

    // ── Event broadcaster (Redis pub/sub → gateway WebSocket) ──
    let broadcaster = build_broadcaster(redis_client.clone());

    // ── Inference ONNX ──
    let (inference, tokenizer, inference_limiter) = build_inference();

    // Discord API (un seul client partage, cree ici avant conduct pour l'injecter).
    let discord_api: Arc<dyn crate::adapters::outbound::discord_api::DiscordApi> =
        Arc::new(DiscordApiService::new(config.discord_bot_token.clone()));

    // ── Services applicatifs ──
    let conduct_uc = Arc::new(ManageConductService::new(
        conduct_repo.clone(),
        infraction_repo.clone(),
        broadcaster.clone(),
        discord_api.clone(),
    ));

    // Buffer in-memory partage (tension de salon). Pas de persistance :
    // reset au restart bot, c'est OK car seulement les N derniers messages.
    let channel_tension_buffer = Arc::new(crate::domain::services::moderation::channel_tension::ChannelTensionBuffer::new());

    let analyze_uc = Arc::new(
        AnalyzeMessageService::new(
            rule_repo.clone(),
            infraction_repo.clone(),
            cache.clone(),
            conduct_uc.clone(),
            bot_config_repo.clone(),
            inference_limiter.clone(),
        )
        .with_text_inference(inference.clone(), tokenizer)
        .with_channel_tension(channel_tension_buffer.clone()),
    );
    let analyze_image_uc = Arc::new(AnalyzeImageService::new(
        inference.clone(),
        rule_repo.clone(),
        infraction_repo.clone(),
        cache.clone(),
        conduct_uc.clone(),
        bot_config_repo.clone(),
        inference_limiter.clone(),
    ));
    let rules_uc = Arc::new(ManageRulesService::new(rule_repo.clone(), cache.clone()));
    let infractions_uc = Arc::new(ManageInfractionsService::new(infraction_repo.clone()));
    let tickets_uc = Arc::new(ManageTicketsService::new(ticket_repo.clone(), cache.clone()));
    // Phase 5C — Batch writes : idem que log_repo, pour les audit events.
    // Phase 1 dual-write : creation deplacee plus tot pour pouvoir injecter
    // audit_logs_uc dans security_uc et moderation_uc.
    let audit_log_repo = Arc::new(BatchedPgAuditLogRepository::new(
        pg_pool.clone(),
        BatchWriterConfig::default(),
    ));
    let audit_logs_uc = Arc::new(ManageAuditLogsService::new(audit_log_repo));

    let user_activity_repo: Arc<dyn crate::ports::outbound::audit::user_activity_repository::UserActivityRepository> =
        Arc::new(PgUserActivityRepository::new(pg_pool.clone()));
    let welcome_config_repo: Arc<dyn crate::ports::outbound::community::welcome_config_repository::WelcomeConfigRepository> =
        Arc::new(PgWelcomeConfigRepository::new(pg_pool.clone()));
    // Use case Welcome (Phase 3) — handlers HTTP/gRPC passent par ce port
    // inbound, jamais par le repo direct.
    let welcome_config_uc: Arc<dyn crate::ports::inbound::community::manage_welcome_config::ManageWelcomeConfigUseCase> =
        Arc::new(crate::application::community::manage_welcome_config_service::ManageWelcomeConfigService::new(
            welcome_config_repo.clone(),
        ));
    // Automod reviews (sync Discord <-> web).
    let automod_review_repo: Arc<dyn crate::ports::outbound::moderation::automod_review_repository::AutomodReviewRepository> = Arc::new(
        crate::adapters::outbound::postgres::moderation::automod_review_repository::PgAutomodReviewRepository::new(pg_pool.clone()),
    );
    let automod_reviews_uc: Arc<dyn crate::ports::inbound::moderation::manage_automod_reviews::ManageAutomodReviewsUseCase> =
        Arc::new(crate::application::moderation::manage_automod_reviews_service::ManageAutomodReviewsService::new(
            automod_review_repo.clone(),
        ));

    let watched_user_repo = Arc::new(PgWatchedUserRepository::new(pg_pool.clone()));
    let security_uc = Arc::new(
        ManageSecurityService::new(
            security_repo.clone(),
            cache.clone(),
            watched_user_repo.clone(),
            bot_config_repo.clone(),
            moderation_repo.clone(),
        )
        .with_audit_logs_uc(
            audit_logs_uc.clone() as Arc<dyn crate::ports::inbound::audit::manage_audit_logs::ManageAuditLogsUseCase>
        ),
    );
    // Note : la creation de moderation_uc est differee plus bas pour pouvoir
    // injecter strikes_uc via with_strikes_uc (log_action_with_strike).
    let stats_uc = Arc::new(ManageStatsService::new(
        stats_repo.clone(),
        infraction_repo.clone(),
        cache.clone(),
        redis_client.clone(),
    ));
    let voice_channels_uc = Arc::new(ManageVoiceChannelsService::new(
        voice_channel_repo.clone(),
        cache.clone(),
        bot_config_repo.clone(),
    ));
    let role_panel_repo = Arc::new(PgRolePanelRepository::new(pg_pool.clone()));
    let role_panels_uc = Arc::new(ManageRolePanelsService::new(role_panel_repo));
    let analytics_repo = Arc::new(PgAnalyticsRepository::new(pg_pool.clone()));
    let daily_activity_repo = Arc::new(PgDailyActivityRepository::new(pg_pool.clone()));
    let level_repo = Arc::new(PgLevelRepository::new(pg_pool.clone()));
    let levels_uc = Arc::new(ManageLevelsService::new(level_repo));
    let notes_uc = Arc::new(ManageNotesService::new(notes_repo));
    let reminders_uc = Arc::new(ManageRemindersService::new(reminder_repo));
    let strikes_uc = Arc::new(ManageStrikesService::new(strike_repo.clone()));
    let moderation_uc = Arc::new(
        ManageModerationService::new(
            moderation_repo.clone(),
            strike_repo.clone(),
            cache.clone(),
            conduct_uc.clone(),
        )
        .with_strikes_uc(strikes_uc.clone() as Arc<dyn crate::ports::inbound::moderation::manage_strikes::ManageStrikesUseCase>)
        .with_audit_logs_uc(
            audit_logs_uc.clone() as Arc<dyn crate::ports::inbound::audit::manage_audit_logs::ManageAuditLogsUseCase>
        ),
    );
    let member_repo = Arc::new(PgMemberRepository::new(pg_pool.clone()));
    let discord_role_repo = Arc::new(PgDiscordRoleRepository::new(pg_pool.clone()));
    let wallet_repo = Arc::new(PgWalletRepository::new(pg_pool.clone()));
    let blackjack_repo = Arc::new(PgBlackjackRepository::new(pg_pool.clone()));
    // `blackjack_svc` est instancie plus bas, apres la construction de
    // `wallet_uc` (dependance de la migration #4).
    let coude_player_repo = Arc::new(PgPlayerRepository::new(pg_pool.clone()));
    let coude_players_uc = Arc::new(ManageCoudePlayersService::new(coude_player_repo.clone()));
    let coude_combat_repo = Arc::new(PgCombatRepository::new(pg_pool.clone()));
    let coude_combats_uc: Arc<dyn crate::ports::inbound::coude::manage_combats::ManageCoudeCombatsUseCase> =
        Arc::new(
            ManageCoudeCombatsService::new(coude_combat_repo.clone()).with_surprise_gate(
                coude_players_uc.clone()
                    as Arc<dyn crate::ports::inbound::coude::manage_players::ManageCoudePlayersUseCase>,
                bot_config_repo.clone(),
            ),
        );
    // `coude_bet_repo` est construit plus bas, apres `wallet_uc`
    // (Migration #7 : le repo delegue les mutations user_wallets au
    // service wallet unifie pour la detection faillite/jackpot).
    let coude_economy_repo = Arc::new(PgEconomyRepository::new(pg_pool.clone()));

    // Phase 9 Part D — railleries (cree en amont : utilise par le wallet UC
    // unifie, les services de resolution de combat, et l'economy UC pour
    // les taunts "don genereux").
    let coude_taunts_repo: Arc<dyn crate::ports::outbound::coude::taunts_repository::TauntsRepository> =
        Arc::new(PgTauntsRepository::new(pg_pool.clone()));

    // Maledictions — repo cree tot pour pouvoir le brancher dans taunts
    // (effet Insomnia) et wheel (effet Heartbreak).
    let coude_curses_repo: Arc<dyn crate::ports::outbound::coude::curses_repository::CursesRepository> =
        Arc::new(PgCursesRepository::new(pg_pool.clone()));

    // Filet de securite et vendetta — repos crees tot pour pouvoir les
    // brancher dans bets (boost x1.5 paris gagnants) et combat
    // (bonus +100% revanche). Re-utilises plus bas pour creer les UC.
    let coude_safety_net_repo: Arc<dyn crate::ports::outbound::coude::safety_net_repository::SafetyNetRepository> =
        Arc::new(PgSafetyNetRepository::new(pg_pool.clone()));
    let coude_vendetta_repo: Arc<dyn crate::ports::outbound::coude::vendetta_repository::VendettaRepository> =
        Arc::new(PgVendettaRepository::new(pg_pool.clone()));

    let coude_taunts_uc: Arc<dyn crate::ports::inbound::coude::manage_taunts::ManageCoudeTauntsUseCase> = Arc::new(
        ManageCoudeTauntsService::new(
            coude_taunts_repo,
            coude_player_repo.clone(),
            bot_config_repo.clone(),
        )
        .with_curses_repo(coude_curses_repo.clone()),
    );

    // Migration wallet unifie : use case qui centralise les mutations
    // `user_wallets` + detecte faillite/jackpot en retournant les
    // TauntEvent a dispatcher. Depend de `coude_taunts_uc`.
    let wallet_uc: Arc<dyn crate::ports::inbound::casino::manage_wallet::ManageWalletUseCase> =
        Arc::new(ManageWalletService::new(
            wallet_repo.clone(),
            coude_taunts_uc.clone(),
        ));

    // Migration #7 : bet repo instantie apres wallet_uc pour pouvoir
    // deleguer les mutations user_wallets via credit_tx/debit_tx.
    let coude_bet_repo = Arc::new(PgBetRepository::new(
        pg_pool.clone(),
        wallet_uc.clone(),
    ));
    // Bets ne depend que d'une lecture de combat — on injecte le narrow port
    // `CombatQueryRepository` (impl par `PgCombatRepository`) plutot que
    // le use case complet `ManageCoudeCombatsUseCase`. Cf. P0 #2 audit.
    let combat_query_repo: Arc<dyn crate::ports::outbound::coude::combat_query_repository::CombatQueryRepository> =
        coude_combat_repo.clone();
    let coude_bets_uc = Arc::new(
        ManageCoudeBetsService::new(coude_bet_repo, combat_query_repo)
            .with_safety_net_repo(coude_safety_net_repo.clone())
            .with_bot_config_repo(bot_config_repo.clone()),
    );

    // Migration #4 : `blackjack_svc` passe ses mutations wallet (mise, cashout,
    // double down) par `wallet_uc` pour centralisation + detection auto des
    // taunts (faillite, jackpot). `wallet_repo` reste injecte pour
    // `get_or_create` au demarrage de la toute premiere partie.
    let blackjack_svc = Arc::new(BlackjackService::new(
        blackjack_repo,
        wallet_repo.clone(),
        wallet_uc.clone(),
    ));

    // Slot machine — nouvelle feature (migration 157).
    let slot_repo = Arc::new(crate::adapters::outbound::postgres::casino::slot_repository::PgSlotRepository::new(pg_pool.clone()));
    let slot_uc: Arc<dyn crate::ports::inbound::casino::manage_slot::ManageSlotUseCase> =
        Arc::new(crate::application::casino::manage_slot_service::ManageSlotService::new(
            slot_repo,
            bot_config_repo.clone(),
            wallet_uc.clone(),
            pg_pool.clone(),
        ));

    // Roue du Destin — Sprint 2 sign'ature (migration 158).
    let wheel_repo = Arc::new(crate::adapters::outbound::postgres::casino::wheel_repository::PgWheelRepository::new(pg_pool.clone()));
    let wheel_uc: Arc<dyn crate::ports::inbound::casino::manage_wheel::ManageWheelUseCase> =
        Arc::new(
            crate::application::casino::manage_wheel_service::ManageWheelService::new(
                wheel_repo,
                wallet_uc.clone(),
                pg_pool.clone(),
            )
            .with_curses_repo(coude_curses_repo.clone()),
        );

    let coude_economy_uc = Arc::new(
        ManageCoudeEconomyService::new(
            coude_economy_repo.clone(),
            wallet_uc.clone(),
            coude_taunts_uc.clone(),
        )
        .with_leaky_wallet_support(wallet_repo.clone(), coude_curses_repo.clone())
        .with_player_repo(coude_player_repo.clone()),
    );
    let coude_inventory_repo = Arc::new(PgInventoryRepository::new(pg_pool.clone()));
    let coude_inventory_uc = Arc::new(
        ManageCoudeInventoryService::new(coude_inventory_repo)
            .with_bot_config_repo(bot_config_repo.clone()),
    );
    let coude_social_repo: Arc<dyn crate::ports::outbound::coude::social_repository::SocialRepository> =
        Arc::new(PgSocialRepository::new(pg_pool.clone()));
    let coude_social_uc = Arc::new(ManageCoudeSocialService::new(
        coude_social_repo.clone(),
        coude_player_repo.clone(),
        coude_economy_repo.clone(),
        bot_config_repo.clone(),
        wallet_uc.clone(),
    ));

    // Phase 10 — braquage (depend de cashbox_repo, inventory_uc, wallet_repo).
    let coude_heist_repo: Arc<dyn crate::ports::outbound::coude::heist_repository::HeistRepository> =
        Arc::new(PgHeistRepository::new(pg_pool.clone()));

    // Phase 2 refacto : use case dedie qui orchestre la resolution batch des
    // combats betting. Remplacera coude-worker/src/jobs/resolve_betting.rs
    // en Phase 3.
    let resolve_betting_batch_uc: Arc<dyn crate::ports::inbound::coude::resolve_betting_batch::ResolveBettingBatchUseCase> =
        Arc::new(ResolveBettingBatchService::new(
            coude_combat_repo.clone(),
            coude_player_repo.clone(),
            wallet_repo.clone(),
            coude_bets_uc.clone(),
            coude_inventory_uc.clone(),
            coude_social_uc.clone(),
            coude_taunts_uc.clone(),
            bot_config_repo.clone(),
        ));
    let coude_cashbox_repo: Arc<dyn crate::ports::outbound::coude::cashbox_repository::CashboxRepository> =
        Arc::new(PgCashboxRepository::new(pg_pool.clone()));
    let expire_combats_batch_uc: Arc<dyn crate::ports::inbound::coude::expire_combats_batch::ExpireCombatsBatchUseCase> =
        Arc::new(ExpireCombatsBatchService::new(
            coude_combat_repo.clone(),
            coude_player_repo.clone(),
            wallet_repo.clone(),
            coude_cashbox_repo.clone(),
            coude_bets_uc.clone(),
        ));
    let coude_catalog_uc: Arc<dyn crate::ports::inbound::coude::manage_catalog::ManageCoudeCatalogUseCase> =
        Arc::new(ManageCoudeCatalogService::new());
    let coude_cashbox_uc: Arc<dyn crate::ports::inbound::coude::manage_cashbox::ManageCoudeCashboxUseCase> = Arc::new(
        ManageCoudeCashboxService::new(coude_cashbox_repo.clone(), wallet_repo.clone()),
    );

    // Phase 10 — heist UC (depend de cashbox_repo + inventory_uc + wallet_repo).
    let coude_heist_uc: Arc<dyn crate::ports::inbound::coude::manage_heist::ManageCoudeHeistUseCase> = Arc::new(
        ManageCoudeHeistService::new(
            coude_heist_repo.clone(),
            coude_cashbox_repo.clone(),
            coude_inventory_uc.clone(),
            wallet_repo.clone(),
            bot_config_repo.clone(),
        )
        .with_player_repo(coude_player_repo.clone()),
    );

    // Maledictions (cf. COUPE_AMELIORATIONS 5.1) — repo deja cree plus haut
    // pour permettre le branchement Heartbreak dans wheel.
    let coude_curses_uc: Arc<dyn crate::ports::inbound::coude::manage_curses::ManageCoudeCursesUseCase> = Arc::new(
        ManageCoudeCursesService::new(coude_curses_repo.clone(), wallet_repo.clone()),
    );

    // Filet de securite (cf. COUPE_AMELIORATIONS 4.4) — repo deja cree
    // plus haut pour permettre le branchement dans bets et combat.
    let coude_safety_net_uc: Arc<dyn crate::ports::inbound::coude::manage_safety_net::ManageCoudeSafetyNetUseCase> =
        Arc::new(
            ManageCoudeSafetyNetService::new(coude_safety_net_repo.clone())
                .with_bot_config_repo(bot_config_repo.clone()),
        );

    // Vendetta (cf. COUPE_AMELIORATIONS 5.3) — repo deja cree plus haut.
    let coude_vendetta_uc: Arc<dyn crate::ports::inbound::coude::manage_vendetta::ManageCoudeVendettaUseCase> =
        Arc::new(ManageCoudeVendettaService::new(coude_vendetta_repo.clone()));

    // Memorial des clodos / tout-ou-rien log (cf. COUPE_AMELIORATIONS 6.1).
    let coude_tout_ou_rien_repo: Arc<dyn crate::ports::outbound::coude::tout_ou_rien_repository::ToutOuRienRepository> =
        Arc::new(PgToutOuRienRepository::new(pg_pool.clone()));

    // Phase 2 #1 audit : RNG /tout-ou-rien migre cote API.
    let play_tout_ou_rien_uc: Arc<
        dyn crate::ports::inbound::coude::play_tout_ou_rien::PlayToutOuRienUseCase,
    > = Arc::new(PlayToutOuRienService::new(
        coude_player_repo.clone(),
        wallet_uc.clone(),
        coude_social_repo.clone(),
        coude_tout_ou_rien_repo.clone(),
    ));

    // Phase 2 #2 audit : RNG /travaux migre cote API.
    let play_travaux_uc: Arc<dyn crate::ports::inbound::coude::play_travaux::PlayTravauxUseCase> =
        Arc::new(PlayTravauxService::new(
            coude_heist_repo.clone(),
            coude_player_repo.clone(),
            wallet_uc.clone(),
            coude_social_repo.clone(),
        ));

    // Phase 2 #4 audit : RNG /voler (d20 + steal %) migre cote API.
    let roll_steal_uc: Arc<dyn crate::ports::inbound::coude::roll_steal::RollStealUseCase> =
        Arc::new(RollStealService::new());

    // Phase 3 #9 audit : catalogue de templates flavor (steal/heist/prank).
    let coude_flavor_templates_repo: Arc<
        dyn crate::ports::outbound::coude::flavor_templates_repository::FlavorTemplatesRepository,
    > = Arc::new(PgFlavorTemplatesRepository::new(pg_pool.clone()));

    // Sync Discord <-> Web (Phase 1 — cf. SYNC_DISCORD_WEB_DESIGN.md).
    // Repo outbound + use case inbound : on injecte uniquement le use
    // case dans AppState pour respecter l'archi hexagonale (handlers
    // HTTP/gRPC ne touchent jamais les repos directement).
    let discord_action_message_repo: Arc<
        dyn crate::ports::outbound::audit::discord_action_message_repository::DiscordActionMessageRepository,
    > = Arc::new(
        crate::adapters::outbound::postgres::audit::discord_action_message_repository::PgDiscordActionMessageRepository::new(
            pg_pool.clone(),
        ),
    );
    let discord_action_messages_uc: Arc<
        dyn crate::ports::inbound::audit::manage_discord_action_messages::ManageDiscordActionMessagesUseCase,
    > = Arc::new(crate::application::audit::manage_discord_action_messages_service::ManageDiscordActionMessagesService::new(
        discord_action_message_repo,
    ));

    // Primes collectives (cf. COUPE_AMELIORATIONS 5.3).
    let coude_bounty_repo: Arc<dyn crate::ports::outbound::coude::bounty_repository::BountyRepository> =
        Arc::new(PgBountyRepository::new(pg_pool.clone()));

    // Compteurs de refus / dette d honneur (cf. COUPE_AMELIORATIONS 5.3).
    let coude_refusal_count_repo: Arc<dyn crate::ports::outbound::coude::refusal_count_repository::RefusalCountRepository> =
        Arc::new(PgRefusalCountRepository::new(pg_pool.clone()));

    // Coalitions (cf. COUPE_AMELIORATIONS 5.3).
    let coude_coalition_repo: Arc<dyn crate::ports::outbound::coude::coalition_repository::CoalitionRepository> =
        Arc::new(PgCoalitionRepository::new(pg_pool.clone()));

    // Ultimates par classe (cf. COUPE_AMELIORATIONS 3.1).
    let coude_ultimate_repo: Arc<dyn crate::ports::outbound::coude::ultimate_repository::UltimateRepository> =
        Arc::new(PgUltimateRepository::new(pg_pool.clone()));

    let coude_steal_protection_repo: Arc<
        dyn crate::ports::outbound::coude::steal_protection_repository::StealProtectionRepository,
    > = Arc::new(PgStealProtectionRepository::new(pg_pool.clone()));
    let coude_steal_protections_uc: Arc<
        dyn crate::ports::inbound::coude::manage_steal_protections::ManageCoudeStealProtectionsUseCase,
    > = Arc::new(ManageCoudeStealProtectionsService::new(
        coude_steal_protection_repo,
    ));
    let coude_steal_boost_repo: Arc<dyn crate::ports::outbound::coude::steal_boost_repository::StealBoostRepository> =
        Arc::new(PgStealBoostRepository::new(pg_pool.clone()));
    let coude_steal_boosts_uc: Arc<dyn crate::ports::inbound::coude::manage_steal_boosts::ManageCoudeStealBoostsUseCase> =
        Arc::new(
            ManageCoudeStealBoostsService::new(coude_steal_boost_repo)
                .with_bot_config_repo(bot_config_repo.clone()),
        );
    let resolve_combat_now_uc: Arc<dyn crate::ports::inbound::coude::resolve_combat_now::ResolveCombatNowUseCase> =
        Arc::new(
            ResolveCombatNowService::new(
                coude_combat_repo.clone(),
                coude_combats_uc.clone(),
                coude_players_uc.clone() as Arc<dyn crate::ports::inbound::coude::manage_players::ManageCoudePlayersUseCase>,
                wallet_repo.clone(),
                coude_bets_uc.clone(),
                coude_inventory_uc.clone(),
                coude_social_uc.clone(),
                coude_taunts_uc.clone(),
                bot_config_repo.clone(),
            )
            .with_curses_repo(coude_curses_repo.clone())
            .with_safety_net_repo(coude_safety_net_repo.clone())
            .with_vendetta_repo(coude_vendetta_repo.clone())
            .with_player_repo(coude_player_repo.clone())
            .with_bounty_repo(coude_bounty_repo.clone())
            .with_coalition_repo(coude_coalition_repo.clone())
            .with_ultimate_repo(coude_ultimate_repo.clone()),
        );
    let resolve_friendly_duel_uc: Arc<dyn crate::ports::inbound::coude::resolve_friendly_duel::ResolveFriendlyDuelUseCase> =
        Arc::new(crate::application::coude::combat::resolve_friendly_duel::ResolveFriendlyDuelService::new(
            coude_player_repo.clone(),
            coude_players_uc.clone() as Arc<dyn crate::ports::inbound::coude::manage_players::ManageCoudePlayersUseCase>,
            bot_config_repo.clone(),
        ));
    let watched_users_uc = Arc::new(ManageWatchedUsersService::new(
        watched_user_repo,
        infractions_uc.clone(),
        moderation_uc.clone(),
        security_uc.clone(),
        conduct_uc.clone(),
        notes_uc.clone(),
    ));

    let members_uc = Arc::new(ManageMembersService::new(
        member_repo,
        infractions_uc.clone(),
        moderation_uc.clone(),
        conduct_uc.clone(),
        stats_uc.clone(),
    ));

    // ── Discord API service : instance deja creee plus haut pour conduct.
    // On re-declare ici pour garder la variable accessible dans la suite du
    // bootstrap (AppState.discord_api).

    // ── Job client (queue Redis → worker) ──
    let queue_key =
        std::env::var("REDIS_QUEUE_KEY").unwrap_or_else(|_| "sentinel:jobs".to_string());
    let job_client = JobClient::new(redis_client.clone(), queue_key);

    // ── State ──
    AppState {
        analyze_uc,
        analyze_image_uc,
        rules_uc,
        infractions_uc,
        tickets_uc,
        security_uc,
        moderation_uc,
        stats_uc,
        voice_channels_uc,
        conduct_uc,
        watched_users_uc,
        audit_logs_uc,
        levels_uc,
        role_panels_uc,
        notes_uc,
        reminders_uc,
        strikes_uc,
        members_uc,
        analytics_repo,
        daily_activity_repo,
        log_repo,
        guild_repo,
        bot_config_repo,
        discord_role_repo,
        wallet_repo,
        wallet_uc: wallet_uc.clone(),
        blackjack_svc,
        slot_uc,
        wheel_uc,
        coude_players_uc,
        coude_combats_uc,
        coude_bets_uc,
        coude_economy_uc,
        coude_inventory_uc,
        coude_social_uc,
        resolve_betting_batch_uc,
        expire_combats_batch_uc,
        resolve_combat_now_uc,
        resolve_friendly_duel_uc,
        coude_catalog_uc,
        coude_cashbox_uc,
        coude_steal_protections_uc,
        coude_steal_boosts_uc,
        coude_taunts_uc,
        coude_heist_uc,
        coude_curses_uc,
        coude_safety_net_uc,
        coude_vendetta_uc,
        coude_tout_ou_rien_repo,
        play_tout_ou_rien_uc,
        play_travaux_uc,
        roll_steal_uc,
        coude_flavor_templates_repo,
        discord_action_messages_uc,
        coude_bounty_repo: coude_bounty_repo.clone(),
        coude_refusal_count_repo,
        coude_coalition_repo: coude_coalition_repo.clone(),
        coude_ultimate_repo: coude_ultimate_repo.clone(),
        broadcaster,
        job_client,
        discord_api,
        inference: inference.clone(),
        api_key: config.api_key.clone(),
        discord_bot_token: config.discord_bot_token.clone(),
        user_activity_repo: user_activity_repo.clone(),
        welcome_config_uc,
        automod_reviews_uc,
        export_uc: Arc::new(ExportService::new(pg_pool.clone())),
        evidence_repo: Arc::new(PgEvidenceRepository::new(pg_pool.clone())),
        review_repo: Arc::new(PgReviewRepository::new(pg_pool.clone())),
        modstats_repo: Arc::new(PgModstatsRepository::new(pg_pool.clone())),
        game_repo: Arc::new(PgGameRepository::new(pg_pool.clone())),
        sponsorship_repo: Arc::new(PgSponsorshipRepository::new(pg_pool.clone())),
        temp_role_repo: Arc::new(PgTempRoleRepository::new(pg_pool.clone())),
        pending_action_repo: Arc::new(PgPendingActionRepository::new(pg_pool.clone())),
        blackjack_table_repo: Arc::new(PgBlackjackTableRepository::new(pg_pool.clone())),
        pg_pool: pg_pool.clone(),
        redis_client: redis_client.clone(),
        cache: Some(cache.clone()),
        superadmin_user_ids: Arc::new(config.superadmin_user_ids.clone()),
        discord_oauth_client_id: config.discord_oauth_client_id.clone(),
        discord_oauth_client_secret: config.discord_oauth_client_secret.clone(),
        discord_oauth_redirect_uri: config.discord_oauth_redirect_uri.clone(),
        web_front_url: config.web_front_url.clone(),
    }
}
