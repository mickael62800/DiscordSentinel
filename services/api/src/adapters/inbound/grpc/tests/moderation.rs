use super::*;

    use super::*;
    use chrono::TimeZone;
    use uuid::Uuid;
    use crate::domain::value_objects::ModerationGravity;

    fn ts() -> chrono::DateTime<chrono::Utc> {
        chrono::Utc.with_ymd_and_hms(2026, 1, 15, 12, 0, 0).unwrap()
    }

    fn sample_action() -> ModerationAction {
        ModerationAction {
            id: Uuid::nil(),
            guild_id: "g".into(),
            channel_id: "c".into(),
            moderator_id: "mod".into(),
            moderator_name: "Mod".into(),
            target_id: "u".into(),
            target_name: "Joe".into(),
            action_type: "warn".into(),
            reason: "spam".into(),
            gravity: Some(ModerationGravity::High),
            duration: Some(3600),
            created_at: ts(),
        }
    }

    #[test]
    fn moderation_action_to_proto_full_mapping() {
        let p = moderation_action_to_proto(sample_action());
        assert_eq!(p.guild_id, "g");
        assert_eq!(p.moderator_name, "Mod");
        assert_eq!(p.action_type, "warn");
        assert_eq!(p.reason, "spam");
        assert_eq!(p.gravity.as_deref(), Some("high"));
        assert_eq!(p.duration, Some(3600));
        assert_eq!(p.created_at, ts().to_rfc3339());
    }

    #[test]
    fn moderation_action_to_proto_no_gravity_no_duration() {
        let mut a = sample_action();
        a.gravity = None;
        a.duration = None;
        let p = moderation_action_to_proto(a);
        assert!(p.gravity.is_none());
        assert!(p.duration.is_none());
    }

    #[test]
    fn moderation_action_gravity_low_serialised() {
        let mut a = sample_action();
        a.gravity = Some(ModerationGravity::Low);
        let p = moderation_action_to_proto(a);
        assert_eq!(p.gravity.as_deref(), Some("low"));
    }

    #[test]
    fn user_history_to_proto_full_mapping() {
        let h = UserModerationHistory {
            target_id: "u".into(),
            target_name: "Joe".into(),
            total_warns: 3,
            total_mutes: 1,
            total_bans: 0,
            actions: vec![sample_action(), sample_action()],
        };
        let p = user_history_to_proto(h);
        assert_eq!(p.target_id, "u");
        assert_eq!(p.total_warns, 3);
        assert_eq!(p.total_mutes, 1);
        assert_eq!(p.total_bans, 0);
        assert_eq!(p.actions.len(), 2);
    }

    #[test]
    fn user_history_to_proto_empty_history() {
        let h = UserModerationHistory {
            target_id: "u".into(),
            target_name: "Clean".into(),
            total_warns: 0,
            total_mutes: 0,
            total_bans: 0,
            actions: vec![],
        };
        let p = user_history_to_proto(h);
        assert!(p.actions.is_empty());
    }
