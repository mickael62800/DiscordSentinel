//! Assemblage complet de l'AppState : tous les repos + services (DI).

use std::sync::Arc;

use crate::adapters::inbound::http::state::AppState;
use crate::adapters::outbound::batching::audit_log_batcher::BatchedPgAuditLogRepository;
use crate::adapters::outbound::batching::batch_writer::BatchWriterConfig;
use crate::adapters::outbound::batching::log_batcher::BatchedPgLogRepository;
use crate::adapters::outbound::discord_api::DiscordApiService;
use crate::adapters::outbound::job_client::JobClient;
use crate::adapters::outbound::postgres::audit::analytics_repository::PgAnalyticsRepository;
use crate::adapters::outbound::postgres::audit::modstats_repository::PgModstatsRepository;
use crate::adapters::outbound::postgres::audit::security_event_repository::PgSecurityEventRepository;
use crate::adapters::outbound::postgres::audit::stats_repository::PgStatsRepository;
use crate::adapters::outbound::postgres::audit::user_activity_repository::PgUserActivityRepository;
use crate::adapters::outbound::postgres::audit::watched_user_repository::PgWatchedUserRepository;
use crate::adapters::outbound::postgres::casino::blackjack_repository::PgBlackjackRepository;
use crate::adapters::outbound::postgres::casino::blackjack_table_repository::PgBlackjackTableRepository;
use crate::adapters::outbound::postgres::casino::game_repository::PgGameRepository;
use crate::adapters::outbound::postgres::casino::wallet_repository::PgWalletRepository;
use crate::adapters::outbound::postgres::community::daily_activity_repository::PgDailyActivityRepository;
use crate::adapters::outbound::postgres::community::discord_role_repository::PgDiscordRoleRepository;
use crate::adapters::outbound::postgres::community::level_repository::PgLevelRepository;
use crate::adapters::outbound::postgres::community::member_repository::PgMemberRepository;
use crate::adapters::outbound::postgres::community::role_panel_repository::PgRolePanelRepository;
use crate::adapters::outbound::postgres::community::temp_role_repository::PgTempRoleRepository;
use crate::adapters::outbound::postgres::community::voice_channel_repository::PgVoiceChannelRepository;
use crate::adapters::outbound::postgres::community::welcome_config_repository::PgWelcomeConfigRepository;
use crate::adapters::outbound::postgres::coude::bet_repository::PgBetRepository;
use crate::adapters::outbound::postgres::coude::cashbox_repository::PgCashboxRepository;
use crate::adapters::outbound::postgres::coude::tournament_repository::PgTournamentRepository;
use crate::adapters::outbound::postgres::coude::combat_repository::PgCombatRepository;
use crate::adapters::outbound::postgres::coude::curses_repository::PgCursesRepository;
use crate::adapters::outbound::postgres::coude::economy_repository::PgEconomyRepository;
use crate::adapters::outbound::postgres::coude::flavor_templates_repository::PgFlavorTemplatesRepository;
use crate::adapters::outbound::postgres::coude::heist_repository::PgHeistRepository;
use crate::adapters::outbound::postgres::coude::inventory_repository::PgInventoryRepository;
use crate::adapters::outbound::postgres::coude::player_repository::PgPlayerRepository;
use crate::adapters::outbound::postgres::coude::refusal_count_repository::PgRefusalCountRepository;
use crate::adapters::outbound::postgres::coude::safety_net_repository::PgSafetyNetRepository;
use crate::adapters::outbound::postgres::coude::social_repository::PgSocialRepository;
use crate::adapters::outbound::postgres::coude::sponsorship_repository::PgSponsorshipRepository;
use crate::adapters::outbound::postgres::coude::steal_attempt_repository::PgStealAttemptRepository;
use crate::adapters::outbound::postgres::coude::steal_boost_repository::PgStealBoostRepository;
use crate::adapters::outbound::postgres::coude::steal_protection_repository::PgStealProtectionRepository;
use crate::adapters::outbound::postgres::coude::taunts_repository::PgTauntsRepository;
use crate::adapters::outbound::postgres::coude::tout_ou_rien_repository::PgToutOuRienRepository;
use crate::adapters::outbound::postgres::moderation::evidence_repository::PgEvidenceRepository;
use crate::adapters::outbound::postgres::moderation::infraction_repository::PgInfractionRepository;
use crate::adapters::outbound::postgres::moderation::moderation_repository::PgModerationRepository;
use crate::adapters::outbound::postgres::moderation::notes_repository::PgNotesRepository;
use crate::adapters::outbound::postgres::moderation::pending_action_repository::PgPendingActionRepository;
use crate::adapters::outbound::postgres::moderation::reminder_repository::PgReminderRepository;
use crate::adapters::outbound::postgres::moderation::review_repository::PgReviewRepository;
use crate::adapters::outbound::postgres::moderation::rule_repository::PgRuleRepository;
use crate::adapters::outbound::postgres::moderation::strike_repository::PgStrikeRepository;
use crate::adapters::outbound::postgres::system::bot_config_repository::PgBotConfigRepository;
use crate::adapters::outbound::postgres::system::guild_repository::PgGuildRepository;
use crate::adapters::outbound::postgres::system::ticket_repository::PgTicketRepository;
use crate::adapters::outbound::redis_cache::RedisCache;
use crate::application::ai::analyze_image_service::AnalyzeImageService;
use crate::application::ai::analyze_message_service::AnalyzeMessageService;
use crate::application::audit::manage_audit_logs_service::ManageAuditLogsService;
use crate::application::audit::manage_security_service::ManageSecurityService;
use crate::application::audit::manage_stats_service::ManageStatsService;
use crate::application::audit::manage_watched_users_service::ManageWatchedUsersService;
use crate::application::casino::blackjack_service::BlackjackService;
use crate::application::casino::manage_wallet_service::ManageWalletService;
use crate::application::community::manage_levels_service::ManageLevelsService;
use crate::application::community::manage_members_service::ManageMembersService;
use crate::application::community::manage_role_panels_service::ManageRolePanelsService;
use crate::application::community::voice_channels::ManageVoiceChannelsService;
use crate::application::coude::bet::manage::ManageCoudeBetsService;
use crate::application::coude::bet::resolve_batch::ResolveBettingBatchService;
use crate::application::coude::combat::expire_batch::ExpireCombatsBatchService;
use crate::application::coude::combat::manage::ManageCoudeCombatsService;
use crate::application::coude::combat::resolve_now::ResolveCombatNowService;
use crate::application::coude::manage_cashbox_service::ManageCoudeCashboxService;
use crate::application::coude::manage_tournaments_service::ManageTournamentsService;
use crate::application::coude::manage_catalog_service::ManageCoudeCatalogService;
use crate::application::coude::manage_curses_service::ManageCoudeCursesService;
use crate::application::coude::manage_economy_service::ManageCoudeEconomyService;
use crate::application::coude::manage_heist_service::ManageCoudeHeistService;
use crate::application::coude::manage_inventory_service::ManageCoudeInventoryService;
use crate::application::coude::manage_players_service::ManageCoudePlayersService;
use crate::application::coude::manage_safety_net_service::ManageCoudeSafetyNetService;
use crate::application::coude::manage_social_service::ManageCoudeSocialService;
use crate::application::coude::manage_taunts_service::ManageCoudeTauntsService;
use crate::application::coude::play_tout_ou_rien_service::PlayToutOuRienService;
use crate::application::coude::steal::manage_attempts::ManageStealAttemptsService;
use crate::application::coude::steal::manage_boosts::ManageCoudeStealBoostsService;
use crate::application::coude::steal::manage_protections::ManageCoudeStealProtectionsService;
use crate::application::coude::steal::resolve::ResolveStealService;
use crate::application::coude::steal::roll::RollStealService;
use crate::application::moderation::manage_infractions_service::ManageInfractionsService;
use crate::application::moderation::manage_moderation_service::ManageModerationService;
use crate::application::moderation::manage_notes_service::ManageNotesService;
use crate::application::moderation::manage_reminders_service::ManageRemindersService;
use crate::application::moderation::manage_rules_service::ManageRulesService;
use crate::application::moderation::manage_strikes_service::ManageStrikesService;
use crate::application::system::export_service::ExportService;
use crate::application::system::manage_tickets_service::ManageTicketsService;
use crate::config::AppConfig;

/// Construit l'etat complet de l'application (tous les repos + services).
/// Consomme le pool et le client Redis (via clones).
pub async fn build_app_state(
    config: &AppConfig,
    pg_pool: sqlx::PgPool,
    redis_client: redis::Client,
) -> AppState {
    // ── Adapters sortants ──
    let uow: Arc<dyn sentinel_core::ports::uow::UnitOfWork> = Arc::new(
        crate::adapters::outbound::postgres::uow::PgUnitOfWork::new(pg_pool.clone()),
    );
    let rule_repo = Arc::new(PgRuleRepository::new(pg_pool.clone()));
    let infraction_repo = Arc::new(PgInfractionRepository::new(pg_pool.clone()));
    let ticket_repo = Arc::new(PgTicketRepository::new(pg_pool.clone()));
    let security_repo = Arc::new(PgSecurityEventRepository::new(pg_pool.clone()));
    let moderation_repo = Arc::new(PgModerationRepository::new(pg_pool.clone()));
    let stats_repo = Arc::new(PgStatsRepository::new(pg_pool.clone()));
    let voice_channel_repo = Arc::new(PgVoiceChannelRepository::new(pg_pool.clone()));
    let age_ban_repo = Arc::new(
        crate::adapters::outbound::postgres::community::age_ban_repository::PgAgeBanRepository::new(
            pg_pool.clone(),
        ),
    );
    let bot_config_repo = Arc::new(PgBotConfigRepository::new(pg_pool.clone()));
    let guild_repo = Arc::new(PgGuildRepository::new(pg_pool.clone()));
    // Phase 5C — Batch writes : BatchedPgLogRepository bufferise les inserts et
    // flush via multi-row INSERT toutes les 500ms ou 100 entries.
    let log_repo = Arc::new(BatchedPgLogRepository::new(
        pg_pool.clone(),
        BatchWriterConfig::default(),
    ));
    // Use case lecture/purge des logs systeme — reutilise le meme repo batche.
    let system_logs_uc: Arc<dyn sentinel_core::ports::inbound::system::manage_system_logs::ManageSystemLogsUseCase> =
        Arc::new(sentinel_core::application::system::manage_system_logs_service::ManageSystemLogsService::new(
            log_repo.clone(),
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
    let broadcaster = crate::bootstrap::build_broadcaster(redis_client.clone());

    // ── Inference ONNX ──
    let (inference, tokenizer, inference_limiter) = crate::bootstrap::build_inference();

    // Discord API (un seul client partage).
    let discord_api: Arc<dyn crate::adapters::outbound::discord_api::DiscordApi> =
        Arc::new(DiscordApiService::new(config.discord_bot_token.clone()));

    // ── Services applicatifs ──
    // Buffer in-memory partage (tension de salon). Pas de persistance :
    // reset au restart bot, c'est OK car seulement les N derniers messages.
    let channel_tension_buffer = Arc::new(
        sentinel_core::domain::services::moderation::channel_tension::ChannelTensionBuffer::new(),
    );

    let analyze_uc = Arc::new(
        AnalyzeMessageService::new(
            rule_repo.clone(),
            infraction_repo.clone(),
            cache.clone(),
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
        bot_config_repo.clone(),
        inference_limiter.clone(),
    ));
    // Dataset IA : repo Postgres (SQL ai_dataset_messages) + use case (bornage
    // des filtres, validation des ids). Le handler ne fait que RBAC + map.
    let dataset_repo = Arc::new(
        crate::adapters::outbound::postgres::ai::dataset_repository::PgDatasetRepository::new(
            pg_pool.clone(),
        ),
    );
    let dataset_uc: Arc<dyn crate::ports::inbound::ai::manage_dataset::ManageDatasetUseCase> =
        Arc::new(
            crate::application::ai::manage_dataset_service::ManageDatasetService::new(dataset_repo),
        );

    // File de jobs IA : repo Postgres (SQL ai_jobs) + use case (validation
    // job_type/guild_id). Le handler ne fait que parse/map.
    let ai_job_repo = Arc::new(
        crate::adapters::outbound::postgres::ai::ai_job_repository::PgAiJobRepository::new(
            pg_pool.clone(),
        ),
    );
    let ai_jobs_uc: Arc<dyn crate::ports::inbound::ai::manage_ai_jobs::ManageAiJobsUseCase> =
        Arc::new(
            crate::application::ai::manage_ai_jobs_service::ManageAiJobsService::new(ai_job_repo),
        );

    let rules_uc = Arc::new(ManageRulesService::new(rule_repo.clone(), cache.clone()));
    let infractions_uc = Arc::new(ManageInfractionsService::new(infraction_repo.clone()));
    let tickets_uc = Arc::new(ManageTicketsService::new(
        ticket_repo.clone(),
        cache.clone(),
    ));
    // Phase 5C — Batch writes : idem que log_repo, pour les audit events.
    // Phase 1 dual-write : creation deplacee plus tot pour pouvoir injecter
    // audit_logs_uc dans security_uc et moderation_uc.
    let audit_log_repo = Arc::new(BatchedPgAuditLogRepository::new(
        pg_pool.clone(),
        BatchWriterConfig::default(),
    ));
    let audit_logs_uc = Arc::new(ManageAuditLogsService::new(audit_log_repo));

    // Detection d'anomalie de moderation (mass ban/delete/role). Le CALCUL
    // (fenetre glissante) vit dans un adapter memoire serveur ; la DECISION
    // (seuil + reset) dans le service coeur. Le bot n'agrege/ne decide plus.
    let anomaly_max_buffer = std::env::var("ANOMALY_DETECTOR_MAX_BUFFER_SIZE")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(500usize);
    let anomaly_eviction_target = std::env::var("ANOMALY_DETECTOR_EVICTION_TARGET")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(100usize);
    let anomaly_counter = Arc::new(
        crate::adapters::outbound::audit::in_memory_anomaly_counter::InMemoryAnomalyCounter::new(
            anomaly_max_buffer,
            anomaly_eviction_target,
        ),
    );
    let detect_anomaly_uc = Arc::new(
        sentinel_core::application::audit::detect_moderation_anomaly_service::DetectModerationAnomalyService::new(
            anomaly_counter,
        ),
    );

    // Rapport hebdomadaire agrege server-side : comptage postgres par event_type
    // sur 7 jours (remonte de l'ancien WeeklyTracker RAM du bot).
    let weekly_report_uc = Arc::new(
        sentinel_core::application::audit::get_weekly_report_service::GetWeeklyReportService::new(
            Arc::new(
                crate::adapters::outbound::postgres::audit::audit_event_counter::PgAuditEventCounter::new(
                    pg_pool.clone(),
                ),
            ),
        ),
    );

    let user_activity_repo: Arc<
        dyn crate::ports::outbound::audit::user_activity_repository::UserActivityRepository,
    > = Arc::new(PgUserActivityRepository::new(pg_pool.clone()));
    let welcome_config_repo: Arc<
        dyn crate::ports::outbound::community::welcome_config_repository::WelcomeConfigRepository,
    > = Arc::new(PgWelcomeConfigRepository::new(pg_pool.clone()));
    // Use case Welcome (Phase 3) — handlers HTTP/gRPC passent par ce port
    // inbound, jamais par le repo direct.
    let welcome_config_uc: Arc<dyn crate::ports::inbound::community::manage_welcome_config::ManageWelcomeConfigUseCase> =
        Arc::new(crate::application::community::manage_welcome_config_service::ManageWelcomeConfigService::new(
            welcome_config_repo.clone(),
        ));
    // Verification d'age : DECISION server-side (le bot n'execute que l'action
    // Discord). Lit la config welcome du serveur via le meme repo.
    let age_check_uc: Arc<dyn crate::ports::inbound::community::evaluate_age_declaration::EvaluateAgeDeclarationUseCase> =
        Arc::new(crate::application::community::evaluate_age_declaration_service::EvaluateAgeDeclarationService::new(
            welcome_config_repo.clone(),
        ));
    // Automod reviews (sync Discord <-> web).
    let automod_review_repo: Arc<dyn crate::ports::outbound::moderation::automod_review_repository::AutomodReviewRepository> = Arc::new(
        crate::adapters::outbound::postgres::moderation::automod_review_repository::PgAutomodReviewRepository::new(pg_pool.clone()),
    );
    let automod_adaptive_slowmode_repo: Arc<dyn crate::ports::outbound::moderation::adaptive_slowmode_repository::AdaptiveSlowmodeRepository> = Arc::new(
        crate::adapters::outbound::postgres::moderation::adaptive_slowmode_repository::PgAdaptiveSlowmodeRepository::new(pg_pool.clone()),
    );
    let automod_reviews_uc: Arc<dyn crate::ports::inbound::moderation::manage_automod_reviews::ManageAutomodReviewsUseCase> =
        Arc::new(crate::application::moderation::manage_automod_reviews_service::ManageAutomodReviewsService::new(
            automod_review_repo.clone(),
        ));

    // Reset complet d'un serveur (factory reset, owner-only).
    let reset_guild_uc: Arc<dyn crate::ports::inbound::system::reset_guild::ResetGuildUseCase> =
        Arc::new(crate::application::system::reset_guild_service::ResetGuildService::new(Arc::new(
            crate::adapters::outbound::postgres::system::guild_reset_repository::PgGuildResetRepository::new(pg_pool.clone()),
        )));

    let watched_user_repo = Arc::new(PgWatchedUserRepository::new(pg_pool.clone()));
    let security_uc = Arc::new(
        ManageSecurityService::new(
            security_repo.clone(),
            cache.clone(),
            watched_user_repo.clone(),
            bot_config_repo.clone(),
            moderation_repo.clone(),
        )
        .with_audit_logs_uc(audit_logs_uc.clone()
            as Arc<dyn crate::ports::inbound::audit::manage_audit_logs::ManageAuditLogsUseCase>),
    );
    // Note : la creation de moderation_uc est differee plus bas pour pouvoir
    // injecter strikes_uc via with_strikes_uc (log_action_with_strike).
    let service_registry: Arc<
        dyn sentinel_core::ports::outbound::system::service_registry::ServiceRegistry,
    > = Arc::new(
        crate::adapters::outbound::redis_service_registry::RedisServiceRegistry::new(
            redis_client.clone(),
        ),
    );
    let stats_uc = Arc::new(ManageStatsService::new(
        stats_repo.clone(),
        infraction_repo.clone(),
        cache.clone(),
        service_registry,
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
    let levels_uc = Arc::new(ManageLevelsService::new(level_repo, bot_config_repo.clone()));
    let announcement_repo = Arc::new(crate::adapters::outbound::postgres::community::announcement_repository::PgAnnouncementRepository::new(pg_pool.clone()));
    let announcements_uc: Arc<dyn crate::ports::inbound::community::manage_announcements::ManageAnnouncementsUseCase> = Arc::new(crate::application::community::manage_announcements_service::ManageAnnouncementsService::new(announcement_repo, bot_config_repo.clone()));
    let confession_repo = Arc::new(crate::adapters::outbound::postgres::community::confession_repository::PgConfessionRepository::new(pg_pool.clone()));
    let confessions_uc: Arc<
        dyn crate::ports::inbound::community::manage_confessions::ManageConfessionsUseCase,
    > = Arc::new(
        crate::application::community::manage_confessions_service::ManageConfessionsService::new(
            confession_repo,
        ),
    );
    let notes_uc = Arc::new(ManageNotesService::new(notes_repo));
    let reminders_uc = Arc::new(ManageRemindersService::new(reminder_repo));
    let strikes_uc = Arc::new(ManageStrikesService::new(strike_repo.clone()));
    // Copilote de moderation (lecture seule) : reutilise le use case strikes
    // (ladder d'escalade) + un port focalise pour l'historique & la
    // jurisprudence automod (anti-ancrage : exclut les reviews 'voting').
    let moderation_copilot_repo: Arc<
        dyn crate::ports::outbound::moderation::moderation_copilot_repository::ModerationCopilotRepository,
    > = Arc::new(
        crate::adapters::outbound::postgres::moderation::moderation_copilot_repository::PgModerationCopilotRepository::new(pg_pool.clone()),
    );
    let moderation_copilot_uc: Arc<
        dyn crate::ports::inbound::moderation::moderation_copilot::ModerationCopilotUseCase,
    > = Arc::new(
        crate::application::moderation::manage_moderation_copilot_service::ManageModerationCopilotService::new(
            strikes_uc.clone()
                as Arc<dyn crate::ports::inbound::moderation::manage_strikes::ManageStrikesUseCase>,
            moderation_copilot_repo,
        ),
    );
    let moderation_uc = Arc::new(
        ManageModerationService::new(moderation_repo.clone(), strike_repo.clone(), cache.clone())
            .with_strikes_uc(strikes_uc.clone()
                as Arc<dyn crate::ports::inbound::moderation::manage_strikes::ManageStrikesUseCase>)
            .with_audit_logs_uc(audit_logs_uc.clone()
                as Arc<
                    dyn crate::ports::inbound::audit::manage_audit_logs::ManageAuditLogsUseCase,
                >),
    );
    let member_repo = Arc::new(PgMemberRepository::new(pg_pool.clone()));
    let discord_role_repo = Arc::new(PgDiscordRoleRepository::new(pg_pool.clone()));
    let wallet_repo = Arc::new(PgWalletRepository::new(pg_pool.clone()));
    let blackjack_repo = Arc::new(PgBlackjackRepository::new(pg_pool.clone()));
    let blackjack_table_repo = Arc::new(PgBlackjackTableRepository::new(pg_pool.clone()));
    // `blackjack_svc` est instancie plus bas, apres la construction de
    // `wallet_uc` (dependance de la migration #4).
    let coude_player_repo = Arc::new(PgPlayerRepository::new(pg_pool.clone()));
    let coude_players_uc = Arc::new(ManageCoudePlayersService::new(coude_player_repo.clone()));
    let coude_combat_repo = Arc::new(PgCombatRepository::new(pg_pool.clone()));
    let coude_combats_uc: Arc<
        dyn crate::ports::inbound::coude::manage_combats::ManageCoudeCombatsUseCase,
    > = Arc::new(
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
    let coude_taunts_repo: Arc<
        dyn crate::ports::outbound::coude::taunts_repository::TauntsRepository,
    > = Arc::new(PgTauntsRepository::new(pg_pool.clone()));

    // Maledictions — repo cree tot pour pouvoir le brancher dans taunts
    // (effet Insomnia) et wheel (effet Heartbreak).
    let coude_curses_repo: Arc<
        dyn crate::ports::outbound::coude::curses_repository::CursesRepository,
    > = Arc::new(PgCursesRepository::new(pg_pool.clone()));

    // Filet de securite — repo cree tot pour le brancher dans bets/combat.
    let coude_safety_net_repo: Arc<
        dyn crate::ports::outbound::coude::safety_net_repository::SafetyNetRepository,
    > = Arc::new(PgSafetyNetRepository::new(pg_pool.clone()));

    let coude_taunts_uc: Arc<
        dyn crate::ports::inbound::coude::manage_taunts::ManageCoudeTauntsUseCase,
    > = Arc::new(
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
            member_repo.clone(),
            bot_config_repo.clone(),
        ));

    // Bump : repo Postgres + use case (recompense graduee, cooldown atomique,
    // credit wallet, seuil VIP). Toute la regle metier vit dans le service.
    let bump_repo = Arc::new(
        crate::adapters::outbound::postgres::community::bump_repository::PgBumpRepository::new(
            pg_pool.clone(),
        ),
    );
    let bump_uc: Arc<dyn crate::ports::inbound::community::manage_bump::ManageBumpUseCase> =
        Arc::new(
            crate::application::community::manage_bump_service::ManageBumpService::new(
                bot_config_repo.clone(),
                bump_repo,
                wallet_uc.clone(),
            ),
        );

    // Eligibilite Community : decisions server-side (prerequis de role +
    // validation de parrainage). Lit la config via bot_config_repo ; regles
    // pures dans le domaine. Le bot ne fournit que les donnees Discord.
    let eligibility_uc: Arc<
        dyn crate::ports::inbound::community::check_eligibility::CheckEligibilityUseCase,
    > = Arc::new(
        crate::application::community::check_eligibility_service::CheckEligibilityService::new(
            bot_config_repo.clone(),
        ),
    );

    // Classement mensuel : repo Postgres (deltas d'XP + baselines) + use case
    // (gates de publication, assemblage des tops, pose des baselines). Le
    // handler HTTP ne fait que RBAC + envoi Discord.
    let monthly_ranking_repo = Arc::new(
        crate::adapters::outbound::postgres::community::monthly_ranking_repository::PgMonthlyRankingRepository::new(
            pg_pool.clone(),
        ),
    );
    let monthly_ranking_uc: Arc<
        dyn crate::ports::inbound::community::manage_monthly_ranking::ManageMonthlyRankingUseCase,
    > = Arc::new(
        crate::application::community::manage_monthly_ranking_service::ManageMonthlyRankingService::new(
            bot_config_repo.clone(),
            monthly_ranking_repo,
        ),
    );

    // Snapshots analytics : repo Postgres (SQL des jobs) + use case (config par
    // guild, deltas de baseline, filtres de publication). Les handlers HTTP ne
    // font que declencher/RBAC/poster.
    let snapshot_repo = Arc::new(
        crate::adapters::outbound::postgres::audit::snapshot_repository::PgSnapshotRepository::new(
            pg_pool.clone(),
        ),
    );
    let snapshots_uc: Arc<
        dyn crate::ports::inbound::audit::manage_snapshots::ManageSnapshotsUseCase,
    > = Arc::new(
        crate::application::audit::manage_snapshots_service::ManageSnapshotsService::new(
            bot_config_repo.clone(),
            snapshot_repo,
            analytics_repo.clone(),
        ),
    );

    // Invitations : repo Postgres + use case (generation code unique, octroi de
    // role atomique au redeem). Le SQL vit dans le repo, la regle metier dans le
    // service, le handler HTTP ne fait que parse/RBAC/map.
    let invitation_repo = Arc::new(
        crate::adapters::outbound::postgres::system::invitation_repository::PgInvitationRepository::new(
            pg_pool.clone(),
        ),
    );
    let invitations_uc: Arc<
        dyn crate::ports::inbound::system::manage_invitations::ManageInvitationsUseCase,
    > = Arc::new(
        crate::application::system::manage_invitations_service::ManageInvitationsService::new(
            invitation_repo,
        ),
    );

    // Tamagotchi : repo + use case (debite les coins via le wallet partage).
    let pet_repo: Arc<dyn crate::ports::outbound::tamagotchi::pet_repository::PetRepository> =
        Arc::new(
            crate::adapters::outbound::postgres::tamagotchi::pet_repository::PgPetRepository::new(
                pg_pool.clone(),
            ),
        );
    let pets_uc: Arc<dyn crate::ports::inbound::tamagotchi::manage_pets::ManagePetsUseCase> =
        Arc::new(
            sentinel_core::application::tamagotchi::manage_pets_service::ManagePetsService::new(
                pet_repo.clone(),
                wallet_uc.clone(),
                bot_config_repo.clone(),
            ),
        );

    // Administrateur tournant : repo + use case.
    let rotation_repo: Arc<dyn crate::ports::outbound::system::admin_rotation_repository::AdminRotationRepository> =
        Arc::new(crate::adapters::outbound::postgres::system::admin_rotation_repository::PgAdminRotationRepository::new(pg_pool.clone()));
    let rotation_uc: Arc<
        dyn crate::ports::inbound::system::manage_rotation::ManageRotationUseCase,
    > = Arc::new(
        sentinel_core::application::system::manage_rotation_service::ManageRotationService::new(
            rotation_repo.clone(),
        ),
    );

    // Bans IP (panel securite) : repo DB + file-shim host + reader fail2ban.
    let ip_ban_repo: Arc<dyn crate::ports::outbound::system::ip_ban_repository::IpBanRepository> =
        Arc::new(
            crate::adapters::outbound::postgres::system::ip_ban_repository::PgIpBanRepository::new(
                pg_pool.clone(),
            ),
        );
    let host_ban_queue: Arc<dyn crate::ports::outbound::system::host_ban_queue::HostBanQueue> =
        Arc::new(crate::adapters::outbound::host_security::ban_queue::FileBanQueue::new());
    let fail2ban_reader: Arc<
        dyn crate::ports::outbound::system::host_ban_queue::Fail2banStatusReader,
    > = Arc::new(crate::adapters::outbound::host_security::fail2ban::Fail2banFileReader::new());
    let ip_bans_uc: Arc<dyn crate::ports::inbound::system::manage_ip_bans::ManageIpBansUseCase> =
        Arc::new(
            sentinel_core::application::system::manage_ip_bans_service::ManageIpBansService::new(
                ip_ban_repo,
                host_ban_queue,
                fail2ban_reader,
            ),
        );

    // Sondes de securite host (JSON cron) : reader fichier + use case pass-through.
    let host_probe_reader: Arc<
        dyn crate::ports::outbound::system::host_probe_reader::HostProbeReader,
    > = Arc::new(
        crate::adapters::outbound::host_security::probe_reader::FileHostProbeReader::new(),
    );
    let host_probe_uc: Arc<
        dyn crate::ports::inbound::system::read_host_probe::ReadHostProbeUseCase,
    > = Arc::new(
        sentinel_core::application::system::read_host_probe_service::ReadHostProbeService::new(
            host_probe_reader,
        ),
    );

    // Analyse des logs securite (top IPs, echecs d'auth, trafic).
    let security_log_repo: Arc<dyn crate::ports::outbound::system::security_log_repository::SecurityLogRepository> =
        Arc::new(crate::adapters::outbound::postgres::system::security_log_repository::PgSecurityLogRepository::new(pg_pool.clone()));
    let security_logs_uc: Arc<dyn crate::ports::inbound::system::read_security_logs::ReadSecurityLogsUseCase> =
        Arc::new(sentinel_core::application::system::read_security_logs_service::ReadSecurityLogsService::new(security_log_repo));

    // Audit & maintenance securite (journal d'audit, logins, purge des logs).
    let security_audit_repo: Arc<dyn crate::ports::outbound::system::security_audit_repository::SecurityAuditRepository> =
        Arc::new(crate::adapters::outbound::postgres::system::security_audit_repository::PgSecurityAuditRepository::new(pg_pool.clone()));
    let security_audit_uc: Arc<dyn crate::ports::inbound::system::manage_security_audit::ManageSecurityAuditUseCase> =
        Arc::new(sentinel_core::application::system::manage_security_audit_service::ManageSecurityAuditService::new(security_audit_repo));

    // OAuth Discord web : repo Postgres (sessions + logins) + use case. Le SQL
    // vit dans l'adapter ; l'echange HTTP avec Discord + CSRF/cookies restent
    // au handler (concern HTTP).
    let oauth_session_repo: Arc<dyn crate::ports::outbound::system::oauth_session_repository::OAuthSessionRepository> =
        Arc::new(crate::adapters::outbound::postgres::system::oauth_session_repository::PgOAuthSessionRepository::new(pg_pool.clone()));
    let oauth_uc: Arc<dyn crate::ports::inbound::system::manage_oauth::ManageOAuthUseCase> =
        Arc::new(sentinel_core::application::system::manage_oauth_service::ManageOAuthService::new(
            oauth_session_repo,
        ));

    // RBAC applicatif (CRUD roles owner) : repo Postgres + use case. Le SQL
    // (api_users / api_user_guilds) vit dans l'adapter, les garde-fous metier
    // (anti-lockout, dernier owner) dans le service ; le handler ne fait que
    // gate/parse/map. Distinct du middleware RBAC (resolution de role).
    let rbac_repo: Arc<dyn crate::ports::outbound::system::rbac_repository::RbacRepository> =
        Arc::new(crate::adapters::outbound::postgres::system::rbac_repository::PgRbacRepository::new(pg_pool.clone()));
    let rbac_admin_uc: Arc<dyn crate::ports::inbound::system::manage_rbac::ManageRbacUseCase> =
        Arc::new(sentinel_core::application::system::manage_rbac_service::ManageRbacService::new(
            rbac_repo,
        ));

    // Quarantaine de securite : repo Postgres (SQL security_quarantine_pending) +
    // use case (calcul du delai avant kick). Le handler ne fait que parse/RBAC/map.
    let quarantine_repo: Arc<dyn crate::ports::outbound::system::quarantine_repository::QuarantineRepository> =
        Arc::new(crate::adapters::outbound::postgres::system::quarantine_repository::PgQuarantineRepository::new(pg_pool.clone()));
    let quarantine_uc: Arc<dyn crate::ports::inbound::system::manage_quarantine::ManageQuarantineUseCase> =
        Arc::new(sentinel_core::application::system::manage_quarantine_service::ManageQuarantineService::new(
            quarantine_repo,
        ));

    // Lockdown de securite : repo Postgres (SQL security_lockdown_active) + use
    // case (calcul de l'expiration). Le handler ne fait que parse/RBAC/map.
    let lockdown_repo: Arc<dyn crate::ports::outbound::system::lockdown_repository::LockdownRepository> =
        Arc::new(crate::adapters::outbound::postgres::system::lockdown_repository::PgLockdownRepository::new(pg_pool.clone()));
    let lockdown_uc: Arc<dyn crate::ports::inbound::system::manage_lockdown::ManageLockdownUseCase> =
        Arc::new(sentinel_core::application::system::manage_lockdown_service::ManageLockdownService::new(
            lockdown_repo,
        ));

    // Slowmode de securite manuel : repo Postgres (SQL security_slowmode_active) +
    // use case (calcul de l'expiration). Le handler ne fait que parse/RBAC/map.
    // Distinct de l'automod adaptatif (moderation).
    let slowmode_repo: Arc<dyn crate::ports::outbound::system::slowmode_repository::SlowmodeRepository> =
        Arc::new(crate::adapters::outbound::postgres::system::slowmode_repository::PgSlowmodeRepository::new(pg_pool.clone()));
    let slowmode_uc: Arc<dyn crate::ports::inbound::system::manage_slowmode::ManageSlowmodeUseCase> =
        Arc::new(sentinel_core::application::system::manage_slowmode_service::ManageSlowmodeService::new(
            slowmode_repo,
        ));

    // Visibilite des composants UI par role : repo Postgres (SQL
    // rbac_component_visibility + transaction batch) + use case. Le handler ne
    // fait que parse/RBAC/valider/map.
    let component_visibility_repo: Arc<dyn crate::ports::outbound::system::component_visibility_repository::ComponentVisibilityRepository> =
        Arc::new(crate::adapters::outbound::postgres::system::component_visibility_repository::PgComponentVisibilityRepository::new(pg_pool.clone()));
    let component_visibility_uc: Arc<dyn crate::ports::inbound::system::manage_component_visibility::ManageComponentVisibilityUseCase> =
        Arc::new(sentinel_core::application::system::manage_component_visibility_service::ManageComponentVisibilityService::new(
            component_visibility_repo,
        ));

    // Overrides RBAC de min_role par composant sensible : repo Postgres (SQL
    // rbac_component_min_role) + use case. Le handler ne fait que RBAC/valider
    // (registry component_gates) puis mapper ; le cache Redis reste au handler.
    let component_min_role_repo: Arc<dyn crate::ports::outbound::system::component_min_role_repository::ComponentMinRoleRepository> =
        Arc::new(crate::adapters::outbound::postgres::system::component_min_role_repository::PgComponentMinRoleRepository::new(pg_pool.clone()));
    let component_min_role_uc: Arc<dyn crate::ports::inbound::system::manage_component_min_role::ManageComponentMinRoleUseCase> =
        Arc::new(sentinel_core::application::system::manage_component_min_role_service::ManageComponentMinRoleService::new(
            component_min_role_repo,
        ));

    // Persistance fire-and-forget des bots (streaks, etc.) : repo Postgres
    // (SQL user_levels) + use case pass-through. Le handler ne fait que
    // parser/valider/mapper.
    let bot_persistence_repo: Arc<dyn crate::ports::outbound::system::bot_persistence_repository::BotPersistenceRepository> =
        Arc::new(crate::adapters::outbound::postgres::system::bot_persistence_repository::PgBotPersistenceRepository::new(pg_pool.clone()));
    let bot_persistence_uc: Arc<dyn crate::ports::inbound::system::manage_bot_persistence::ManageBotPersistenceUseCase> =
        Arc::new(sentinel_core::application::system::manage_bot_persistence_service::ManageBotPersistenceService::new(
            bot_persistence_repo,
        ));

    // Audit serveur (server_events) : repo Postgres + use case (bornage des
    // filtres de lecture). Le handler ne fait que parse/RBAC/map.
    let server_event_repo: Arc<dyn crate::ports::outbound::system::server_event_repository::ServerEventRepository> =
        Arc::new(crate::adapters::outbound::postgres::system::server_event_repository::PgServerEventRepository::new(pg_pool.clone()));
    let server_events_uc: Arc<dyn crate::ports::inbound::system::manage_server_events::ManageServerEventsUseCase> =
        Arc::new(sentinel_core::application::system::manage_server_events_service::ManageServerEventsService::new(
            server_event_repo,
        ));

    // Cert TLS + GeoIP (infra externe : fichier/openssl + http ip-api).
    let tls_cert_reader: Arc<dyn crate::ports::outbound::system::tls_cert_reader::TlsCertReader> =
        Arc::new(crate::adapters::outbound::host_security::tls_cert::FileTlsCertReader::new());
    let tls_cert_uc: Arc<dyn crate::ports::inbound::system::read_tls_cert::ReadTlsCertUseCase> =
        Arc::new(
            sentinel_core::application::system::read_tls_cert_service::ReadTlsCertService::new(
                tls_cert_reader,
            ),
        );
    let geoip_lookup: Arc<dyn crate::ports::outbound::system::geoip_lookup::GeoIpLookup> =
        Arc::new(crate::adapters::outbound::geoip::IpApiGeoIpLookup::new());
    let geoip_uc: Arc<dyn crate::ports::inbound::system::lookup_geoip::LookupGeoIpUseCase> =
        Arc::new(
            sentinel_core::application::system::lookup_geoip_service::LookupGeoIpService::new(
                geoip_lookup,
            ),
        );

    // Migration #7 : bet repo instantie apres wallet_uc pour pouvoir
    // deleguer les mutations user_wallets via credit_tx/debit_tx.
    let coude_bet_repo = Arc::new(PgBetRepository::new(pg_pool.clone(), wallet_uc.clone()));
    // Bets ne depend que d'une lecture de combat — on injecte le narrow port
    // `CombatQueryRepository` (impl par `PgCombatRepository`) plutot que
    // le use case complet `ManageCoudeCombatsUseCase`. Cf. P0 #2 audit.
    let combat_query_repo: Arc<
        dyn crate::ports::outbound::coude::combat_query_repository::CombatQueryRepository,
    > = coude_combat_repo.clone();
    let coude_bets_uc = Arc::new(
        ManageCoudeBetsService::new(coude_bet_repo, combat_query_repo)
            .with_safety_net_repo(coude_safety_net_repo.clone())
            .with_bot_config_repo(bot_config_repo.clone()),
    );

    // Migration #4 : `blackjack_svc` passe ses mutations wallet (mise, cashout,
    // double down) par `wallet_uc` pour centralisation + detection auto des
    // taunts (faillite, jackpot). `wallet_repo` reste injecte pour
    // `get_or_create` au demarrage de la toute premiere partie.
    let blackjack_svc = Arc::new(
        BlackjackService::new(
            blackjack_repo,
            wallet_repo.clone(),
            wallet_uc.clone(),
            bot_config_repo.clone(),
        )
        .with_table_repo(blackjack_table_repo.clone()),
    );

    // Slot machine — nouvelle feature (migration 157).
    let slot_repo = Arc::new(
        crate::adapters::outbound::postgres::casino::slot_repository::PgSlotRepository::new(
            pg_pool.clone(),
        ),
    );
    let slot_uc: Arc<dyn crate::ports::inbound::casino::manage_slot::ManageSlotUseCase> = Arc::new(
        crate::application::casino::manage_slot_service::ManageSlotService::new(
            slot_repo,
            bot_config_repo.clone(),
            wallet_uc.clone(),
            uow.clone(),
        ),
    );

    // Roue du Destin — Sprint 2 sign'ature (migration 158).
    let wheel_repo = Arc::new(
        crate::adapters::outbound::postgres::casino::wheel_repository::PgWheelRepository::new(
            pg_pool.clone(),
        ),
    );
    let wheel_uc: Arc<dyn crate::ports::inbound::casino::manage_wheel::ManageWheelUseCase> =
        Arc::new(
            crate::application::casino::manage_wheel_service::ManageWheelService::new(
                wheel_repo,
                wallet_uc.clone(),
                uow.clone(),
            )
            .with_curses_repo(coude_curses_repo.clone())
            .with_bot_config_repo(bot_config_repo.clone()),
        );

    let coude_economy_uc = Arc::new(
        ManageCoudeEconomyService::new(
            coude_economy_repo.clone(),
            wallet_uc.clone(),
            coude_taunts_uc.clone(),
        )
        .with_leaky_wallet_support(wallet_repo.clone(), coude_curses_repo.clone())
        .with_player_repo(coude_player_repo.clone())
        .with_bot_config_repo(bot_config_repo.clone()),
    );
    let coude_inventory_repo = Arc::new(PgInventoryRepository::new(pg_pool.clone()));
    let coude_inventory_uc = Arc::new(
        ManageCoudeInventoryService::new(coude_inventory_repo)
            .with_bot_config_repo(bot_config_repo.clone()),
    );
    let coude_social_repo: Arc<
        dyn crate::ports::outbound::coude::social_repository::SocialRepository,
    > = Arc::new(PgSocialRepository::new(pg_pool.clone()));
    let coude_social_uc = Arc::new(ManageCoudeSocialService::new(
        coude_social_repo.clone(),
        coude_player_repo.clone(),
        coude_economy_repo.clone(),
        bot_config_repo.clone(),
        wallet_uc.clone(),
    ));

    // Phase 10 — braquage (depend de cashbox_repo, inventory_uc, wallet_repo).
    let coude_heist_repo: Arc<
        dyn crate::ports::outbound::coude::heist_repository::HeistRepository,
    > = Arc::new(PgHeistRepository::new(pg_pool.clone()));

    // Phase 2 refacto : use case dedie qui orchestre la resolution batch des
    // combats betting. Remplacera coude-worker/src/jobs/resolve_betting.rs
    // en Phase 3.
    let resolve_betting_batch_uc: Arc<
        dyn crate::ports::inbound::coude::resolve_betting_batch::ResolveBettingBatchUseCase,
    > = Arc::new(ResolveBettingBatchService::new(
        coude_combat_repo.clone(),
        coude_player_repo.clone(),
        wallet_repo.clone(),
        coude_bets_uc.clone(),
        coude_inventory_uc.clone(),
        coude_social_uc.clone(),
        coude_taunts_uc.clone(),
        bot_config_repo.clone(),
    ));
    let coude_cashbox_repo: Arc<
        dyn crate::ports::outbound::coude::cashbox_repository::CashboxRepository,
    > = Arc::new(PgCashboxRepository::new(pg_pool.clone()));
    let expire_combats_batch_uc: Arc<
        dyn crate::ports::inbound::coude::expire_combats_batch::ExpireCombatsBatchUseCase,
    > = Arc::new(ExpireCombatsBatchService::new(
        coude_combat_repo.clone(),
        coude_player_repo.clone(),
        wallet_repo.clone(),
        coude_cashbox_repo.clone(),
        coude_bets_uc.clone(),
    ));
    let coude_catalog_uc: Arc<
        dyn crate::ports::inbound::coude::manage_catalog::ManageCoudeCatalogUseCase,
    > = Arc::new(ManageCoudeCatalogService::new());
    let coude_cashbox_uc: Arc<
        dyn crate::ports::inbound::coude::manage_cashbox::ManageCoudeCashboxUseCase,
    > = Arc::new(ManageCoudeCashboxService::new(
        coude_cashbox_repo.clone(),
        wallet_repo.clone(),
    ));

    // Tournoi hebdomadaire — repo d'agregation + use case (assemblage classement).
    let tournament_repo: Arc<
        dyn crate::ports::outbound::coude::tournament_repository::TournamentRepository,
    > = Arc::new(PgTournamentRepository::new(pg_pool.clone()));
    let tournaments_uc: Arc<
        dyn crate::ports::inbound::coude::manage_tournaments::ManageTournamentsUseCase,
    > = Arc::new(ManageTournamentsService::new(
        tournament_repo,
        bot_config_repo.clone(),
    ));

    // Phase 10 — heist UC (depend de cashbox_repo + inventory_uc + wallet_repo).
    let coude_heist_uc: Arc<
        dyn crate::ports::inbound::coude::manage_heist::ManageCoudeHeistUseCase,
    > = Arc::new(
        ManageCoudeHeistService::new(
            coude_heist_repo.clone(),
            coude_cashbox_repo.clone(),
            coude_inventory_uc.clone(),
            wallet_repo.clone(),
            bot_config_repo.clone(),
            coude_social_repo.clone(),
        )
        .with_player_repo(coude_player_repo.clone()),
    );

    // Maledictions (cf. COUPE_AMELIORATIONS 5.1) — repo deja cree plus haut
    // pour permettre le branchement Heartbreak dans wheel.
    let coude_curses_uc: Arc<
        dyn crate::ports::inbound::coude::manage_curses::ManageCoudeCursesUseCase,
    > = Arc::new(
        ManageCoudeCursesService::new(coude_curses_repo.clone(), wallet_repo.clone())
            .with_bot_config_repo(bot_config_repo.clone()),
    );

    // Filet de securite (cf. COUPE_AMELIORATIONS 4.4) — repo deja cree
    // plus haut pour permettre le branchement dans bets et combat.
    let coude_safety_net_uc: Arc<
        dyn crate::ports::inbound::coude::manage_safety_net::ManageCoudeSafetyNetUseCase,
    > = Arc::new(
        ManageCoudeSafetyNetService::new(coude_safety_net_repo.clone())
            .with_bot_config_repo(bot_config_repo.clone()),
    );

    // Memorial des clodos / tout-ou-rien log (cf. COUPE_AMELIORATIONS 6.1).
    let coude_tout_ou_rien_repo: Arc<
        dyn crate::ports::outbound::coude::tout_ou_rien_repository::ToutOuRienRepository,
    > = Arc::new(PgToutOuRienRepository::new(pg_pool.clone()));

    // Phase 2 #1 audit : RNG /tout-ou-rien migre cote API.
    let play_tout_ou_rien_uc: Arc<
        dyn crate::ports::inbound::coude::play_tout_ou_rien::PlayToutOuRienUseCase,
    > = Arc::new(
        PlayToutOuRienService::new(
            coude_player_repo.clone(),
            wallet_uc.clone(),
            coude_social_repo.clone(),
            coude_tout_ou_rien_repo.clone(),
        )
        .with_bot_config_repo(bot_config_repo.clone()),
    );

    // Phase 2 #4 audit : RNG /voler (d20 + steal %) migre cote API.
    let roll_steal_uc: Arc<dyn crate::ports::inbound::coude::roll_steal::RollStealUseCase> =
        Arc::new(RollStealService::new().with_bot_config_repo(bot_config_repo.clone()));

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

    // Compteurs de refus / dette d honneur (cf. COUPE_AMELIORATIONS 5.3).
    let coude_refusal_count_repo: Arc<
        dyn crate::ports::outbound::coude::refusal_count_repository::RefusalCountRepository,
    > = Arc::new(PgRefusalCountRepository::new(pg_pool.clone()));

    let coude_steal_protection_repo: Arc<
        dyn crate::ports::outbound::coude::steal_protection_repository::StealProtectionRepository,
    > = Arc::new(PgStealProtectionRepository::new(pg_pool.clone()));
    let coude_steal_protections_uc: Arc<
        dyn crate::ports::inbound::coude::manage_steal_protections::ManageCoudeStealProtectionsUseCase,
    > = Arc::new(ManageCoudeStealProtectionsService::new(
        coude_steal_protection_repo,
    ));
    let coude_steal_boost_repo: Arc<
        dyn crate::ports::outbound::coude::steal_boost_repository::StealBoostRepository,
    > = Arc::new(PgStealBoostRepository::new(pg_pool.clone()));
    let coude_steal_boosts_uc: Arc<
        dyn crate::ports::inbound::coude::manage_steal_boosts::ManageCoudeStealBoostsUseCase,
    > = Arc::new(
        ManageCoudeStealBoostsService::new(coude_steal_boost_repo)
            .with_bot_config_repo(bot_config_repo.clone()),
    );
    // Resolution serveur-side complete du vol (ResolveStealUseCase) :
    // decide l'issue + calcule butin/penalite + mute les wallets. Le bot
    // devient un adaptateur mince (rend l'embed + dispatch railleries).
    let resolve_steal_uc: Arc<
        dyn crate::ports::inbound::coude::resolve_steal::ResolveStealUseCase,
    > = Arc::new(ResolveStealService::new(
        roll_steal_uc.clone(),
        coude_players_uc.clone()
            as Arc<dyn crate::ports::inbound::coude::manage_players::ManageCoudePlayersUseCase>,
        coude_economy_uc.clone(),
        coude_taunts_uc.clone(),
        coude_steal_protections_uc.clone(),
        coude_steal_boosts_uc.clone(),
        coude_flavor_templates_repo.clone(),
        bot_config_repo.clone(),
    ));

    // Phase 5 — tentatives /voler persistees (repo outbound + use case).
    let coude_steal_attempt_repo: Arc<
        dyn crate::ports::outbound::coude::steal_attempt_repository::StealAttemptRepository,
    > = Arc::new(PgStealAttemptRepository::new(pg_pool.clone()));
    let coude_steal_attempts_uc: Arc<
        dyn crate::ports::inbound::coude::manage_steal_attempts::ManageStealAttemptsUseCase,
    > = Arc::new(ManageStealAttemptsService::new(coude_steal_attempt_repo));
    let resolve_combat_now_uc: Arc<
        dyn crate::ports::inbound::coude::resolve_combat_now::ResolveCombatNowUseCase,
    > = Arc::new(
        ResolveCombatNowService::new(
            coude_combat_repo.clone(),
            coude_combats_uc.clone(),
            coude_players_uc.clone()
                as Arc<dyn crate::ports::inbound::coude::manage_players::ManageCoudePlayersUseCase>,
            wallet_repo.clone(),
            coude_bets_uc.clone(),
            coude_inventory_uc.clone(),
            coude_social_uc.clone(),
            coude_taunts_uc.clone(),
            bot_config_repo.clone(),
        )
        .with_curses_repo(coude_curses_repo.clone())
        .with_safety_net_repo(coude_safety_net_repo.clone()),
    );
    let resolve_friendly_duel_uc: Arc<
        dyn crate::ports::inbound::coude::resolve_friendly_duel::ResolveFriendlyDuelUseCase,
    > = Arc::new(
        crate::application::coude::combat::resolve_friendly_duel::ResolveFriendlyDuelService::new(
            coude_player_repo.clone(),
            coude_players_uc.clone()
                as Arc<dyn crate::ports::inbound::coude::manage_players::ManageCoudePlayersUseCase>,
            bot_config_repo.clone(),
        ),
    );
    let watched_users_uc = Arc::new(ManageWatchedUsersService::new(
        watched_user_repo,
        infractions_uc.clone(),
        moderation_uc.clone(),
        security_uc.clone(),
        notes_uc.clone(),
    ));

    let members_uc = Arc::new(ManageMembersService::new(
        member_repo,
        infractions_uc.clone(),
        moderation_uc.clone(),
        stats_uc.clone(),
    ));

    // ── Discord API service : instance deja creee plus haut.
    // On re-declare ici pour garder la variable accessible dans la suite du
    // bootstrap (AppState.discord_api).

    // ── Job client (queue Redis → worker) ──
    let queue_key =
        std::env::var("REDIS_QUEUE_KEY").unwrap_or_else(|_| "sentinel:jobs".to_string());
    let job_client = JobClient::new(redis_client.clone(), queue_key);

    // ── Game Portal (plateforme serveurs Docker) ──────────────────────
    let game_template_repo = Arc::new(
        crate::adapters::outbound::postgres::game::template_repository::PgGameTemplateRepository::new(pg_pool.clone()),
    );
    let game_template_settings_repo = Arc::new(
        crate::adapters::outbound::postgres::game::session_repository::PgGameTemplateSettingsRepository::new(pg_pool.clone()),
    );
    let game_session_reg_repo = Arc::new(
        crate::adapters::outbound::postgres::game::session_repository::PgGameSessionRegistrationRepository::new(pg_pool.clone()),
    );
    let game_server_repo = Arc::new(
        crate::adapters::outbound::postgres::game::server_repository::PgGameServerRepository::new(
            pg_pool.clone(),
        ),
    );
    let game_config_repo = Arc::new(
        crate::adapters::outbound::postgres::game::config_repository::PgGameServerConfigRepository::new(pg_pool.clone()),
    );
    let game_audit_repo = Arc::new(
        crate::adapters::outbound::postgres::game::audit_repository::PgGameAuditRepository::new(
            pg_pool.clone(),
        ),
    );
    let docker_client =
        match crate::adapters::outbound::game_runtime::docker_runtime::make_docker_client() {
            Ok(c) => Some(c),
            Err(e) => {
                tracing::warn!(error = %e, "Docker socket indisponible — Game Portal lifecycle inactif");
                None
            }
        };
    // Fallback : si Docker n'est pas dispo, on instancie quand meme un
    // adapter qui retournera Internal a chaque appel (les endpoints de
    // listing/detail continuent de marcher, seules create/start/etc. echouent).
    let container_runtime: Arc<
        dyn crate::ports::outbound::game::container_runtime::ContainerRuntime,
    > = match docker_client.clone() {
        Some(d) => Arc::new(
            crate::adapters::outbound::game_runtime::docker_runtime::DockerContainerRuntime::new(d),
        ),
        None => {
            Arc::new(crate::adapters::outbound::game_runtime::noop_runtime::NoopContainerRuntime)
        }
    };
    let rcon_client: Arc<dyn crate::ports::outbound::game::rcon_client::RconClient> = Arc::new(
        crate::adapters::outbound::game_runtime::rcon_minecraft::MinecraftRconClient::new(),
    );
    let port_allocator: Arc<dyn crate::ports::outbound::game::port_allocator::PortAllocator> =
        Arc::new(
            crate::adapters::outbound::game_runtime::redis_port_allocator::RedisPortAllocator::new(
                redis_client.clone(),
            ),
        );
    let game_templates_uc: Arc<
        dyn crate::ports::inbound::game::manage_game_templates::ManageGameTemplatesUseCase,
    > = Arc::new(
        crate::application::game::manage_templates_service::ManageGameTemplatesService::new(
            game_template_repo.clone(),
            bot_config_repo.clone(),
        ),
    );
    let game_session_repo: Arc<dyn crate::ports::outbound::game::player_session_repository::PlayerSessionRepository> = Arc::new(
        crate::adapters::outbound::postgres::game::player_session_repository::PgPlayerSessionRepository::new(pg_pool.clone()),
    );
    let game_server_repo_dyn: Arc<
        dyn crate::ports::outbound::game::game_server_repository::GameServerRepository,
    > = game_server_repo.clone();
    let game_template_repo_dyn: Arc<
        dyn crate::ports::outbound::game::game_template_repository::GameTemplateRepository,
    > = game_template_repo.clone();
    let game_audit_repo_dyn: Arc<
        dyn crate::ports::outbound::game::game_audit_repository::GameAuditRepository,
    > = game_audit_repo.clone();
    let game_servers_uc: Arc<
        dyn crate::ports::inbound::game::manage_game_servers::ManageGameServersUseCase,
    > = Arc::new(
        crate::application::game::manage_game_servers_service::ManageGameServersService {
            server_repo: game_server_repo,
            template_repo: game_template_repo,
            config_repo: game_config_repo,
            audit_repo: game_audit_repo,
            container_runtime: container_runtime.clone(),
            rcon_client: rcon_client.clone(),
            port_allocator: port_allocator.clone(),
            bot_config: bot_config_repo.clone(),
        },
    );

    // ── State ──
    let modstats_repo: Arc<
        dyn crate::ports::outbound::audit::modstats_repository::ModstatsRepository,
    > = Arc::new(PgModstatsRepository::new(pg_pool.clone()));
    let modstats_uc: Arc<
        dyn crate::ports::inbound::moderation::read_modstats::ReadModstatsUseCase,
    > = Arc::new(
        crate::application::moderation::read_modstats_service::ReadModstatsService::new(
            modstats_repo.clone(),
        ),
    );

    // ── Jeu Influence (Phase 1) ──
    let influence_citizen_repo: Arc<
        dyn sentinel_core::ports::outbound::influence::citizen_repository::CitizenRepository,
    > = Arc::new(
        crate::adapters::outbound::postgres::influence::citizen_repository::PgCitizenRepository::new(
            pg_pool.clone(),
        ),
    );
    let influence_rep_dims_repo: Arc<
        dyn sentinel_core::ports::outbound::influence::reputation_dims_repository::ReputationDimsRepository,
    > = Arc::new(
        crate::adapters::outbound::postgres::influence::reputation_dims_repository::PgReputationDimsRepository::new(
            pg_pool.clone(),
        ),
    );
    let influence_view_profile_uc: Arc<
        dyn sentinel_core::ports::inbound::influence::view_profile::ViewProfileUseCase,
    > = Arc::new(
        sentinel_core::application::influence::view_profile_service::ViewProfileService::new(
            influence_citizen_repo.clone(),
        )
        .with_bot_config_repo(bot_config_repo.clone())
        .with_wallet_repo(wallet_repo.clone())
        .with_rep_dims_repo(influence_rep_dims_repo.clone()),
    );
    let influence_org_repo: Arc<
        dyn sentinel_core::ports::outbound::influence::organization_repository::OrganizationRepository,
    > = Arc::new(
        crate::adapters::outbound::postgres::influence::organization_repository::PgOrganizationRepository::new(
            pg_pool.clone(),
        ),
    );
    let influence_membership_repo: Arc<
        dyn sentinel_core::ports::outbound::influence::membership_repository::MembershipRepository,
    > = Arc::new(
        crate::adapters::outbound::postgres::influence::membership_repository::PgMembershipRepository::new(
            pg_pool.clone(),
        ),
    );
    let influence_archive_repo: Arc<
        dyn sentinel_core::ports::outbound::influence::information_repository::ArchiveRepository,
    > = Arc::new(
        crate::adapters::outbound::postgres::influence::information_repository::PgArchiveRepository::new(
            pg_pool.clone(),
        ),
    );
    let influence_relation_repo: Arc<
        dyn sentinel_core::ports::outbound::influence::relation_repository::RelationRepository,
    > = Arc::new(
        crate::adapters::outbound::postgres::influence::relation_repository::PgRelationRepository::new(
            pg_pool.clone(),
        ),
    );
    let influence_law_repo: Arc<
        dyn sentinel_core::ports::outbound::influence::law_repository::LawRepository,
    > = Arc::new(
        crate::adapters::outbound::postgres::influence::law_repository::PgLawRepository::new(
            pg_pool.clone(),
        ),
    );
    let influence_orgs_uc: Arc<
        dyn sentinel_core::ports::inbound::influence::manage_organizations::ManageOrganizationsUseCase,
    > = Arc::new(
        sentinel_core::application::influence::manage_organizations_service::ManageOrganizationsService::new(
            influence_citizen_repo.clone(),
            influence_org_repo.clone(),
            influence_membership_repo.clone(),
        )
        .with_bot_config_repo(bot_config_repo.clone())
        .with_relation_repo(influence_relation_repo.clone())
        .with_archive_repo(influence_archive_repo.clone())
        .with_wallet_repo(wallet_repo.clone())
        .with_law_repo(influence_law_repo.clone())
        .with_rep_dims_repo(influence_rep_dims_repo.clone()),
    );
    let influence_archives_uc: Arc<
        dyn sentinel_core::ports::inbound::influence::read_archives::ReadArchivesUseCase,
    > = Arc::new(
        sentinel_core::application::influence::read_archives_service::ReadArchivesService::new(
            influence_archive_repo.clone(),
        )
        .with_bot_config_repo(bot_config_repo.clone()),
    );
    let influence_motion_repo: Arc<
        dyn sentinel_core::ports::outbound::influence::motion_repository::MotionRepository,
    > = Arc::new(
        crate::adapters::outbound::postgres::influence::motion_repository::PgMotionRepository::new(
            pg_pool.clone(),
        ),
    );
    let influence_vote_repo: Arc<
        dyn sentinel_core::ports::outbound::influence::motion_repository::VoteRepository,
    > = Arc::new(
        crate::adapters::outbound::postgres::influence::motion_repository::PgVoteRepository::new(
            pg_pool.clone(),
        ),
    );
    let influence_votes_uc: Arc<
        dyn sentinel_core::ports::inbound::influence::manage_votes::ManageVotesUseCase,
    > = Arc::new(
        sentinel_core::application::influence::manage_votes_service::ManageVotesService::new(
            influence_citizen_repo.clone(),
            influence_org_repo.clone(),
            influence_membership_repo.clone(),
            influence_motion_repo.clone(),
            influence_vote_repo.clone(),
        )
        .with_bot_config_repo(bot_config_repo.clone()),
    );
    let influence_movement_repo: Arc<
        dyn sentinel_core::ports::outbound::influence::movement_repository::MovementRepository,
    > = Arc::new(
        crate::adapters::outbound::postgres::influence::movement_repository::PgMovementRepository::new(
            pg_pool.clone(),
        ),
    );
    let influence_capital_uc: Arc<
        dyn sentinel_core::ports::inbound::influence::manage_capital::ManageCapitalUseCase,
    > = Arc::new(
        sentinel_core::application::influence::manage_capital_service::ManageCapitalService::new(
            influence_citizen_repo.clone(),
            influence_movement_repo.clone(),
        )
        .with_bot_config_repo(bot_config_repo.clone())
        .with_wallet_repo(wallet_repo.clone()),
    );
    let influence_laws_uc: Arc<
        dyn sentinel_core::ports::inbound::influence::manage_laws::ManageLawsUseCase,
    > = Arc::new(
        sentinel_core::application::influence::manage_laws_service::ManageLawsService::new(
            influence_citizen_repo.clone(),
            influence_law_repo.clone(),
            influence_vote_repo.clone(),
        )
        .with_bot_config_repo(bot_config_repo.clone())
        .with_archive_repo(influence_archive_repo.clone())
        .with_rep_dims_repo(influence_rep_dims_repo.clone()),
    );
    let influence_investigation_repo: Arc<
        dyn sentinel_core::ports::outbound::influence::information_repository::InvestigationRepository,
    > = Arc::new(
        crate::adapters::outbound::postgres::influence::information_repository::PgInvestigationRepository::new(
            pg_pool.clone(),
        ),
    );
    let influence_information_repo: Arc<
        dyn sentinel_core::ports::outbound::influence::information_repository::InformationRepository,
    > = Arc::new(
        crate::adapters::outbound::postgres::influence::information_repository::PgInformationRepository::new(
            pg_pool.clone(),
        ),
    );
    let influence_information_uc: Arc<
        dyn sentinel_core::ports::inbound::influence::manage_information::ManageInformationUseCase,
    > = Arc::new(
        sentinel_core::application::influence::manage_information_service::ManageInformationService::new(
            influence_citizen_repo.clone(),
            influence_investigation_repo.clone(),
            influence_information_repo.clone(),
            influence_archive_repo.clone(),
            influence_movement_repo.clone(),
        )
        .with_bot_config_repo(bot_config_repo.clone())
        .with_wallet_repo(wallet_repo.clone())
        .with_rep_dims_repo(influence_rep_dims_repo.clone()),
    );

    // ── Ban en sursis (moderation) ──
    let sursis_repo: Arc<
        dyn sentinel_core::ports::outbound::moderation::sursis_repository::SursisRepository,
    > = Arc::new(
        crate::adapters::outbound::postgres::moderation::sursis_repository::PgSursisRepository::new(
            pg_pool.clone(),
        ),
    );
    let sursis_uc: Arc<
        dyn sentinel_core::ports::inbound::moderation::manage_sursis::ManageSursisUseCase,
    > = Arc::new(
        sentinel_core::application::moderation::manage_sursis_service::ManageSursisService::new(
            sursis_repo,
        ),
    );

    AppState {
        analyze_uc,
        analyze_image_uc,
        dataset_uc,
        ai_jobs_uc,
        rules_uc,
        infractions_uc,
        tickets_uc,
        invitations_uc,
        quarantine_uc,
        lockdown_uc,
        slowmode_uc,
        component_visibility_uc,
        component_min_role_uc,
        bot_persistence_uc,
        server_events_uc,
        security_uc,
        moderation_uc,
        modstats_uc,
        stats_uc,
        voice_channels_uc,
        watched_users_uc,
        audit_logs_uc,
        detect_anomaly_uc,
        weekly_report_uc,
        snapshots_uc,
        levels_uc,
        announcements_uc,
        confessions_uc,
        role_panels_uc,
        notes_uc,
        bump_uc,
        eligibility_uc,
        monthly_ranking_uc,
        reminders_uc,
        strikes_uc,
        moderation_copilot_uc,
        members_uc,
        analytics_repo,
        daily_activity_repo,
        age_ban_repo,
        log_repo,
        system_logs_uc,
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
        coude_steal_attempts_uc,
        coude_taunts_uc,
        coude_heist_uc,
        coude_curses_uc,
        coude_safety_net_uc,
        tournaments_uc,
        coude_tout_ou_rien_repo,
        play_tout_ou_rien_uc,
        roll_steal_uc,
        resolve_steal_uc,
        coude_flavor_templates_repo,
        discord_action_messages_uc,
        coude_refusal_count_repo,
        broadcaster,
        job_client,
        discord_api,
        inference: inference.clone(),
        api_key: config.api_key.clone(),
        discord_bot_token: config.discord_bot_token.clone(),
        user_activity_repo: user_activity_repo.clone(),
        welcome_config_uc,
        age_check_uc,
        automod_reviews_uc,
        automod_adaptive_slowmode_repo,
        reset_guild_uc,
        pets_uc,
        rotation_uc,
        ip_bans_uc,
        host_probe_uc,
        security_logs_uc,
        security_audit_uc,
        oauth_uc,
        rbac_admin_uc,
        tls_cert_uc,
        geoip_uc,
        export_uc: Arc::new(ExportService::new(Arc::new(
            crate::adapters::outbound::postgres::system::export_repository::PgExportRepository::new(
                pg_pool.clone(),
            ),
        ))),
        export_jobs_uc: Arc::new(
            sentinel_core::application::system::manage_export_jobs_service::ManageExportJobsService::new(
                Arc::new(
                    crate::adapters::outbound::postgres::system::export_job_repository::PgExportJobRepository::new(
                        pg_pool.clone(),
                    ),
                ),
            ),
        ),
        evidence_repo: Arc::new(PgEvidenceRepository::new(pg_pool.clone())),
        review_repo: Arc::new(PgReviewRepository::new(pg_pool.clone())),
        modstats_repo,
        game_repo: Arc::new(PgGameRepository::new(pg_pool.clone())),
        sponsorship_repo: Arc::new(PgSponsorshipRepository::new(pg_pool.clone())),
        temp_role_repo: Arc::new(PgTempRoleRepository::new(pg_pool.clone())),
        manage_sponsorships_uc: Arc::new(
            crate::application::community::manage_sponsorships_service::ManageSponsorshipsService::new(
                Arc::new(PgSponsorshipRepository::new(pg_pool.clone())),
                Arc::new(PgTempRoleRepository::new(pg_pool.clone())),
            ),
        ),
        pending_action_repo: Arc::new(PgPendingActionRepository::new(pg_pool.clone())),
        blackjack_table_repo,
        game_servers_uc,
        game_templates_uc,
        game_server_repo: game_server_repo_dyn,
        game_template_repo: game_template_repo_dyn,
        game_template_settings_repo,
        game_session_reg_repo,
        game_audit_repo: game_audit_repo_dyn,
        game_session_repo,
        game_container_runtime: container_runtime,
        game_rcon_client: rcon_client,
        game_port_allocator: port_allocator,
        influence_view_profile_uc,
        influence_orgs_uc,
        influence_votes_uc,
        influence_capital_uc,
        influence_laws_uc,
        influence_information_uc,
        influence_archives_uc,
        sursis_uc,
        pg_pool: pg_pool.clone(),
        redis_client: redis_client.clone(),
        cache: Some(cache.clone()),
        superadmin_user_ids: Arc::new(config.superadmin_user_ids.clone()),
        discord_oauth_client_id: config.discord_oauth_client_id.clone(),
        discord_oauth_client_secret: config.discord_oauth_client_secret.clone(),
        discord_oauth_redirect_uri: config.discord_oauth_redirect_uri.clone(),
        web_front_url: config.web_front_url.clone(),
        container_monitor: Some(crate::adapters::outbound::system::container_monitor::spawn(
            pg_pool.clone(),
        )),
        rate_limiter: Some(Arc::new(
            crate::adapters::outbound::system::rate_limiter::RateLimiter::from_env(),
        )),
        rbac_global_gate: config.rbac_global_gate,
    }
}
