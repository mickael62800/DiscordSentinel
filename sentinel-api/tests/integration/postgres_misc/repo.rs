//! Tests d'integration postgres pour 4 repos : audit_log, security_event,
//! notes, reminders, bot_config. Regroupes dans un seul binaire pour reduire
//! le cout de compilation de la crate de test.

use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use sentinel_api::adapters::outbound::postgres::audit::audit_log_repository::PgAuditLogRepository;
use sentinel_api::adapters::outbound::postgres::audit::security_event_repository::PgSecurityEventRepository;
use sentinel_api::adapters::outbound::postgres::moderation::notes_repository::PgNotesRepository;
use sentinel_api::adapters::outbound::postgres::moderation::reminder_repository::PgReminderRepository;
use sentinel_api::adapters::outbound::postgres::system::bot_config_repository::PgBotConfigRepository;
use sentinel_api::ports::inbound::audit::manage_audit_logs::AuditLogFilters;
use sentinel_api::ports::outbound::audit::audit_log_repository::AuditLogRepository;
use sentinel_api::ports::outbound::audit::security_event_repository::SecurityEventRepository;
use sentinel_api::ports::outbound::moderation::notes_repository::NotesRepository;
use sentinel_api::ports::outbound::moderation::reminder_repository::ReminderRepository;
use sentinel_api::ports::outbound::system::bot_config_repository::BotConfigRepository;
use sentinel_core::domain::entities::audit::audit_log::AuditLog;
use sentinel_core::domain::entities::audit::security_event::SecurityEvent;
use sentinel_core::domain::entities::moderation::action::sanction_reminder::SanctionReminder;
use sentinel_core::domain::entities::moderation::user_note::UserNote;
use sentinel_core::domain::entities::system::discord_ids::GuildId;
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

// ══════════════════════════════════════════════════════════
// AuditLog
// ══════════════════════════════════════════════════════════

fn audit_log(guild: &str, event: &str) -> AuditLog {
    AuditLog {
        id: Uuid::new_v4(),
        guild_id: guild.into(),
        event_type: event.into(),
        actor_id: Some("mod1".into()),
        actor_name: Some("Mod".into()),
        target_id: Some("user1".into()),
        target_name: Some("User".into()),
        channel_id: Some("chan1".into()),
        channel_name: Some("general".into()),
        details: serde_json::json!({"reason": "test"}),
        created_at: Utc::now(),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn audit_log_save_and_find_all() {
    let repo = PgAuditLogRepository::new(pool().await);
    let g = fresh_id();
    repo.save(&audit_log(&g, "mod_warn")).await.unwrap();
    repo.save(&audit_log(&g, "mod_ban")).await.unwrap();

    let logs = repo
        .find_all(
            Some(&g),
            &AuditLogFilters {
                event_type: None,
                actor_id: None,
                target_id: None,
                limit: 50,
                offset: 0,

                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(logs.len(), 2);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn audit_log_filter_by_event_type() {
    let repo = PgAuditLogRepository::new(pool().await);
    let g = fresh_id();
    repo.save(&audit_log(&g, "mod_warn")).await.unwrap();
    repo.save(&audit_log(&g, "mod_ban")).await.unwrap();

    let warns = repo
        .find_all(
            Some(&g),
            &AuditLogFilters {
                event_type: Some("mod_warn".into()),
                actor_id: None,
                target_id: None,
                limit: 50,
                offset: 0,

                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(warns.len(), 1);
    assert_eq!(warns[0].event_type, "mod_warn");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn audit_log_filter_combinations() {
    let repo = PgAuditLogRepository::new(pool().await);
    let g = fresh_id();
    let mut a = audit_log(&g, "mod_ban");
    a.actor_id = Some("admin1".into());
    a.target_id = Some("victim1".into());
    let mut b = audit_log(&g, "mod_ban");
    b.actor_id = Some("admin2".into());
    b.target_id = Some("victim2".into());
    repo.save(&a).await.unwrap();
    repo.save(&b).await.unwrap();
    let filtered = repo
        .find_all(
            Some(&g),
            &AuditLogFilters {
                event_type: None,
                actor_id: Some("admin1".into()),
                target_id: Some("victim1".into()),
                limit: 50,
                offset: 0,

                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(filtered.len(), 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn audit_log_delete_older_than_days() {
    let p = pool().await;
    let repo = PgAuditLogRepository::new(p.clone());
    let g = fresh_id();
    // Un log ancien (2 jours)
    let mut old_log = audit_log(&g, "mod_warn");
    old_log.created_at = Utc::now() - chrono::Duration::days(2);
    sqlx::query(
        "INSERT INTO audit_logs (id, guild_id, event_type, actor_id, actor_name, target_id, target_name, channel_id, channel_name, details, created_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)"
    ).bind(old_log.id).bind(old_log.guild_id.as_str()).bind(&old_log.event_type)
     .bind(&old_log.actor_id).bind(&old_log.actor_name)
     .bind(&old_log.target_id).bind(&old_log.target_name)
     .bind(&old_log.channel_id).bind(&old_log.channel_name)
     .bind(&old_log.details).bind(old_log.created_at)
     .execute(&p).await.unwrap();
    // Un log recent
    repo.save(&audit_log(&g, "mod_warn")).await.unwrap();

    let n = repo.delete_older_than_days(&g, 1).await.unwrap();
    assert_eq!(n, 1);
    let remaining = repo
        .find_all(
            Some(&g),
            &AuditLogFilters {
                event_type: None,
                actor_id: None,
                target_id: None,
                limit: 50,
                offset: 0,

                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(remaining.len(), 1);
}

// ══════════════════════════════════════════════════════════
// SecurityEvent (reads via audit_logs)
// ══════════════════════════════════════════════════════════

async fn seed_security_audit(p: &PgPool, guild: &str, severity: &str, user_ids: Vec<&str>) -> Uuid {
    let id = Uuid::new_v4();
    let details = serde_json::json!({
        "severity": severity,
        "description": "test event",
        "user_ids": user_ids,
    });
    sqlx::query(
        "INSERT INTO audit_logs (id, guild_id, event_type, details) \
         VALUES ($1, $2, 'security_raid', $3)",
    )
    .bind(id)
    .bind(guild)
    .bind(details)
    .execute(p)
    .await
    .unwrap();
    id
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn security_save_is_noop() {
    let repo = PgSecurityEventRepository::new(pool().await);
    repo.save(&SecurityEvent {
        id: Uuid::new_v4(),
        guild_id: "g".into(),
        event_type: "raid".into(),
        severity: "high".into(),
        description: "x".into(),
        user_ids: vec![],
        created_at: Utc::now(),
    })
    .await
    .unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn security_find_by_guild_scopes_and_strips_prefix() {
    let p = pool().await;
    let repo = PgSecurityEventRepository::new(p.clone());
    let g = fresh_id();
    seed_security_audit(&p, &g, "high", vec!["u1", "u2"]).await;
    let events = repo.find_by_guild(&g).await.unwrap();
    assert_eq!(events.len(), 1);
    // "security_raid" -> "raid"
    assert_eq!(events[0].event_type, "raid");
    assert_eq!(events[0].severity, "high");
    assert_eq!(events[0].user_ids, vec!["u1".to_string(), "u2".to_string()]);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn security_find_all_returns_recent() {
    let p = pool().await;
    let repo = PgSecurityEventRepository::new(p.clone());
    let g = fresh_id();
    seed_security_audit(&p, &g, "low", vec![]).await;
    let events = repo.find_all().await.unwrap();
    assert!(events.iter().any(|e| e.guild_id == GuildId::new(g.clone())));
}

// ══════════════════════════════════════════════════════════
// Notes
// ══════════════════════════════════════════════════════════

fn note(guild: &str, user: &str, content: &str) -> UserNote {
    UserNote {
        id: Uuid::new_v4(),
        guild_id: guild.into(),
        user_id: user.into(),
        author_id: "admin".into(),
        author_name: "Admin".into(),
        content: content.into(),
        category: "general".into(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn notes_save_and_find() {
    let repo = PgNotesRepository::new(pool().await);
    let g = fresh_id();
    let u = fresh_id();
    repo.save(&note(&g, &u, "Note 1")).await.unwrap();
    repo.save(&note(&g, &u, "Note 2")).await.unwrap();
    let notes = repo.find_by_user(&g, &u).await.unwrap();
    assert_eq!(notes.len(), 2);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn notes_delete_valid_uuid() {
    let repo = PgNotesRepository::new(pool().await);
    let g = fresh_id();
    let u = fresh_id();
    let n = note(&g, &u, "Note");
    repo.save(&n).await.unwrap();
    repo.delete(&n.id.to_string()).await.unwrap();
    assert!(repo.find_by_user(&g, &u).await.unwrap().is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn notes_delete_invalid_uuid_returns_not_found() {
    let repo = PgNotesRepository::new(pool().await);
    let err = repo.delete("not-a-uuid").await.unwrap_err();
    assert!(matches!(
        err,
        sentinel_core::domain::errors::DomainError::NotFound(_)
    ));
}

// ══════════════════════════════════════════════════════════
// Reminders
// ══════════════════════════════════════════════════════════

fn reminder(guild: &str, remind_in_seconds: i64) -> SanctionReminder {
    let now = Utc::now();
    SanctionReminder {
        id: Uuid::new_v4(),
        guild_id: guild.into(),
        moderator_id: "mod".into(),
        moderator_name: "Mod".into(),
        target_id: "target".into(),
        target_name: "Target".into(),
        action_type: "mute".into(),
        reason: "test".into(),
        action_id: Uuid::new_v4(),
        remind_at: now + chrono::Duration::seconds(remind_in_seconds),
        expires_at: now + chrono::Duration::seconds(remind_in_seconds + 3600),
        status: "pending".into(),
        created_at: now,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn reminder_find_pending_only_past_remind_at() {
    let repo = PgReminderRepository::new(pool().await);
    let g = fresh_id();
    let past = reminder(&g, -60); // past
    let future = reminder(&g, 3600);
    repo.save(&past).await.unwrap();
    repo.save(&future).await.unwrap();
    let pending = repo.find_pending().await.unwrap();
    assert!(pending.iter().any(|r| r.id == past.id));
    assert!(!pending.iter().any(|r| r.id == future.id));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn reminder_mark_sent_removes_from_pending() {
    let repo = PgReminderRepository::new(pool().await);
    let g = fresh_id();
    let past = reminder(&g, -60);
    repo.save(&past).await.unwrap();
    repo.mark_sent(past.id).await.unwrap();
    let pending = repo.find_pending().await.unwrap();
    assert!(!pending.iter().any(|r| r.id == past.id));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn reminder_cancel_for_action() {
    let repo = PgReminderRepository::new(pool().await);
    let g = fresh_id();
    let past = reminder(&g, -60);
    let action_id = past.action_id;
    repo.save(&past).await.unwrap();
    repo.cancel_for_action(action_id).await.unwrap();
    let pending = repo.find_pending().await.unwrap();
    assert!(!pending.iter().any(|r| r.id == past.id));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn reminder_find_by_guild_returns_all() {
    let repo = PgReminderRepository::new(pool().await);
    let g = fresh_id();
    repo.save(&reminder(&g, 60)).await.unwrap();
    repo.save(&reminder(&g, 120)).await.unwrap();
    let all = repo.find_by_guild(&g).await.unwrap();
    assert_eq!(all.len(), 2);
}

// ══════════════════════════════════════════════════════════
// BotConfig
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn bot_config_get_config_empty_when_none() {
    let repo = PgBotConfigRepository::new(pool().await);
    assert!(repo
        .get_config(&fresh_id(), "automod-bot")
        .await
        .unwrap()
        .is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn bot_config_set_and_get() {
    let repo = PgBotConfigRepository::new(pool().await);
    let g = fresh_id();
    repo.set_config(&g, "automod-bot", "jackpot_threshold", "5000")
        .await
        .unwrap();
    repo.set_config(&g, "automod-bot", "chaos_enabled", "true")
        .await
        .unwrap();
    let entries = repo.get_config(&g, "automod-bot").await.unwrap();
    assert_eq!(entries.len(), 2);
    let jackpot = entries
        .iter()
        .find(|e| e.config_key == "jackpot_threshold")
        .unwrap();
    assert_eq!(jackpot.config_value, "5000");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn bot_config_set_is_upsert() {
    let repo = PgBotConfigRepository::new(pool().await);
    let g = fresh_id();
    repo.set_config(&g, "bot1", "key", "v1").await.unwrap();
    repo.set_config(&g, "bot1", "key", "v2").await.unwrap();
    let entries = repo.get_config(&g, "bot1").await.unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].config_value, "v2");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn bot_config_get_all_aggregates_multiple_bots() {
    let repo = PgBotConfigRepository::new(pool().await);
    let g = fresh_id();
    repo.set_config(&g, "bot1", "k1", "v").await.unwrap();
    repo.set_config(&g, "bot2", "k2", "v").await.unwrap();
    let all = repo.get_all_config(&g).await.unwrap();
    assert_eq!(all.len(), 2);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn bot_config_delete() {
    let repo = PgBotConfigRepository::new(pool().await);
    let g = fresh_id();
    repo.set_config(&g, "bot1", "k", "v").await.unwrap();
    repo.delete_config(&g, "bot1", "k").await.unwrap();
    assert!(repo.get_config(&g, "bot1").await.unwrap().is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn bot_config_get_definitions_returns_seeded_bots() {
    // Les bot_definitions sont seeded par les migrations — au moins 1.
    let repo = PgBotConfigRepository::new(pool().await);
    let defs = repo.get_definitions().await.unwrap();
    assert!(!defs.is_empty());
}
