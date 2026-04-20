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
