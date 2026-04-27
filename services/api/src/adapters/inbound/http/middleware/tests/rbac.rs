use super::*;

    

    #[test]
    fn role_ordering_hierarchy() {
        assert!(Role::Owner > Role::Admin);
        assert!(Role::Admin > Role::Moderator);
        assert!(Role::Moderator > Role::Viewer);
    }

    #[test]
    fn role_satisfies_own_level() {
        assert!(Role::Admin.satisfies(Role::Admin));
        assert!(Role::Owner.satisfies(Role::Admin));
        assert!(!Role::Moderator.satisfies(Role::Admin));
    }

    #[test]
    fn role_from_str_valid() {
        assert_eq!(Role::from_str("owner"), Some(Role::Owner));
        assert_eq!(Role::from_str("admin"), Some(Role::Admin));
        assert_eq!(Role::from_str("moderator"), Some(Role::Moderator));
        assert_eq!(Role::from_str("viewer"), Some(Role::Viewer));
    }

    #[test]
    fn role_from_str_invalid() {
        assert_eq!(Role::from_str("superuser"), None);
        assert_eq!(Role::from_str(""), None);
    }

    #[test]
    fn role_roundtrip() {
        for r in [Role::Viewer, Role::Moderator, Role::Admin, Role::Owner] {
            assert_eq!(Role::from_str(r.as_str()), Some(r));
        }
    }

    #[test]
    fn require_role_accepts_equal() {
        let ctx = RoleContext {
            discord_user_id: "1".into(),
            role: Some(Role::Admin),
            guild_id: Some("42".into()),
        };
        assert!(require_role(&ctx, Role::Admin).is_ok());
    }

    #[test]
    fn require_role_accepts_higher() {
        let ctx = RoleContext {
            discord_user_id: "1".into(),
            role: Some(Role::Owner),
            guild_id: Some("42".into()),
        };
        assert!(require_role(&ctx, Role::Admin).is_ok());
    }

    #[test]
    fn require_role_rejects_lower() {
        let ctx = RoleContext {
            discord_user_id: "1".into(),
            role: Some(Role::Viewer),
            guild_id: Some("42".into()),
        };
        assert_eq!(
            require_role(&ctx, Role::Admin),
            Err(StatusCode::FORBIDDEN)
        );
    }

    #[test]
    fn require_role_rejects_no_role() {
        let ctx = RoleContext {
            discord_user_id: "1".into(),
            role: None,
            guild_id: None,
        };
        assert_eq!(
            require_role(&ctx, Role::Viewer),
            Err(StatusCode::FORBIDDEN)
        );
    }

    #[test]
    fn extract_guild_id_from_snowflake() {
        assert_eq!(
            extract_guild_id_from_path("/api/coude/123456789012345678/players"),
            Some("123456789012345678".to_string())
        );
    }

    #[test]
    fn extract_guild_id_no_match() {
        assert_eq!(extract_guild_id_from_path("/api/health"), None);
    }

    // ── Tests des helpers check_role (Phase 7 B refactor) ────────────

    #[test]
    fn check_role_passes_through_when_none() {
        // Aucun RoleContext → l'appel passe sans erreur (cas bot/internal)
        let result = check_role(&None, Role::Owner, "msg");
        assert!(result.is_ok());
    }

    #[test]
    fn check_role_ok_when_sufficient() {
        use axum::Extension;
        let ctx = RoleContext {
            discord_user_id: "1".into(),
            role: Some(Role::Admin),
            guild_id: Some("42".into()),
        };
        let rbac = Some(Extension(ctx));
        assert!(check_role(&rbac, Role::Moderator, "msg").is_ok());
        assert!(check_role(&rbac, Role::Admin, "msg").is_ok());
    }

    #[test]
    fn check_role_forbidden_when_insufficient() {
        use axum::Extension;
        let ctx = RoleContext {
            discord_user_id: "1".into(),
            role: Some(Role::Moderator),
            guild_id: Some("42".into()),
        };
        let rbac = Some(Extension(ctx));
        let result = check_role(&rbac, Role::Admin, "admin requis");
        assert!(result.is_err());
        // On ne peut pas facilement assert sur le message car ApiError ne
        // derive pas PartialEq, mais la presence de l'erreur est suffisante.
    }

    #[test]
    fn check_role_forbidden_when_role_is_none() {
        // RoleContext present mais role = None (endpoint sans guild_id)
        use axum::Extension;
        let ctx = RoleContext {
            discord_user_id: "1".into(),
            role: None,
            guild_id: None,
        };
        let rbac = Some(Extension(ctx));
        let result = check_role(&rbac, Role::Viewer, "msg");
        assert!(result.is_err());
    }
