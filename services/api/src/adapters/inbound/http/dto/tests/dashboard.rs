use super::*;
use crate::domain::entities::{DashboardStats, Guild, Infraction, LogEntry, ModerationAction, Rule};
use crate::domain::value_objects::{Action, DetectionFlags, FlagType, ModerationGravity};
use chrono::{TimeZone, Utc};
use uuid::Uuid;

fn flags() -> DetectionFlags {
    DetectionFlags { spam: false, insult: false, link: false, phishing: false }
}

fn sample_infraction(action: Action, duration: Option<u64>) -> Infraction {
    Infraction {
        id: Uuid::new_v4(),
        guild_id: "g".into(),
        channel_id: "c".into(),
        user_id: "u".into(),
        username: "alice".into(),
        message_id: "m".into(),
        content: "hi".into(),
        flags: flags(),
        score: 0.5,
        action,
        reason: "reason".into(),
        duration,
        created_at: Utc.with_ymd_and_hms(2024, 5, 1, 10, 0, 0).unwrap(),
    }
}

fn sample_action() -> ModerationAction {
    ModerationAction {
        id: Uuid::new_v4(),
        guild_id: "g".into(),
        channel_id: "c".into(),
        moderator_id: "mod".into(),
        moderator_name: "Mod".into(),
        target_id: "u".into(),
        target_name: "alice".into(),
        action_type: "ban".into(),
        reason: "r".into(),
        gravity: Some(ModerationGravity::High),
        duration: Some(7200),
        created_at: Utc::now(),
    }
}

fn sample_rule(flag_type: FlagType, warn: f64, delete: f64, mute: f64, ban: f64) -> Rule {
    let now = Utc::now();
    Rule {
        id: Uuid::new_v4(),
        guild_id: "g42".into(),
        flag_type,
        weight: 2.5,
        threshold_warn: warn,
        threshold_delete: delete,
        threshold_mute: mute,
        threshold_ban: ban,
        enabled: true,
        created_at: now,
        updated_at: now,
    }
}

#[test]
fn dashboard_stats_dto_preserves_all_fields() {
    let s = DashboardStats {
        total_servers: 10, total_users: 200, messages_today: 3000, infractions_today: 5,
        bots_online: 2, bots_total: 3, workers_online: 1, workers_total: 1,
        postgres_online: true, redis_online: false,
    };
    let dto = DashboardStatsDto::from(s);
    assert_eq!(dto.total_servers, 10);
    assert_eq!(dto.messages_today, 3000);
    assert!(dto.postgres_online);
    assert!(!dto.redis_online);
}

#[test]
fn log_entry_dto_formats_timestamp_rfc3339() {
    let e = LogEntry {
        id: Uuid::new_v4(),
        timestamp: Utc.with_ymd_and_hms(2024, 1, 2, 3, 4, 5).unwrap(),
        level: "INFO".into(),
        bot: "sentinel".into(),
        server: "g".into(),
        message: "msg".into(),
        category: "cat".into(),
        details: serde_json::json!({"k":"v"}),
    };
    let id = e.id.to_string();
    let dto = LogEntryDto::from(e);
    assert_eq!(dto.id, id);
    assert_eq!(dto.timestamp, "2024-01-02T03:04:05+00:00");
    assert_eq!(dto.details["k"], "v");
}

#[test]
fn create_log_dto_all_optional_except_message() {
    let dto: CreateLogDto = serde_json::from_value(serde_json::json!({"message": "hi"})).unwrap();
    assert_eq!(dto.message, "hi");
    assert!(dto.level.is_none());
    assert!(dto.bot.is_none());
    assert!(dto.details.is_none());
}

#[test]
fn dashboard_infraction_from_infraction_marks_automod_and_detection() {
    let inf = sample_infraction(Action::Warn, Some(600));
    let dto = DashboardInfractionDto::from(inf);
    assert_eq!(dto.moderator, "AutoMod");
    assert_eq!(dto.source, "detection");
    assert_eq!(dto.infraction_type, "warn");
    assert_eq!(dto.duration, Some(600));
    assert_eq!(dto.server, "g");
}

#[test]
fn dashboard_infraction_skips_none_duration_in_json() {
    let dto = DashboardInfractionDto::from(sample_infraction(Action::None, None));
    let v = serde_json::to_value(&dto).unwrap();
    assert!(v.get("duration").is_none());
}

#[test]
fn dashboard_infraction_from_action_marks_action_source() {
    let a = sample_action();
    let mod_name = a.moderator_name.clone();
    let dto = DashboardInfractionDto::from(a);
    assert_eq!(dto.source, "action");
    assert_eq!(dto.moderator, mod_name);
    assert_eq!(dto.infraction_type, "ban");
    assert_eq!(dto.duration, Some(7200));
}

#[test]
fn dashboard_rule_action_ban_when_threshold_ban_set() {
    let r = sample_rule(FlagType::Spam, 1.0, 2.0, 3.0, 4.0);
    let dto = DashboardRuleDto::from(r);
    assert_eq!(dto.action, "ban");
}

#[test]
fn dashboard_rule_action_mute_when_no_ban() {
    let r = sample_rule(FlagType::Insult, 1.0, 2.0, 3.0, 0.0);
    let dto = DashboardRuleDto::from(r);
    assert_eq!(dto.action, "mute");
}

#[test]
fn dashboard_rule_action_delete_when_only_delete_set() {
    let r = sample_rule(FlagType::Link, 1.0, 2.0, 0.0, 0.0);
    let dto = DashboardRuleDto::from(r);
    assert_eq!(dto.action, "delete");
}

#[test]
fn dashboard_rule_action_warn_when_no_thresholds() {
    let r = sample_rule(FlagType::Nsfw, 0.0, 0.0, 0.0, 0.0);
    let dto = DashboardRuleDto::from(r);
    assert_eq!(dto.action, "warn");
}

#[test]
fn dashboard_rule_labels_known_flags() {
    for (flag, expected) in [
        (FlagType::Spam, "Anti-Spam"),
        (FlagType::Insult, "Anti-Insulte"),
        (FlagType::Link, "Anti-Lien"),
        (FlagType::Phishing, "Anti-Hameconnage"),
        (FlagType::Nsfw, "Anti-NSFW"),
        (FlagType::Illicit, "Anti-Illicite"),
        (FlagType::Anger, "Detection colere"),
        (FlagType::Rage, "Detection rage"),
        (FlagType::Threat, "Detection menace"),
        (FlagType::Harassment, "Detection harcelement"),
    ] {
        let r = sample_rule(flag.clone(), 0.0, 0.0, 0.0, 0.0);
        let dto = DashboardRuleDto::from(r);
        assert!(dto.name.starts_with(expected), "{:?} -> {}", flag, dto.name);
        assert!(dto.description.contains(expected));
    }
}

#[test]
fn dashboard_rule_description_includes_guild_and_weight() {
    let r = sample_rule(FlagType::Spam, 0.0, 0.0, 0.0, 0.0);
    let dto = DashboardRuleDto::from(r);
    assert!(dto.description.contains("g42"));
    assert!(dto.description.contains("2.5"));
}

#[test]
fn guild_dto_preserves_fields() {
    let g = Guild {
        guild_id: "g".into(), name: "Guild".into(), icon: Some("i".into()), member_count: 42,
        registered_at: Utc::now(), updated_at: Utc::now(),
    };
    let dto = GuildDto::from(g);
    assert_eq!(dto.guild_id, "g");
    assert_eq!(dto.member_count, 42);
    assert_eq!(dto.icon.as_deref(), Some("i"));
}

#[test]
fn register_guild_dto_optional_fields() {
    let dto: RegisterGuildDto = serde_json::from_value(serde_json::json!({
        "guild_id": "g", "name": "Guild"
    })).unwrap();
    assert!(dto.icon.is_none());
    assert!(dto.member_count.is_none());
    assert!(dto.owner_id.is_none());
}

#[test]
fn guild_filter_params_empty_object() {
    let p: GuildFilterParams = serde_json::from_str("{}").unwrap();
    assert!(p.guild_id.is_none());
}
