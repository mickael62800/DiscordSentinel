use super::*;

    use super::*;
    use chrono::TimeZone;
    use uuid::Uuid;
    use crate::domain::value_objects::VoiceChannelKind;

    fn ts() -> chrono::DateTime<chrono::Utc> {
        chrono::Utc.with_ymd_and_hms(2026, 1, 15, 12, 0, 0).unwrap()
    }

    fn sample_channel(kind: VoiceChannelKind) -> VoiceChannel {
        VoiceChannel {
            id: Uuid::nil(),
            guild_id: "g".into(),
            owner_id: "u".into(),
            owner_name: "Joe".into(),
            channel_id: "ch".into(),
            text_channel_id: Some("t".into()),
            members_channel_id: Some("m".into()),
            queue_channel_id: None,
            category_id: Some("cat".into()),
            channel_name: "Salon Joe".into(),
            kind,
            visibility: "public".into(),
            queue_enabled: false,
            locked: false,
            stage_enabled: false,
            member_limit: Some(10),
            status: Some("active".into()),
            channel_status: "active".into(),
            closed_at: None,
            created_at: ts(),
        }
    }

    #[test]
    fn voice_channel_to_proto_public() {
        let p = voice_channel_to_proto(sample_channel(VoiceChannelKind::Public));
        assert_eq!(p.kind, "public");
        assert_eq!(p.member_limit, Some(10));
        assert_eq!(p.text_channel_id.as_deref(), Some("t"));
        assert!(p.queue_channel_id.is_none());
        assert_eq!(p.created_at, ts().to_rfc3339());
    }

    #[test]
    fn voice_channel_to_proto_private() {
        let p = voice_channel_to_proto(sample_channel(VoiceChannelKind::Private));
        assert_eq!(p.kind, "private");
    }

    #[test]
    fn voice_theme_to_proto_full_mapping() {
        use crate::domain::entities::VoiceChannelTheme;
        let id = Uuid::new_v4();
        let theme = VoiceChannelTheme {
            id,
            guild_id: "g1".into(),
            name: "Gaming".into(),
            emoji: Some("🎮".into()),
            channel_name_template: "{user}'s Game".into(),
            member_limit: Some(5),
            visibility: "visible".into(),
            locked: false,
            queue_enabled: true,
            bitrate: Some(96000),
            slowmode_secs: Some(10),
            stage_enabled: false,
            is_default: true,
            sort_order: 3,
            created_at: ts(),
        };
        let p = voice_theme_to_proto(theme);
        assert_eq!(p.id, id.to_string());
        assert_eq!(p.guild_id, "g1");
        assert_eq!(p.name, "Gaming");
        assert_eq!(p.emoji.as_deref(), Some("🎮"));
        assert_eq!(p.channel_name_template, "{user}'s Game");
        assert_eq!(p.member_limit, Some(5));
        assert_eq!(p.visibility, "visible");
        assert!(p.queue_enabled);
        assert_eq!(p.bitrate, Some(96000));
        assert_eq!(p.slowmode_secs, Some(10));
        assert!(p.is_default);
        assert_eq!(p.sort_order, 3);
        assert_eq!(p.created_at, ts().to_rfc3339());
    }

    #[test]
    fn voice_theme_to_proto_minimal_optionals() {
        use crate::domain::entities::VoiceChannelTheme;
        let theme = VoiceChannelTheme {
            id: Uuid::nil(),
            guild_id: "g".into(),
            name: "Basic".into(),
            emoji: None,
            channel_name_template: "{user}".into(),
            member_limit: None,
            visibility: "hidden".into(),
            locked: true,
            queue_enabled: false,
            bitrate: None,
            slowmode_secs: None,
            stage_enabled: true,
            is_default: false,
            sort_order: 0,
            created_at: ts(),
        };
        let p = voice_theme_to_proto(theme);
        assert!(p.emoji.is_none());
        assert!(p.member_limit.is_none());
        assert!(p.bitrate.is_none());
        assert!(p.slowmode_secs.is_none());
        assert!(p.locked);
        assert!(p.stage_enabled);
    }

    #[test]
    fn voice_channel_to_proto_locked_with_no_limit() {
        let mut c = sample_channel(VoiceChannelKind::Public);
        c.locked = true;
        c.member_limit = None;
        c.status = None;
        let p = voice_channel_to_proto(c);
        assert!(p.locked);
        assert!(p.member_limit.is_none());
        assert!(p.status.is_none());
    }
