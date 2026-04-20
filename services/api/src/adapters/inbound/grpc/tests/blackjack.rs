use super::*;

    use super::*;
    use chrono::TimeZone;
    use uuid::Uuid;

    fn ts() -> chrono::DateTime<chrono::Utc> {
        chrono::Utc.with_ymd_and_hms(2026, 1, 15, 12, 0, 0).unwrap()
    }

    #[test]
    fn game_is_over_recognises_terminal_states() {
        for s in &[
            "player_blackjack",
            "player_bust",
            "dealer_bust",
            "player_win",
            "dealer_win",
            "push",
        ] {
            assert!(game_is_over(s), "{s} doit etre terminal");
        }
    }

    #[test]
    fn game_is_over_rejects_in_progress_states() {
        for s in &["in_progress", "waiting", "", "unknown"] {
            assert!(!game_is_over(s), "{s} ne doit PAS etre terminal");
        }
    }

    #[test]
    fn parse_uuid_valid_ok() {
        let id = Uuid::new_v4();
        assert_eq!(parse_uuid(&id.to_string()).unwrap(), id);
    }

    #[test]
    fn parse_uuid_invalid_returns_invalid_argument() {
        let err = parse_uuid("not-a-uuid").unwrap_err();
        assert_eq!(err.code(), tonic::Code::InvalidArgument);
        assert!(err.message().contains("UUID"));
    }

    #[test]
    fn card_to_proto_includes_filename() {
        let card = Card { rank: "As".into(), suit: "hearts".into() };
        let p = card_to_proto(&card);
        assert_eq!(p.rank, "As");
        assert_eq!(p.suit, "hearts");
        assert_eq!(p.filename, "As_hearts.jpg");
    }

    #[test]
    fn blackjack_game_to_proto_full_mapping() {
        let g = BlackjackGame {
            id: Uuid::nil(),
            guild_id: "g".into(),
            user_id: "u".into(),
            username: "Joe".into(),
            bet: 100,
            player_hand: vec![
                Card { rank: "10".into(), suit: "spades".into() },
                Card { rank: "As".into(), suit: "hearts".into() },
            ],
            dealer_hand: vec![Card { rank: "King".into(), suit: "clubs".into() }],
            deck: vec![],
            status: "player_blackjack".into(),
            player_score: 21,
            dealer_score: 10,
            doubled: false,
            payout: 250,
            created_at: ts(),
            finished_at: Some(ts()),
        };
        let p = blackjack_game_to_proto(g);
        assert_eq!(p.bet, 100);
        assert_eq!(p.player_hand.len(), 2);
        assert_eq!(p.dealer_hand.len(), 1);
        assert_eq!(p.player_score, 21);
        assert_eq!(p.payout, 250);
        assert_eq!(p.status, "player_blackjack");
        assert_eq!(p.created_at, ts().to_rfc3339());
        assert_eq!(p.finished_at, Some(ts().to_rfc3339()));
    }

    #[test]
    fn blackjack_game_to_proto_unfinished() {
        let g = BlackjackGame {
            id: Uuid::nil(), guild_id: "g".into(), user_id: "u".into(),
            username: "x".into(), bet: 50,
            player_hand: vec![], dealer_hand: vec![], deck: vec![],
            status: "in_progress".into(), player_score: 0, dealer_score: 0,
            doubled: false, payout: 0,
            created_at: ts(), finished_at: None,
        };
        let p = blackjack_game_to_proto(g);
        assert!(p.finished_at.is_none());
    }

    #[test]
    fn wallet_to_proto_full_mapping() {
        let w = Wallet {
            id: Uuid::nil(),
            guild_id: "g".into(),
            user_id: "u".into(),
            username: "rich".into(),
            coins: 5000,
            total_earned: 10000,
            total_spent: 5000,
            created_at: ts(),
            updated_at: ts(),
        };
        let p = wallet_to_proto(w);
        assert_eq!(p.coins, 5000);
        assert_eq!(p.total_earned, 10000);
        assert_eq!(p.total_spent, 5000);
        assert_eq!(p.username, "rich");
    }
