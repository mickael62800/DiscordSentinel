use super::*;

    use super::*;
    use chrono::TimeZone;
    use uuid::Uuid;

    fn ts() -> chrono::DateTime<chrono::Utc> {
        chrono::Utc.with_ymd_and_hms(2026, 1, 15, 12, 0, 0).unwrap()
    }

    fn sample_panel() -> RolePanel {
        RolePanel {
            id: Uuid::nil(),
            guild_id: "g".into(),
            channel_id: "c".into(),
            message_id: Some("m".into()),
            title: "Roles".into(),
            description: "Choisis".into(),
            mode: "buttons".into(),
            max_roles: Some(3),
            enabled: true,
            created_at: ts(),
            updated_at: ts(),
        }
    }

    #[test]
    fn role_panel_to_proto_full_mapping() {
        let p = role_panel_to_proto(sample_panel());
        assert_eq!(p.id, Uuid::nil().to_string());
        assert_eq!(p.guild_id, "g");
        assert_eq!(p.channel_id, "c");
        assert_eq!(p.message_id.as_deref(), Some("m"));
        assert_eq!(p.title, "Roles");
        assert_eq!(p.mode, "buttons");
        assert_eq!(p.max_roles, Some(3));
        assert!(p.enabled);
    }

    #[test]
    fn role_panel_to_proto_optional_fields_none() {
        let mut panel = sample_panel();
        panel.message_id = None;
        panel.max_roles = None;
        panel.enabled = false;
        let p = role_panel_to_proto(panel);
        assert!(p.message_id.is_none());
        assert!(p.max_roles.is_none());
        assert!(!p.enabled);
    }

    #[test]
    fn role_panel_entry_to_proto_full_mapping() {
        let e = RolePanelEntry {
            id: Uuid::nil(),
            panel_id: Uuid::nil(),
            role_id: "r1".into(),
            role_name: "Gamer".into(),
            emoji: Some("🎮".into()),
            label: "Joueur".into(),
            style: "primary".into(),
            position: 2,
        };
        let p = role_panel_entry_to_proto(e);
        assert_eq!(p.role_id, "r1");
        assert_eq!(p.label, "Joueur");
        assert_eq!(p.style, "primary");
        assert_eq!(p.position, 2);
        assert_eq!(p.emoji.as_deref(), Some("🎮"));
    }

    #[test]
    fn role_panel_detail_to_proto_includes_entries() {
        let detail = RolePanelDetail {
            panel: sample_panel(),
            entries: vec![
                RolePanelEntry {
                    id: Uuid::nil(), panel_id: Uuid::nil(),
                    role_id: "a".into(), role_name: "A".into(),
                    emoji: None, label: "A".into(), style: "primary".into(), position: 0,
                },
                RolePanelEntry {
                    id: Uuid::nil(), panel_id: Uuid::nil(),
                    role_id: "b".into(), role_name: "B".into(),
                    emoji: None, label: "B".into(), style: "primary".into(), position: 1,
                },
            ],
        };
        let p = role_panel_detail_to_proto(detail);
        assert!(p.panel.is_some());
        assert_eq!(p.entries.len(), 2);
        assert_eq!(p.entries[0].role_id, "a");
        assert_eq!(p.entries[1].position, 1);
    }

    #[test]
    fn auto_role_to_proto_full_mapping() {
        let r = AutoRole {
            id: Uuid::nil(),
            guild_id: "g".into(),
            role_id: "r".into(),
            role_name: "Member".into(),
            delay_secs: 60,
            enabled: true,
        };
        let p = auto_role_to_proto(r);
        assert_eq!(p.role_id, "r");
        assert_eq!(p.role_name, "Member");
        assert_eq!(p.delay_secs, 60);
        assert!(p.enabled);
    }
