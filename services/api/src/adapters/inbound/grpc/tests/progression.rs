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

    // ── RPC tests avec mock ──

    use async_trait::async_trait;
    use std::sync::Arc;
    use std::sync::Mutex;
    use crate::adapters::inbound::ws::broadcaster::EventBroadcaster;
    use crate::ports::inbound::community::manage_levels::AddXpCommand;
    use crate::ports::inbound::community::manage_levels::ManageLevelsUseCase;
    use crate::ports::inbound::community::manage_levels::SaveLevelConfigCommand;
    use crate::domain::entities::community::level::LevelConfig;
    use crate::domain::errors::DomainError;

    #[derive(Default)]
    struct MockLevelsUc {
        add_xp_calls: Mutex<Vec<AddXpCommand>>,
        user_level_return: Mutex<Option<UserLevel>>,
        leaderboard_calls: Mutex<Vec<(Option<XpSource>, i64)>>,
        rewards_return: Mutex<Vec<LevelReward>>,
    }

    #[async_trait]
    impl ManageLevelsUseCase for MockLevelsUc {
        async fn get_config(&self, _: &str) -> Result<LevelConfig, DomainError> { unimplemented!() }
        async fn save_config(&self, _: SaveLevelConfigCommand) -> Result<LevelConfig, DomainError> { unimplemented!() }
        async fn add_xp(&self, cmd: AddXpCommand) -> Result<AddXpResult, DomainError> {
            let source = cmd.source;
            self.add_xp_calls.lock().unwrap().push(cmd);
            Ok(AddXpResult {
                user_level: sample_user_level(),
                leveled_up: false,
                old_level: 5,
                reward_role_id: None,
                source,
            })
        }
        async fn get_user_level(&self, _: &str, _: &str) -> Result<UserLevel, DomainError> {
            Ok(self.user_level_return.lock().unwrap().clone().unwrap_or_else(sample_user_level))
        }
        async fn get_leaderboard(&self, _: &str, limit: i64) -> Result<Vec<UserLevel>, DomainError> {
            self.leaderboard_calls.lock().unwrap().push((None, limit));
            Ok(vec![sample_user_level()])
        }
        async fn get_leaderboard_by_source(&self, _: &str, source: XpSource, limit: i64) -> Result<Vec<UserLevel>, DomainError> {
            self.leaderboard_calls.lock().unwrap().push((Some(source), limit));
            Ok(vec![])
        }
        async fn get_rewards(&self, _: &str) -> Result<Vec<LevelReward>, DomainError> {
            Ok(self.rewards_return.lock().unwrap().clone())
        }
        async fn get_rewards_by_source(&self, _: &str, _: XpSource) -> Result<Vec<LevelReward>, DomainError> { Ok(vec![]) }
        async fn set_reward(&self, _: &str, _: i32, _: &str, _: XpSource) -> Result<LevelReward, DomainError> { unimplemented!() }
        async fn delete_reward(&self, _: &str, _: i32, _: XpSource) -> Result<(), DomainError> { Ok(()) }
    }

    fn grpc(uc: Arc<MockLevelsUc>) -> ProgressionGrpc {
        ProgressionGrpc {
            levels_uc: uc,
            broadcaster: Arc::new(EventBroadcaster::new()),
        }
    }

    #[tokio::test]
    async fn add_xp_delegates_to_uc_with_source() {
        let uc = Arc::new(MockLevelsUc::default());
        let g = grpc(uc.clone());
        let _ = g.add_xp(Request::new(proto::AddXpRequest {
            guild_id: "g".into(),
            user_id: "u".into(),
            username: "alice".into(),
            amount: 250,
            source: proto_common::XpSource::Voice as i32,
        })).await.unwrap();
        let calls = uc.add_xp_calls.lock().unwrap();
        assert_eq!(calls[0].amount, 250);
        assert_eq!(calls[0].source, XpSource::Voice);
    }

    #[tokio::test]
    async fn add_xp_unspecified_source_defaults_to_text() {
        let uc = Arc::new(MockLevelsUc::default());
        let g = grpc(uc.clone());
        let _ = g.add_xp(Request::new(proto::AddXpRequest {
            guild_id: "g".into(),
            user_id: "u".into(),
            username: "a".into(),
            amount: 10,
            source: proto_common::XpSource::Unspecified as i32,
        })).await.unwrap();
        assert_eq!(uc.add_xp_calls.lock().unwrap()[0].source, XpSource::Text);
    }

    #[tokio::test]
    async fn get_user_level_returns_proto() {
        let g = grpc(Arc::new(MockLevelsUc::default()));
        let resp = g.get_user_level(Request::new(proto::GetUserLevelRequest {
            guild_id: "g1".into(), user_id: "u1".into(),
        })).await.unwrap();
        let u = resp.into_inner();
        assert_eq!(u.xp, 500); // sample_user_level
    }

    #[tokio::test]
    async fn get_leaderboard_default_limit_when_zero() {
        let uc = Arc::new(MockLevelsUc::default());
        let g = grpc(uc.clone());
        let _ = g.get_leaderboard(Request::new(proto::GetLeaderboardRequest {
            guild_id: "g".into(),
            limit: 0,
            source: proto_common::XpSource::Unspecified as i32,
        })).await.unwrap();
        let calls = uc.leaderboard_calls.lock().unwrap();
        assert_eq!(calls[0].1, 25); // default 25 quand limit=0
    }

    #[tokio::test]
    async fn get_leaderboard_caps_at_100() {
        let uc = Arc::new(MockLevelsUc::default());
        let g = grpc(uc.clone());
        let _ = g.get_leaderboard(Request::new(proto::GetLeaderboardRequest {
            guild_id: "g".into(),
            limit: 500,
            source: proto_common::XpSource::Unspecified as i32,
        })).await.unwrap();
        let calls = uc.leaderboard_calls.lock().unwrap();
        assert_eq!(calls[0].1, 100);
    }

    #[tokio::test]
    async fn get_leaderboard_with_source_filter_delegates_to_by_source() {
        let uc = Arc::new(MockLevelsUc::default());
        let g = grpc(uc.clone());
        let _ = g.get_leaderboard(Request::new(proto::GetLeaderboardRequest {
            guild_id: "g".into(),
            limit: 50,
            source: proto_common::XpSource::Voice as i32,
        })).await.unwrap();
        let calls = uc.leaderboard_calls.lock().unwrap();
        assert_eq!(calls[0].0, Some(XpSource::Voice));
        assert_eq!(calls[0].1, 50);
    }

    #[tokio::test]
    async fn get_rewards_delegates_to_uc() {
        let uc = Arc::new(MockLevelsUc::default());
        uc.rewards_return.lock().unwrap().push(LevelReward {
            id: Uuid::new_v4(),
            guild_id: "g".into(),
            level: 10,
            role_id: "r".into(),
            source: XpSource::Text,
        });
        let g = grpc(uc);
        let resp = g.get_rewards(Request::new(proto::GetRewardsRequest {
            guild_id: "g".into(),
        })).await.unwrap();
        assert_eq!(resp.into_inner().rewards.len(), 1);
    }
