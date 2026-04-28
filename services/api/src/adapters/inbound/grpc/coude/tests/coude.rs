use super::*;

    use chrono::TimeZone;
    use chrono::Utc;
    use uuid::Uuid;

    use crate::domain::entities::coude::bet::BetPayout;
    use crate::domain::entities::coude::bet::BetResolutionPlan;
    use crate::domain::entities::coude::bet::CoudeBet;
    use crate::domain::entities::coude::combat::CoudeCombat;
    use crate::domain::entities::coude::social::CoudeCurrentSeason;
    use crate::domain::entities::coude::social::CoudeEvent;
    use crate::domain::entities::coude::bet::FighterBetBonus as CoudeFighterBetBonus;
    use crate::domain::entities::coude::inventory::CoudeInsurance;
    use crate::domain::entities::coude::inventory::CoudeInventoryItem;
    use crate::domain::entities::coude::social::CoudeLeaderboardEntry;
    use crate::domain::entities::coude::player::CoudePlayer;
    use crate::domain::entities::coude::inventory::CoudePrime;
    use crate::domain::entities::coude::social::LeaderboardCategory;
    use crate::domain::entities::coude::bet::RefundSummary;
    use crate::domain::entities::coude::player::XpProgress;
    use crate::domain::enums::coude::coude_class::CoudeClass;

    fn ts() -> chrono::DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 1, 15, 12, 30, 0).unwrap()
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
        assert!(err.message().contains("UUID invalide"));
    }

    #[test]
    fn coude_player_to_proto_full_mapping() {
        let p = CoudePlayer {
            guild_id: "g1".into(),
            user_id: "u1".into(),
            username: "alice".into(),
            coins: 1234,
            total_wins: 5,
            total_losses: 3,
            total_draws: 1,
            total_earned: 2000,
            total_lost: 700,
            total_stolen: 50,
            cowardice_count: 2,
            chaos_events: 4,
            casino_wins: 7,
            casino_losses: 9,
            level: 12,
            xp: 4500,
            stat_points: 3,
            atk: 8,
            def: 6,
            class: Some(CoudeClass::Tank),
            title: Some("Champion".into()),
            class_changed_at: None,
            hp_current: 80,
            hp_max: 100,
            hp_last_regen: None,
            repos_last_used: None,
            season: 2,
            created_at: ts(),
            updated_at: ts(),
        };
        let pr = players::coude_player_to_proto(p.clone());
        assert_eq!(pr.guild_id, "g1");
        assert_eq!(pr.user_id, "u1");
        assert_eq!(pr.username, "alice");
        assert_eq!(pr.coins, 1234);
        assert_eq!(pr.total_wins, 5);
        assert_eq!(pr.total_losses, 3);
        assert_eq!(pr.total_draws, 1);
        assert_eq!(pr.level, 12);
        assert_eq!(pr.xp, 4500);
        assert_eq!(pr.atk, 8);
        assert_eq!(pr.def, 6);
        assert_eq!(pr.hp_current, 80);
        assert_eq!(pr.hp_max, 100);
        assert_eq!(pr.season, 2);
        assert_eq!(pr.class.as_deref(), Some(CoudeClass::Tank.as_str()));
        assert_eq!(pr.title.as_deref(), Some("Champion"));
        assert_eq!(pr.created_at, ts().to_rfc3339());
    }

    #[test]
    fn coude_player_to_proto_optional_class_none() {
        let p = CoudePlayer {
            guild_id: "g".into(), user_id: "u".into(), username: "x".into(),
            coins: 0, total_wins: 0, total_losses: 0, total_draws: 0,
            total_earned: 0, total_lost: 0, total_stolen: 0,
            cowardice_count: 0, chaos_events: 0, casino_wins: 0, casino_losses: 0,
            level: 1, xp: 0, stat_points: 0, atk: 0, def: 0,
            class: None, title: None, class_changed_at: None,
            hp_current: 100, hp_max: 100, hp_last_regen: None, repos_last_used: None,
            season: 1, created_at: ts(), updated_at: ts(),
        };
        let pr = players::coude_player_to_proto(p);
        assert!(pr.class.is_none());
        assert!(pr.title.is_none());
    }

    #[test]
    fn xp_progress_to_proto_mapping() {
        let x = XpProgress { new_xp: 1500, new_level: 8, leveled_up: true, stat_points_gained: 2 };
        let p = players::xp_progress_to_proto(x);
        assert_eq!(p.new_xp, 1500);
        assert_eq!(p.new_level, 8);
        assert!(p.leveled_up);
        assert_eq!(p.stat_points_gained, 2);
    }

    #[test]
    fn combat_to_proto_full_mapping() {
        let id = Uuid::new_v4();
        let c = CoudeCombat {
            id,
            guild_id: "g1".into(),
            channel_id: Some("c1".into()),
            attacker_id: "a".into(),
            attacker_name: "Atk".into(),
            defender_id: "d".into(),
            defender_name: "Def".into(),
            mise: 500,
            status: "resolved".into(),
            winner_id: Some("a".into()),
            attacker_roll: Some(15),
            defender_roll: Some(10),
            chaos_event: Some("eclipse".into()),
            special_attack: Some("uppercut".into()),
            defender_special: None,
            coins_transferred: Some(500),
            result_message: Some("Victoire".into()),
            message_id: Some("m1".into()),
            created_at: ts(),
            accepted_at: Some(ts()),
            resolved_at: Some(ts()),
        };
        let pr = combats::combat_to_proto(c);
        assert_eq!(pr.id, id.to_string());
        assert_eq!(pr.channel_id.as_deref(), Some("c1"));
        assert_eq!(pr.mise, 500);
        assert_eq!(pr.status, "resolved");
        assert_eq!(pr.winner_id.as_deref(), Some("a"));
        assert_eq!(pr.attacker_roll, Some(15));
        assert_eq!(pr.coins_transferred, Some(500));
        assert_eq!(pr.accepted_at, Some(ts().to_rfc3339()));
    }

    #[test]
    fn bet_to_proto_mapping() {
        let bid = Uuid::from_u128(42);
        let b = CoudeBet {
            id: bid,
            guild_id: "g".into(),
            combat_id: Uuid::nil(),
            bettor_id: "u".into(),
            bettor_name: "Joe".into(),
            backed_id: "a".into(),
            amount: 100,
            won: Some(true),
            payout: Some(200),
        };
        let pr = bets::bet_to_proto(b);
        assert_eq!(pr.id, bid.to_string());
        assert_eq!(pr.amount, 100);
        assert_eq!(pr.won, Some(true));
        assert_eq!(pr.payout, Some(200));
    }

    #[test]
    fn bet_payout_to_proto_mapping() {
        let pid = Uuid::from_u128(1);
        let p = BetPayout {
            bet_id: pid, bettor_id: "u".into(), bettor_name: "n".into(),
            backed_id: "a".into(), amount_bet: 100, payout: 250, won: true,
        };
        let pr = bets::bet_payout_to_proto(p);
        assert_eq!(pr.bet_id, pid.to_string());
        assert_eq!(pr.amount_bet, 100);
        assert_eq!(pr.payout, 250);
        assert!(pr.won);
    }

    #[test]
    fn bet_resolution_plan_to_proto_with_bonus() {
        let plan = BetResolutionPlan {
            payouts: vec![],
            fighter_bonus: Some(CoudeFighterBetBonus {
                winner_id: "w".into(), winner_bonus: 1000,
                loser_id: "l".into(), loser_bonus: 500,
                total_pot: 2000,
            }),
        };
        let pr = bets::bet_resolution_plan_to_proto(plan);
        assert!(pr.fighter_bonus.is_some());
        let b = pr.fighter_bonus.unwrap();
        assert_eq!(b.winner_bonus, 1000);
        assert_eq!(b.total_pot, 2000);
    }

    #[test]
    fn bet_resolution_plan_to_proto_with_payouts_and_no_bonus() {
        let plan = BetResolutionPlan {
            payouts: vec![
                BetPayout {
                    bet_id: Uuid::from_u128(1),
                    bettor_id: "u1".into(), bettor_name: "Alice".into(),
                    backed_id: "a".into(), amount_bet: 100, payout: 250, won: true,
                },
                BetPayout {
                    bet_id: Uuid::from_u128(2),
                    bettor_id: "u2".into(), bettor_name: "Bob".into(),
                    backed_id: "b".into(), amount_bet: 50, payout: 0, won: false,
                },
            ],
            fighter_bonus: None,
        };
        let pr = bets::bet_resolution_plan_to_proto(plan);
        assert_eq!(pr.payouts.len(), 2);
        assert!(pr.payouts[0].won);
        assert!(!pr.payouts[1].won);
        assert!(pr.fighter_bonus.is_none());
    }

    #[test]
    fn fighter_bonus_to_proto_mapping() {
        let b = CoudeFighterBetBonus {
            winner_id: "winner".into(), winner_bonus: 1500,
            loser_id: "loser".into(), loser_bonus: 250,
            total_pot: 3000,
        };
        let pr = bets::fighter_bonus_to_proto(b);
        assert_eq!(pr.winner_id, "winner");
        assert_eq!(pr.winner_bonus, 1500);
        assert_eq!(pr.loser_id, "loser");
        assert_eq!(pr.loser_bonus, 250);
        assert_eq!(pr.total_pot, 3000);
    }

    #[test]
    fn refund_summary_to_proto_mapping() {
        let s = RefundSummary { refunded_count: 3, refunded_total: 750 };
        let pr = bets::refund_summary_to_proto(s);
        assert_eq!(pr.refunded_count, 3);
        assert_eq!(pr.refunded_total, 750);
    }

    #[test]
    fn inventory_item_to_proto_mapping() {
        let i = CoudeInventoryItem {
            guild_id: "g".into(), user_id: "u".into(),
            item_key: "potion".into(), quantity: 5,
        };
        let pr = inventory::inventory_item_to_proto(i);
        assert_eq!(pr.item_key, "potion");
        assert_eq!(pr.quantity, 5);
    }

    #[test]
    fn prime_to_proto_unclaimed() {
        let p = CoudePrime {
            id: Uuid::new_v4(),
            guild_id: "g".into(),
            target_id: "t".into(), target_name: "T".into(),
            placed_by_id: "p".into(), placed_by_name: "P".into(),
            amount: 1000, claimed: false,
            claimed_by_id: None, claimed_by_name: None, claimed_at: None,
            created_at: ts(),
        };
        let pr = inventory::prime_to_proto(p);
        assert_eq!(pr.amount, 1000);
        assert!(!pr.claimed);
        assert!(pr.claimed_by_id.is_none());
    }

    #[test]
    fn insurance_to_proto_mapping() {
        let id = Uuid::new_v4();
        let i = CoudeInsurance { id, is_scam: true, expires_at: ts() };
        let pr = inventory::insurance_to_proto(i);
        assert_eq!(pr.id, id.to_string());
        assert!(pr.is_scam);
        assert_eq!(pr.expires_at, ts().to_rfc3339());
    }

    #[test]
    fn proto_to_leaderboard_category_all_variants() {
        assert_eq!(
            social::proto_to_leaderboard_category(proto::LeaderboardCategory::Richest as i32),
            LeaderboardCategory::Richest
        );
        assert_eq!(
            social::proto_to_leaderboard_category(proto::LeaderboardCategory::Thieves as i32),
            LeaderboardCategory::Thieves
        );
        assert_eq!(
            social::proto_to_leaderboard_category(proto::LeaderboardCategory::Cowards as i32),
            LeaderboardCategory::Cowards
        );
        assert_eq!(
            social::proto_to_leaderboard_category(proto::LeaderboardCategory::Chaos as i32),
            LeaderboardCategory::Chaos
        );
        assert_eq!(
            social::proto_to_leaderboard_category(proto::LeaderboardCategory::Level as i32),
            LeaderboardCategory::Level
        );
        // Unspecified et valeur invalide => Richest (defaut)
        assert_eq!(
            social::proto_to_leaderboard_category(proto::LeaderboardCategory::Unspecified as i32),
            LeaderboardCategory::Richest
        );
        assert_eq!(
            social::proto_to_leaderboard_category(9999),
            LeaderboardCategory::Richest
        );
    }

    #[test]
    fn current_season_to_proto_mapping() {
        let s = CoudeCurrentSeason {
            season_number: 3, started_at: ts(), ends_at: ts(), days_remaining: 42,
        };
        let pr = social::current_season_to_proto(s);
        assert_eq!(pr.season_number, 3);
        assert_eq!(pr.days_remaining, 42);
    }

    #[test]
    fn leaderboard_entry_to_proto_mapping() {
        let e = CoudeLeaderboardEntry {
            user_id: "u".into(), username: "Top".into(), value: 9999,
        };
        let pr = social::leaderboard_entry_to_proto(e);
        assert_eq!(pr.user_id, "u");
        assert_eq!(pr.value, 9999);
    }

    #[test]
    fn taunt_event_to_proto_mapping() {
        use crate::domain::entities::coude::taunt::TauntEvent;
        let t = TauntEvent {
            channel_id: "chan-42".into(),
            target_user_id: "u1".into(),
            message: "hehe".into(),
            nickname_suffix: "le gros naze".into(),
            streak_kind: "win",
            streak_value: 3,
        };
        let p = super::taunt_event_to_proto(t);
        assert_eq!(p.channel_id, "chan-42");
        assert_eq!(p.target_user_id, "u1");
        assert_eq!(p.message, "hehe");
        assert_eq!(p.nickname_suffix, "le gros naze");
        assert_eq!(p.streak_kind, "win");
        assert_eq!(p.streak_value, 3);
    }

    #[test]
    fn taunt_event_to_proto_empty_suffix() {
        use crate::domain::entities::coude::taunt::TauntEvent;
        let t = TauntEvent {
            channel_id: "c".into(),
            target_user_id: "u".into(),
            message: "".into(),
            nickname_suffix: "".into(),
            streak_kind: "loss",
            streak_value: 0,
        };
        let p = super::taunt_event_to_proto(t);
        assert!(p.nickname_suffix.is_empty());
        assert_eq!(p.streak_kind, "loss");
    }

    #[test]
    fn event_to_proto_mapping() {
        let id = Uuid::new_v4();
        let e = CoudeEvent {
            id, guild_id: "g".into(), event_type: "happy_hour".into(), active: true,
            expires_at: ts(), created_at: ts(),
        };
        let pr = social::event_to_proto(e);
        assert_eq!(pr.id, id.to_string());
        assert!(pr.active);
    }

    // ── redistribution_to_proto + proto_source_to_domain ──

    #[test]
    fn redistribution_to_proto_mapping_with_winners() {
        use crate::ports::inbound::coude::manage_cashbox::RedistributionOutcome;
        let rid = Uuid::new_v4();
        let outcome = RedistributionOutcome {
            redistribution_id: rid,
            total_amount: 5000,
            winners: vec![
                ("u1".to_string(), "Alice".to_string(), 3000),
                ("u2".to_string(), "Bob".to_string(), 2000),
            ],
        };
        let pr = social::redistribution_to_proto("g1".into(), outcome);
        assert!(pr.executed);
        assert_eq!(pr.redistribution_id.as_deref(), Some(rid.to_string().as_str()));
        assert_eq!(pr.total_amount, 5000);
        assert_eq!(pr.guild_id, "g1");
        assert_eq!(pr.winners.len(), 2);
        assert_eq!(pr.winners[0].user_id, "u1");
        assert_eq!(pr.winners[0].username, "Alice");
        assert_eq!(pr.winners[0].amount_won, 3000);
        assert_eq!(pr.winners[1].amount_won, 2000);
    }

    #[test]
    fn redistribution_to_proto_empty_winners() {
        use crate::ports::inbound::coude::manage_cashbox::RedistributionOutcome;
        let outcome = RedistributionOutcome {
            redistribution_id: Uuid::nil(),
            total_amount: 0,
            winners: vec![],
        };
        let pr = social::redistribution_to_proto("g".into(), outcome);
        assert!(pr.executed);
        assert_eq!(pr.total_amount, 0);
        assert!(pr.winners.is_empty());
    }

    #[test]
    fn proto_source_to_domain_all_variants() {
        use crate::domain::entities::coude::cashbox::CashboxSource;
        assert_eq!(
            social::proto_source_to_domain(proto::CashboxDepositSource::CashboxSourceShopPurchase as i32),
            Some(CashboxSource::ShopPurchase)
        );
        assert_eq!(
            social::proto_source_to_domain(proto::CashboxDepositSource::CashboxSourceInsurancePurchase as i32),
            Some(CashboxSource::InsurancePurchase)
        );
        assert_eq!(
            social::proto_source_to_domain(proto::CashboxDepositSource::CashboxSourceProtectionPurchase as i32),
            Some(CashboxSource::ProtectionPurchase)
        );
        assert_eq!(
            social::proto_source_to_domain(proto::CashboxDepositSource::CashboxSourceBoostPurchase as i32),
            Some(CashboxSource::BoostPurchase)
        );
        assert_eq!(
            social::proto_source_to_domain(proto::CashboxDepositSource::CashboxSourceClassChangeCost as i32),
            Some(CashboxSource::ClassChangeCost)
        );
        assert_eq!(
            social::proto_source_to_domain(proto::CashboxDepositSource::CashboxSourceResetStatsCost as i32),
            Some(CashboxSource::ResetStatsCost)
        );
        assert_eq!(
            social::proto_source_to_domain(proto::CashboxDepositSource::CashboxSourceDonationTax as i32),
            Some(CashboxSource::DonationTax)
        );
        assert_eq!(
            social::proto_source_to_domain(proto::CashboxDepositSource::CashboxSourceCowardicePenalty as i32),
            Some(CashboxSource::CowardicePenalty)
        );
        assert_eq!(
            social::proto_source_to_domain(proto::CashboxDepositSource::CashboxSourceBetCommission as i32),
            Some(CashboxSource::BetCommission)
        );
    }

    // ── inventory: steal_boost / steal_protection / proto_steal_duration ──

    #[test]
    fn steal_boost_to_proto_mapping() {
        use crate::domain::entities::coude::steal_boost::CoudeStealBoost;
        let id = Uuid::new_v4();
        let b = CoudeStealBoost {
            id,
            guild_id: "g".into(),
            user_id: "u".into(),
            item_key: "boost_7d".into(),
            expires_at: ts(),
            created_at: ts(),
        };
        let pr = inventory::steal_boost_to_proto(b);
        assert_eq!(pr.id, id.to_string());
        assert_eq!(pr.item_key, "boost_7d");
        assert_eq!(pr.expires_at, ts().to_rfc3339());
        assert_eq!(pr.created_at, ts().to_rfc3339());
    }

    #[test]
    fn steal_protection_to_proto_mapping() {
        use crate::domain::entities::coude::steal_protection::CoudeStealProtection;
        let id = Uuid::new_v4();
        let p = CoudeStealProtection {
            id,
            guild_id: "g".into(),
            user_id: "u".into(),
            item_key: "shield_3d".into(),
            expires_at: ts(),
            created_at: ts(),
        };
        let pr = inventory::steal_protection_to_proto(p);
        assert_eq!(pr.id, id.to_string());
        assert_eq!(pr.user_id, "u");
        assert_eq!(pr.item_key, "shield_3d");
    }

    #[test]
    fn proto_steal_duration_to_domain_all_variants() {
        use crate::domain::entities::coude::steal_protection::StealProtectionDuration as D;
        assert_eq!(
            inventory::proto_steal_duration_to_domain(proto::StealProtectionDurationKind::StealProtectionDurationOneDay as i32),
            Some(D::OneDay)
        );
        assert_eq!(
            inventory::proto_steal_duration_to_domain(proto::StealProtectionDurationKind::StealProtectionDurationThreeDays as i32),
            Some(D::ThreeDays)
        );
        assert_eq!(
            inventory::proto_steal_duration_to_domain(proto::StealProtectionDurationKind::StealProtectionDurationFiveDays as i32),
            Some(D::FiveDays)
        );
        assert_eq!(
            inventory::proto_steal_duration_to_domain(proto::StealProtectionDurationKind::StealProtectionDurationSevenDays as i32),
            Some(D::SevenDays)
        );
        assert_eq!(
            inventory::proto_steal_duration_to_domain(proto::StealProtectionDurationKind::StealProtectionDurationUnspecified as i32),
            None
        );
        assert_eq!(inventory::proto_steal_duration_to_domain(99999), None);
    }

    #[test]
    fn proto_source_to_domain_unspecified_and_invalid() {
        assert_eq!(
            social::proto_source_to_domain(proto::CashboxDepositSource::CashboxSourceUnspecified as i32),
            None
        );
        // Valeur hors enum → try_from echoue → None
        assert_eq!(social::proto_source_to_domain(99999), None);
    }
