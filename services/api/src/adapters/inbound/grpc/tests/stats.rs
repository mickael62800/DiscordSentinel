use super::*;

    use super::*;
    use chrono::TimeZone;
    use uuid::Uuid;

    fn ts() -> chrono::DateTime<chrono::Utc> {
        chrono::Utc.with_ymd_and_hms(2026, 1, 15, 12, 0, 0).unwrap()
    }

    fn sample_user_stats() -> UserStats {
        UserStats {
            id: Uuid::nil(),
            guild_id: "g".into(),
            user_id: "u".into(),
            username: "alice".into(),
            message_count: 1500,
            voice_seconds: 7200,
            updated_at: ts(),
        }
    }

    #[test]
    fn user_stats_to_proto_full_mapping() {
        let p = user_stats_to_proto(sample_user_stats());
        assert_eq!(p.user_id, "u");
        assert_eq!(p.message_count, 1500);
        assert_eq!(p.voice_seconds, 7200);
        assert_eq!(p.updated_at, ts().to_rfc3339());
    }

    #[test]
    fn guild_overview_to_proto_full_mapping() {
        let o = GuildStatsOverview {
            guild_id: "g1".into(),
            total_messages: 50000,
            total_voice_seconds: 360000,
            active_members: 200,
            total_infractions: 30,
            total_warns: 20,
            total_mutes: 8,
            total_bans: 2,
            top_members: vec![sample_user_stats(), sample_user_stats(), sample_user_stats()],
        };
        let p = guild_overview_to_proto(o);
        assert_eq!(p.guild_id, "g1");
        assert_eq!(p.total_messages, 50000);
        assert_eq!(p.total_voice_seconds, 360000);
        assert_eq!(p.active_members, 200);
        assert_eq!(p.total_warns + p.total_mutes + p.total_bans, 30);
        assert_eq!(p.top_members.len(), 3);
    }

    #[test]
    fn guild_overview_to_proto_empty_top_members() {
        let o = GuildStatsOverview {
            guild_id: "g".into(),
            total_messages: 0, total_voice_seconds: 0, active_members: 0,
            total_infractions: 0, total_warns: 0, total_mutes: 0, total_bans: 0,
            top_members: vec![],
        };
        let p = guild_overview_to_proto(o);
        assert!(p.top_members.is_empty());
    }
