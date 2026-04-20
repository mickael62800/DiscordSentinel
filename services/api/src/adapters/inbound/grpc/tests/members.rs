use super::*;

    use super::*;
    use chrono::TimeZone;

    fn ts() -> DateTime<chrono::Utc> {
        chrono::Utc.with_ymd_and_hms(2026, 1, 15, 12, 0, 0).unwrap()
    }

    fn sample_member() -> GuildMember {
        GuildMember {
            guild_id: "g1".into(),
            user_id: "u1".into(),
            username: "alice".into(),
            display_name: Some("Alice".into()),
            avatar: Some("hash123".into()),
            roles: serde_json::json!(["role1", "role2"]),
            joined_at: Some(ts()),
            account_created: Some(ts()),
            is_bot: false,
            last_seen_at: Some(ts()),
        }
    }

    #[test]
    fn member_to_proto_full_mapping() {
        let p = member_to_proto(sample_member()).unwrap();
        assert_eq!(p.guild_id, "g1");
        assert_eq!(p.user_id, "u1");
        assert_eq!(p.username, "alice");
        assert_eq!(p.display_name.as_deref(), Some("Alice"));
        assert!(p.roles_json.contains("role1"));
        assert_eq!(p.joined_at, Some(ts().to_rfc3339()));
        assert!(!p.is_bot);
    }

    #[test]
    fn member_to_proto_with_none_dates() {
        let mut m = sample_member();
        m.joined_at = None;
        m.account_created = None;
        m.last_seen_at = None;
        m.display_name = None;
        m.avatar = None;
        let p = member_to_proto(m).unwrap();
        assert!(p.joined_at.is_none());
        assert!(p.account_created.is_none());
        assert!(p.last_seen_at.is_none());
        assert!(p.display_name.is_none());
        assert!(p.avatar.is_none());
    }

    #[test]
    fn member_round_trip_via_proto() {
        let original = sample_member();
        let p = member_to_proto(original.clone()).unwrap();
        let back = proto_to_member(p).unwrap();
        assert_eq!(back.guild_id, original.guild_id);
        assert_eq!(back.user_id, original.user_id);
        assert_eq!(back.username, original.username);
        assert_eq!(back.display_name, original.display_name);
        assert_eq!(back.is_bot, original.is_bot);
        assert_eq!(back.joined_at, original.joined_at);
        assert_eq!(back.roles, original.roles);
    }

    #[test]
    fn proto_to_member_invalid_roles_json_falls_back_to_empty_array() {
        let p = proto::GuildMember {
            guild_id: "g".into(),
            user_id: "u".into(),
            username: "x".into(),
            display_name: None,
            avatar: None,
            roles_json: "not a json".into(),
            joined_at: None,
            account_created: None,
            is_bot: false,
            last_seen_at: None,
        };
        let m = proto_to_member(p).unwrap();
        assert_eq!(m.roles, serde_json::Value::Array(vec![]));
    }

    #[test]
    fn parse_rfc3339_none_yields_none() {
        assert_eq!(parse_rfc3339(None).unwrap(), None);
    }

    #[test]
    fn parse_rfc3339_valid_date() {
        let s = ts().to_rfc3339();
        let parsed = parse_rfc3339(Some(s)).unwrap();
        assert_eq!(parsed, Some(ts()));
    }

    #[test]
    fn parse_rfc3339_invalid_returns_invalid_argument() {
        let err = parse_rfc3339(Some("not-a-date".into())).unwrap_err();
        assert_eq!(err.code(), tonic::Code::InvalidArgument);
        assert!(err.message().contains("date"));
    }
