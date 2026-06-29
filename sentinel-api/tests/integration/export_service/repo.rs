//! Tests d'integration pour ExportService.execute() — couvre les 3 job_type
//! (infractions, audit_logs, moderation_actions) x 2 formats (csv, json) +
//! les branches d'erreur (job_type inconnu, format inconnu).

use sqlx::PgPool;
use uuid::Uuid;

use sentinel_api::application::system::export_service::ExecuteExportUseCase;
use sentinel_api::application::system::export_service::ExportService;
async fn pool() -> PgPool {
    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://sentinel_test:sentinel_test@localhost:5433/sentinel_test".into()
    });
    PgPool::connect(&url).await.unwrap()
}
fn fresh_id() -> String {
    format!(
        "{}",
        Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128
    )
}

async fn seed_infraction(p: &PgPool, guild: &str) {
    sqlx::query(
        "INSERT INTO infractions (id, guild_id, channel_id, user_id, username, message_id, \
          content, flags, score, action, reason, duration, created_at) \
         VALUES ($1, $2, 'c1', 'u1', 'alice', 'm1', 'bad', '{}'::jsonb, 0.7, 'warn', 'spam', NULL, NOW())"
    ).bind(Uuid::new_v4()).bind(guild).execute(p).await.unwrap();
}

async fn seed_audit_log(p: &PgPool, guild: &str) {
    sqlx::query(
        "INSERT INTO audit_logs (id, guild_id, event_type, actor_id, actor_name, \
          target_id, target_name, channel_id, channel_name, created_at) \
         VALUES ($1, $2, 'message_deleted', 'a1', 'Admin', 'u1', 'alice', 'c1', 'general', NOW())",
    )
    .bind(Uuid::new_v4())
    .bind(guild)
    .execute(p)
    .await
    .unwrap();
}

async fn seed_moderation_action(p: &PgPool, guild: &str) {
    sqlx::query(
        "INSERT INTO moderation_actions (id, guild_id, channel_id, moderator_id, moderator_name, \
          target_id, target_name, action_type, reason, duration, created_at) \
         VALUES ($1, $2, 'c1', 'm1', 'Mod', 'u1', 'alice', 'mute', 'spam', 300, NOW())",
    )
    .bind(Uuid::new_v4())
    .bind(guild)
    .execute(p)
    .await
    .unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn execute_unknown_job_type_returns_validation_error() {
    let svc = ExportService::new(pool().await);
    let err = svc
        .execute(&fresh_id(), "unknown_job", "csv", 100)
        .await
        .unwrap_err();
    assert!(format!("{err:?}").contains("job_type"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn execute_infractions_csv_includes_header_and_row() {
    let p = pool().await;
    let svc = ExportService::new(p.clone());
    let g = fresh_id();
    seed_infraction(&p, &g).await;
    let res = svc.execute(&g, "infractions", "csv", 100).await.unwrap();
    assert_eq!(res.row_count, 1);
    assert!(res.data.starts_with("id,channel_id,user_id,"));
    assert!(res.data.contains("alice"));
    assert!(res.data.contains("warn"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn execute_infractions_json_serializes_array() {
    let p = pool().await;
    let svc = ExportService::new(p.clone());
    let g = fresh_id();
    seed_infraction(&p, &g).await;
    let res = svc.execute(&g, "infractions", "json", 100).await.unwrap();
    assert_eq!(res.row_count, 1);
    assert!(res.data.starts_with('['));
    assert!(res.data.contains("\"action\":\"warn\""));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn execute_audit_logs_csv() {
    let p = pool().await;
    let svc = ExportService::new(p.clone());
    let g = fresh_id();
    seed_audit_log(&p, &g).await;
    let res = svc.execute(&g, "audit_logs", "csv", 100).await.unwrap();
    assert_eq!(res.row_count, 1);
    assert!(res.data.contains("event_type"));
    assert!(res.data.contains("message_deleted"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn execute_moderation_actions_json() {
    let p = pool().await;
    let svc = ExportService::new(p.clone());
    let g = fresh_id();
    seed_moderation_action(&p, &g).await;
    let res = svc
        .execute(&g, "moderation_actions", "json", 100)
        .await
        .unwrap();
    assert_eq!(res.row_count, 1);
    assert!(res.data.contains("\"action_type\":\"mute\""));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn execute_unknown_format_returns_validation_error() {
    let p = pool().await;
    let svc = ExportService::new(p.clone());
    let g = fresh_id();
    seed_infraction(&p, &g).await;
    let err = svc
        .execute(&g, "infractions", "xml", 100)
        .await
        .unwrap_err();
    assert!(format!("{err:?}").contains("format"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn execute_max_rows_clamped_to_minimum_one() {
    // max_rows <= 0 est clampe a 1 (min).
    let p = pool().await;
    let svc = ExportService::new(p.clone());
    let g = fresh_id();
    seed_infraction(&p, &g).await;
    seed_infraction(&p, &g).await;
    let res = svc.execute(&g, "infractions", "csv", 0).await.unwrap();
    assert_eq!(res.row_count, 1); // clampe a 1
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn execute_empty_guild_returns_header_only_csv() {
    let svc = ExportService::new(pool().await);
    let res = svc
        .execute(&fresh_id(), "infractions", "csv", 100)
        .await
        .unwrap();
    assert_eq!(res.row_count, 0);
    assert_eq!(res.data, "id,channel_id,user_id,username,message_id,content,score,action,reason,duration_secs,created_at\n");
}

// ── Couverture complete : moderation_actions csv + audit_logs json + unknown format pour chaque ──

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn execute_moderation_actions_csv() {
    let p = pool().await;
    let svc = ExportService::new(p.clone());
    let g = fresh_id();
    seed_moderation_action(&p, &g).await;
    let res = svc
        .execute(&g, "moderation_actions", "csv", 100)
        .await
        .unwrap();
    assert_eq!(res.row_count, 1);
    assert!(res.data.starts_with("id,moderator_id,"));
    assert!(res.data.contains("mute"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn execute_audit_logs_json() {
    let p = pool().await;
    let svc = ExportService::new(p.clone());
    let g = fresh_id();
    seed_audit_log(&p, &g).await;
    let res = svc.execute(&g, "audit_logs", "json", 100).await.unwrap();
    assert_eq!(res.row_count, 1);
    assert!(res.data.contains("\"event_type\":\"message_deleted\""));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn execute_audit_logs_unknown_format_returns_error() {
    let p = pool().await;
    let svc = ExportService::new(p.clone());
    let g = fresh_id();
    seed_audit_log(&p, &g).await;
    let err = svc.execute(&g, "audit_logs", "xml", 100).await.unwrap_err();
    assert!(format!("{err:?}").contains("format"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn execute_moderation_actions_unknown_format_returns_error() {
    let p = pool().await;
    let svc = ExportService::new(p.clone());
    let g = fresh_id();
    seed_moderation_action(&p, &g).await;
    let err = svc
        .execute(&g, "moderation_actions", "xml", 100)
        .await
        .unwrap_err();
    assert!(format!("{err:?}").contains("format"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn execute_max_rows_clamped_to_50k_cap() {
    // max_rows > 50_000 est clampe a 50_000. Sans donnees, on verifie juste que
    // l'appel reussit sans depassement numerique.
    let svc = ExportService::new(pool().await);
    let res = svc
        .execute(&fresh_id(), "infractions", "csv", 1_000_000)
        .await
        .unwrap();
    assert_eq!(res.row_count, 0);
}
