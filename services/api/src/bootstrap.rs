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

use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use tracing::{error, info};

use crate::adapters::inbound::http::state::AppState;
use crate::adapters::inbound::ws::broadcaster::EventBroadcaster;
use crate::adapters::outbound::batching::{
    BatchWriterConfig, BatchedPgAuditLogRepository, BatchedPgLogRepository,
};
use crate::adapters::outbound::job_client::JobClient;
use crate::adapters::outbound::postgres::{
    PgAnalyticsRepository, PgBlackjackRepository, PgBlackjackTableRepository, PgBotConfigRepository,
    PgConductRepository, PgCoudeBetRepository, PgCoudeCashboxRepository, PgCoudeCombatRepository,
    PgCoudeEconomyRepository, PgCoudeHeistRepository, PgCoudeInventoryRepository,
    PgCoudePlayerRepository, PgCoudeSocialRepository, PgCoudeStealBoostRepository,
    PgCoudeStealProtectionRepository, PgCoudeTauntsRepository, PgDailyActivityRepository,
    PgDiscordRoleRepository, PgEvidenceRepository, PgGameRepository, PgGuildRepository,
    PgIaConfigRepository, PgInfractionRepository, PgLevelRepository, PgMemberRepository,
    PgModerationRepository, PgModstatsRepository, PgNotesRepository, PgPendingActionRepository,
    PgReminderRepository, PgReviewRepository, PgRolePanelRepository, PgRuleRepository,
    PgSecurityEventRepository, PgSponsorshipRepository, PgStatsRepository, PgStrikeRepository,
    PgTempRoleRepository, PgTicketRepository, PgUserActivityRepository, PgVoiceChannelRepository,
    PgWalletRepository, PgWatchedUserRepository, PgWelcomeConfigRepository,
};
use crate::adapters::outbound::redis_cache::RedisCache;
use crate::application::{
    AnalyzeImageService, AnalyzeMessageService, BlackjackService, ExpireCombatsBatchService,
    ExportService, ManageAuditLogsService, ManageConductService, ManageCoudeBetsService,
    ManageCoudeCashboxService, ManageCoudeCatalogService, ManageCoudeCombatsService,
    ManageCoudeEconomyService, ManageCoudeHeistService, ManageCoudeInventoryService,
    ManageCoudePlayersService, ManageCoudeSocialService, ManageCoudeStealBoostsService,
    ManageCoudeStealProtectionsService, ManageCoudeTauntsService, ManageInfractionsService,
    ManageLevelsService, ManageMembersService, ManageModerationService, ManageNotesService,
    ManageRemindersService, ManageRolePanelsService, ManageRulesService, ManageSecurityService,
    ManageStatsService, ManageStrikesService, ManageTicketsService, ManageVoiceChannelsService,
    ManageWatchedUsersService, ResolveBettingBatchService, ResolveCombatNowService,
};
use crate::config::AppConfig;
use crate::domain::services::{DiscordApiService, InferenceRateLimiter, InferenceService, TextTokenizer};

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
    let ia_config_repo = Arc::new(PgIaConfigRepository::new(pg_pool.clone()));
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

    // ── Services applicatifs ──
    let conduct_uc = Arc::new(ManageConductService::new(
        conduct_repo.clone(),
        infraction_repo.clone(),
        broadcaster.clone(),
        config.discord_bot_token.clone(),
    ));

    let analyze_uc = Arc::new(
        AnalyzeMessageService::new(
            rule_repo.clone(),
            infraction_repo.clone(),
            cache.clone(),
            conduct_uc.clone(),
            ia_config_repo.clone(),
            inference_limiter.clone(),
        )
        .with_text_inference(inference.clone(), tokenizer),
    );
    let analyze_image_uc = Arc::new(AnalyzeImageService::new(
        inference.clone(),
        rule_repo.clone(),
        infraction_repo.clone(),
        cache.clone(),
        conduct_uc.clone(),
        ia_config_repo.clone(),
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

    let user_activity_repo: Arc<dyn crate::ports::outbound::UserActivityRepository> =
        Arc::new(PgUserActivityRepository::new(pg_pool.clone()));
    let welcome_config_repo: Arc<dyn crate::ports::outbound::WelcomeConfigRepository> =
        Arc::new(PgWelcomeConfigRepository::new(pg_pool.clone()));
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
            audit_logs_uc.clone() as Arc<dyn crate::ports::inbound::ManageAuditLogsUseCase>
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
        .with_strikes_uc(strikes_uc.clone() as Arc<dyn crate::ports::inbound::ManageStrikesUseCase>)
        .with_audit_logs_uc(
            audit_logs_uc.clone() as Arc<dyn crate::ports::inbound::ManageAuditLogsUseCase>
        ),
    );
    let member_repo = Arc::new(PgMemberRepository::new(pg_pool.clone()));
    let discord_role_repo = Arc::new(PgDiscordRoleRepository::new(pg_pool.clone()));
    let wallet_repo = Arc::new(PgWalletRepository::new(pg_pool.clone()));
    let blackjack_repo = Arc::new(PgBlackjackRepository::new(pg_pool.clone()));
    let blackjack_svc = Arc::new(BlackjackService::new(blackjack_repo, wallet_repo.clone()));
    let coude_player_repo = Arc::new(PgCoudePlayerRepository::new(pg_pool.clone()));
    let coude_players_uc = Arc::new(ManageCoudePlayersService::new(coude_player_repo.clone()));
    let coude_combat_repo = Arc::new(PgCoudeCombatRepository::new(pg_pool.clone()));
    let coude_combats_uc: Arc<dyn crate::ports::inbound::ManageCoudeCombatsUseCase> =
        Arc::new(
            ManageCoudeCombatsService::new(coude_combat_repo.clone()).with_surprise_gate(
                coude_players_uc.clone()
                    as Arc<dyn crate::ports::inbound::ManageCoudePlayersUseCase>,
                bot_config_repo.clone(),
            ),
        );
    let coude_bet_repo = Arc::new(PgCoudeBetRepository::new(pg_pool.clone()));
    let coude_bets_uc = Arc::new(ManageCoudeBetsService::new(
        coude_bet_repo,
        coude_combats_uc.clone(),
    ));
    let coude_economy_repo = Arc::new(PgCoudeEconomyRepository::new(pg_pool.clone()));
    let coude_economy_uc = Arc::new(ManageCoudeEconomyService::new(coude_economy_repo.clone()));
    let coude_inventory_repo = Arc::new(PgCoudeInventoryRepository::new(pg_pool.clone()));
    let coude_inventory_uc = Arc::new(ManageCoudeInventoryService::new(coude_inventory_repo));
    let coude_social_repo = Arc::new(PgCoudeSocialRepository::new(pg_pool.clone()));
    let coude_social_uc = Arc::new(ManageCoudeSocialService::new(
        coude_social_repo,
        coude_player_repo.clone(),
        coude_economy_repo.clone(),
        bot_config_repo.clone(),
    ));

    // Phase 9 Part D — railleries (cree en amont : utilise par les deux
    // services de resolution de combat).
    let coude_taunts_repo: Arc<dyn crate::ports::outbound::CoudeTauntsRepository> =
        Arc::new(PgCoudeTauntsRepository::new(pg_pool.clone()));
    let coude_taunts_uc: Arc<dyn crate::ports::inbound::ManageCoudeTauntsUseCase> = Arc::new(
        ManageCoudeTauntsService::new(coude_taunts_repo, coude_player_repo.clone()),
    );

    // Phase 10 — braquage (depend de cashbox_repo, inventory_uc, wallet_repo).
    let coude_heist_repo: Arc<dyn crate::ports::outbound::CoudeHeistRepository> =
        Arc::new(PgCoudeHeistRepository::new(pg_pool.clone()));

    // Phase 2 refacto : use case dedie qui orchestre la resolution batch des
    // combats betting. Remplacera coude-worker/src/jobs/resolve_betting.rs
    // en Phase 3.
    let resolve_betting_batch_uc: Arc<dyn crate::ports::inbound::ResolveBettingBatchUseCase> =
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
    let coude_cashbox_repo: Arc<dyn crate::ports::outbound::CoudeCashboxRepository> =
        Arc::new(PgCoudeCashboxRepository::new(pg_pool.clone()));
    let expire_combats_batch_uc: Arc<dyn crate::ports::inbound::ExpireCombatsBatchUseCase> =
        Arc::new(ExpireCombatsBatchService::new(
            coude_combat_repo.clone(),
            coude_player_repo.clone(),
            wallet_repo.clone(),
            coude_cashbox_repo.clone(),
            coude_bets_uc.clone(),
        ));
    let coude_catalog_uc: Arc<dyn crate::ports::inbound::ManageCoudeCatalogUseCase> =
        Arc::new(ManageCoudeCatalogService::new());
    let coude_cashbox_uc: Arc<dyn crate::ports::inbound::ManageCoudeCashboxUseCase> = Arc::new(
        ManageCoudeCashboxService::new(coude_cashbox_repo.clone(), wallet_repo.clone()),
    );

    // Phase 10 — heist UC (depend de cashbox_repo + inventory_uc + wallet_repo).
    let coude_heist_uc: Arc<dyn crate::ports::inbound::ManageCoudeHeistUseCase> =
        Arc::new(ManageCoudeHeistService::new(
            coude_heist_repo.clone(),
            coude_cashbox_repo.clone(),
            coude_inventory_uc.clone(),
            wallet_repo.clone(),
            bot_config_repo.clone(),
        ));

    let coude_steal_protection_repo: Arc<
        dyn crate::ports::outbound::CoudeStealProtectionRepository,
    > = Arc::new(PgCoudeStealProtectionRepository::new(pg_pool.clone()));
    let coude_steal_protections_uc: Arc<
        dyn crate::ports::inbound::ManageCoudeStealProtectionsUseCase,
    > = Arc::new(ManageCoudeStealProtectionsService::new(
        coude_steal_protection_repo,
    ));
    let coude_steal_boost_repo: Arc<dyn crate::ports::outbound::CoudeStealBoostRepository> =
        Arc::new(PgCoudeStealBoostRepository::new(pg_pool.clone()));
    let coude_steal_boosts_uc: Arc<dyn crate::ports::inbound::ManageCoudeStealBoostsUseCase> =
        Arc::new(
            ManageCoudeStealBoostsService::new(coude_steal_boost_repo)
                .with_bot_config_repo(bot_config_repo.clone()),
        );
    let resolve_combat_now_uc: Arc<dyn crate::ports::inbound::ResolveCombatNowUseCase> =
        Arc::new(ResolveCombatNowService::new(
            coude_combat_repo.clone(),
            coude_combats_uc.clone(),
            coude_players_uc.clone() as Arc<dyn crate::ports::inbound::ManageCoudePlayersUseCase>,
            wallet_repo.clone(),
            coude_bets_uc.clone(),
            coude_inventory_uc.clone(),
            coude_social_uc.clone(),
            coude_taunts_uc.clone(),
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

    // ── Discord API service ──
    let discord_api = Arc::new(DiscordApiService::new(config.discord_bot_token.clone()));

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
        ia_config_repo,
        discord_role_repo,
        wallet_repo,
        blackjack_svc,
        coude_players_uc,
        coude_combats_uc,
        coude_bets_uc,
        coude_economy_uc,
        coude_inventory_uc,
        coude_social_uc,
        resolve_betting_batch_uc,
        expire_combats_batch_uc,
        resolve_combat_now_uc,
        coude_catalog_uc,
        coude_cashbox_uc,
        coude_steal_protections_uc,
        coude_steal_boosts_uc,
        coude_taunts_uc,
        coude_heist_uc,
        broadcaster,
        job_client,
        discord_api,
        inference: inference.clone(),
        api_key: config.api_key.clone(),
        discord_bot_token: config.discord_bot_token.clone(),
        user_activity_repo: user_activity_repo.clone(),
        welcome_config_repo: welcome_config_repo.clone(),
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
