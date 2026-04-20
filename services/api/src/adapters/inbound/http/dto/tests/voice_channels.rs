use super::*;

    use super::*;
    use chrono::Utc;
    use uuid::Uuid;

    fn make_channel() -> VoiceChannel {
        VoiceChannel {
            id: Uuid::new_v4(),
            guild_id: "guild1".into(),
            owner_id: "owner1".into(),
            owner_name: "Owner".into(),
            channel_id: "chan1".into(),
            text_channel_id: Some("text1".into()),
            members_channel_id: Some("mem1".into()),
            queue_channel_id: None,
            category_id: Some("cat1".into()),
            channel_name: "Salon de Owner".into(),
            kind: crate::domain::value_objects::VoiceChannelKind::Private,
            visibility: "visible".into(),
            queue_enabled: false,
            locked: false,
            stage_enabled: false,
            member_limit: Some(10),
            status: Some("Cool".into()),
            channel_status: "open".into(),
            closed_at: None,
            created_at: Utc::now(),
        }
    }

    fn make_theme() -> VoiceChannelTheme {
        VoiceChannelTheme {
            id: Uuid::new_v4(),
            guild_id: "guild1".into(),
            name: "Gaming".into(),
            emoji: Some("🎮".into()),
            channel_name_template: "{user} Gaming".into(),
            member_limit: Some(5),
            visibility: "visible".into(),
            locked: false,
            queue_enabled: false,
            bitrate: Some(64000),
            slowmode_secs: Some(10),
            stage_enabled: true,
            is_default: true,
            sort_order: 0,
            created_at: Utc::now(),
        }
    }

    fn make_invite_link() -> VoiceChannelInviteLink {
        VoiceChannelInviteLink {
            id: Uuid::new_v4(),
            voice_channel_id: Uuid::new_v4(),
            guild_id: "guild1".into(),
            channel_id: "chan1".into(),
            created_by: "user1".into(),
            created_by_name: "User".into(),
            code: "ABCD1234".into(),
            max_uses: Some(5),
            current_uses: 2,
            expires_at: Utc::now() + chrono::Duration::hours(1),
            revoked: false,
            created_at: Utc::now(),
        }
    }

    // ── VoiceChannel → VoiceChannelResponseDto ──

    #[test]
    fn channel_to_dto_preserves_fields() {
        let ch = make_channel();
        let id = ch.id;
        let dto = VoiceChannelResponseDto::from(ch);
        assert_eq!(dto.id, id.to_string());
        assert_eq!(dto.guild_id, "guild1");
        assert_eq!(dto.kind, "private");
        assert_eq!(dto.member_limit, Some(10));
        assert!(!dto.stage_enabled);
    }

    #[test]
    fn channel_to_dto_formats_dates() {
        let ch = make_channel();
        let dto = VoiceChannelResponseDto::from(ch);
        assert!(dto.created_at.contains("T")); // RFC3339 format
        assert!(dto.closed_at.is_none());
    }

    #[test]
    fn channel_to_dto_closed_at_some() {
        let mut ch = make_channel();
        ch.closed_at = Some(Utc::now());
        let dto = VoiceChannelResponseDto::from(ch);
        assert!(dto.closed_at.is_some());
    }

    // ── VoiceChannelTheme → ThemeResponseDto ──

    #[test]
    fn theme_to_dto_preserves_all_fields() {
        let theme = make_theme();
        let dto = ThemeResponseDto::from(theme);
        assert_eq!(dto.name, "Gaming");
        assert_eq!(dto.emoji, Some("🎮".into()));
        assert_eq!(dto.member_limit, Some(5));
        assert_eq!(dto.bitrate, Some(64000));
        assert_eq!(dto.slowmode_secs, Some(10));
        assert!(dto.stage_enabled);
        assert!(dto.is_default);
    }

    #[test]
    fn theme_to_dto_none_optionals() {
        let mut theme = make_theme();
        theme.emoji = None;
        theme.member_limit = None;
        theme.bitrate = None;
        theme.slowmode_secs = None;
        let dto = ThemeResponseDto::from(theme);
        assert!(dto.emoji.is_none());
        assert!(dto.member_limit.is_none());
    }

    // ── VoiceChannelInviteLink → InviteLinkResponseDto ──

    #[test]
    fn invite_link_to_dto_preserves_fields() {
        let link = make_invite_link();
        let dto = InviteLinkResponseDto::from(link);
        assert_eq!(dto.code, "ABCD1234");
        assert_eq!(dto.max_uses, Some(5));
        assert_eq!(dto.current_uses, 2);
        assert!(!dto.revoked);
    }

    #[test]
    fn invite_link_to_dto_formats_dates() {
        let link = make_invite_link();
        let dto = InviteLinkResponseDto::from(link);
        assert!(dto.expires_at.contains("T"));
        assert!(dto.created_at.contains("T"));
    }

    // ── CreateThemeDto → CreateThemeCommand ──

    #[test]
    fn theme_dto_to_command_sets_empty_guild() {
        let dto = CreateThemeDto {
            name: "Test".into(),
            emoji: None,
            channel_name_template: "{user}".into(),
            member_limit: None,
            visibility: "visible".into(),
            locked: false,
            queue_enabled: false,
            bitrate: None,
            slowmode_secs: None,
            stage_enabled: false,
            is_default: false,
            sort_order: 0,
        };
        let cmd: CreateThemeCommand = dto.into();
        assert_eq!(cmd.guild_id, ""); // set by handler
        assert_eq!(cmd.name, "Test");
    }

    // ── VoiceChannelDetail → VoiceChannelDetailDto ──

    #[test]
    fn detail_to_dto_aggregates_all() {
        let detail = VoiceChannelDetail {
            channel: make_channel(),
            co_admins: vec![],
            bans: vec![],
            invite_links: vec![make_invite_link()],
        };
        let dto = VoiceChannelDetailDto::from(detail);
        assert!(dto.co_admins.is_empty());
        assert!(dto.bans.is_empty());
        assert_eq!(dto.invite_links.len(), 1);
        assert_eq!(dto.invite_links[0].code, "ABCD1234");
    }

    // ── Default functions ──

    #[test]
    fn default_kind_is_public() {
        assert_eq!(default_kind(), "public");
    }

    #[test]
    fn default_visibility_is_visible() {
        assert_eq!(default_visibility(), "visible");
    }

    #[test]
    fn default_channel_name_template_is_user() {
        assert_eq!(default_channel_name_template(), "{user}");
    }
