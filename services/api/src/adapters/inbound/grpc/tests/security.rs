use super::*;

    use super::*;
    use chrono::TimeZone;
    use uuid::Uuid;

    fn ts() -> chrono::DateTime<chrono::Utc> {
        chrono::Utc.with_ymd_and_hms(2026, 1, 15, 12, 0, 0).unwrap()
    }

    #[test]
    fn security_event_to_proto_full_mapping() {
        let e = SecurityEvent {
            id: Uuid::nil(),
            guild_id: "g".into(),
            event_type: "raid".into(),
            severity: "critical".into(),
            description: "Mass join detected".into(),
            user_ids: vec!["u1".into(), "u2".into(), "u3".into()],
            created_at: ts(),
        };
        let p = security_event_to_proto(e);
        assert_eq!(p.id, Uuid::nil().to_string());
        assert_eq!(p.guild_id, "g");
        assert_eq!(p.event_type, "raid");
        assert_eq!(p.severity, "critical");
        assert_eq!(p.description, "Mass join detected");
        assert_eq!(p.user_ids.len(), 3);
        assert_eq!(p.user_ids[1], "u2");
        assert_eq!(p.created_at, ts().to_rfc3339());
    }

    #[test]
    fn security_event_to_proto_no_users() {
        let e = SecurityEvent {
            id: Uuid::nil(),
            guild_id: "g".into(),
            event_type: "scan".into(),
            severity: "info".into(),
            description: String::new(),
            user_ids: vec![],
            created_at: ts(),
        };
        let p = security_event_to_proto(e);
        assert!(p.user_ids.is_empty());
        assert_eq!(p.severity, "info");
    }
