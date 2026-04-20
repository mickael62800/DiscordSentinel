use super::*;

    use super::*;
    use chrono::TimeZone;
    use uuid::Uuid;

    fn ts() -> chrono::DateTime<chrono::Utc> {
        chrono::Utc.with_ymd_and_hms(2026, 1, 15, 12, 0, 0).unwrap()
    }

    fn sample_user_level() -> UserLevel {
        UserLevel {
            id: Uuid::nil(),
            guild_id: "g1".into(),
            user_id: "u1".into(),
            username: "alice".into(),
            xp: 500,
            level: 5,
            xp_text: 300,
            level_text: 3,
            xp_voice: 200,
            level_voice: 2,
            last_xp_at: ts(),
            created_at: ts(),
            updated_at: ts(),
        }
    }

    #[test]
    fn xp_source_from_proto_voice_maps_correctly() {
        assert_eq!(
            xp_source_from_proto(proto_common::XpSource::Voice as i32),
            XpSource::Voice
        );
    }

    #[test]
    fn xp_source_from_proto_text_maps_correctly() {
        assert_eq!(
            xp_source_from_proto(proto_common::XpSource::Text as i32),
            XpSource::Text
        );
    }

    #[test]
    fn xp_source_from_proto_unspecified_defaults_to_text() {
        assert_eq!(
            xp_source_from_proto(proto_common::XpSource::Unspecified as i32),
            XpSource::Text
        );
        // Valeur invalide -> Text aussi (fallback safe).
        assert_eq!(xp_source_from_proto(9999), XpSource::Text);
    }

    #[test]
    fn xp_source_opt_from_proto_distinguishes_unspecified() {
        assert_eq!(
            xp_source_opt_from_proto(proto_common::XpSource::Text as i32),
            Some(XpSource::Text)
        );
        assert_eq!(
            xp_source_opt_from_proto(proto_common::XpSource::Voice as i32),
            Some(XpSource::Voice)
        );
        assert_eq!(
            xp_source_opt_from_proto(proto_common::XpSource::Unspecified as i32),
            None,
            "Unspecified doit retourner None pour distinguer 'aucun filtre'"
        );
    }

    #[test]
    fn xp_source_to_proto_round_trip_text_voice() {
        assert_eq!(
            xp_source_to_proto(XpSource::Text),
            proto_common::XpSource::Text as i32
        );
        assert_eq!(
            xp_source_to_proto(XpSource::Voice),
            proto_common::XpSource::Voice as i32
        );
    }

    #[test]
    fn xp_source_to_proto_days_falls_back_to_text() {
        // Days n'existe pas en proto v1 — fallback Text pour compat.
        assert_eq!(
            xp_source_to_proto(XpSource::Days),
            proto_common::XpSource::Text as i32
        );
    }

    #[test]
    fn user_level_to_proto_full_mapping() {
        let u = sample_user_level();
        let p = user_level_to_proto(u);
        assert_eq!(p.guild_id, "g1");
        assert_eq!(p.user_id, "u1");
        assert_eq!(p.username, "alice");
        assert_eq!(p.xp, 500);
        assert_eq!(p.level, 5);
        assert_eq!(p.xp_text, 300);
        assert_eq!(p.level_text, 3);
        assert_eq!(p.xp_voice, 200);
        assert_eq!(p.level_voice, 2);
        assert_eq!(p.last_xp_at, ts().to_rfc3339());
        // xp_progress doit calculer xp_current/xp_needed coherents.
        assert!(p.xp_needed > 0);
    }

    #[test]
    fn level_reward_to_proto_full_mapping() {
        let r = LevelReward {
            id: Uuid::nil(),
            guild_id: "g".into(),
            level: 10,
            role_id: "role42".into(),
            source: XpSource::Voice,
        };
        let p = level_reward_to_proto(r);
        assert_eq!(p.id, Uuid::nil().to_string());
        assert_eq!(p.level, 10);
        assert_eq!(p.role_id, "role42");
        assert_eq!(p.source, proto_common::XpSource::Voice as i32);
    }

    #[test]
    fn add_xp_result_to_proto_levelup_with_reward() {
        let r = AddXpResult {
            user_level: sample_user_level(),
            leveled_up: true,
            old_level: 4,
            reward_role_id: Some("reward_role".into()),
            source: XpSource::Text,
        };
        let p = add_xp_result_to_proto(r);
        assert!(p.leveled_up);
        assert_eq!(p.old_level, 4);
        assert_eq!(p.reward_role_id.as_deref(), Some("reward_role"));
        assert_eq!(p.source, proto_common::XpSource::Text as i32);
        assert!(p.user.is_some());
        assert_eq!(p.user.unwrap().level, 5);
    }

    #[test]
    fn add_xp_result_to_proto_no_levelup_no_reward() {
        let r = AddXpResult {
            user_level: sample_user_level(),
            leveled_up: false,
            old_level: 5,
            reward_role_id: None,
            source: XpSource::Voice,
        };
        let p = add_xp_result_to_proto(r);
        assert!(!p.leveled_up);
        assert!(p.reward_role_id.is_none());
    }
