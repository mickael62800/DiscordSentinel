//! Implementation gRPC complete du domaine Coup de Coude.
//!
//! Phase 7A : `CoudePlayerService` — 6 methodes hot path joueurs.
//! Phase 7A.opt F.1 : 5 services supplementaires wrappant les 5 use cases
//! restants (combats, bets, economy, inventory, social). coude-bot est
//! maintenant 100% gRPC pour ses appels metier.
//!
//! Refactor 2026-04 : le god-object 1880 LOC a ete splitte en un module
//! directory avec 1 fichier par service + helpers partages ici (parse_uuid,
//! taunt_event_to_proto). Chaque sous-module contient sa propre impl
//! du trait tonic + ses helpers prives (en `pub(super)` quand les tests
//! de mod.rs en ont besoin).

use std::str::FromStr;

use tonic::Status;
use uuid::Uuid;

use sentinel_proto::coude::v1 as proto;

pub(super) fn parse_uuid(s: &str) -> Result<Uuid, Status> {
    Uuid::from_str(s).map_err(|_| Status::invalid_argument(format!("UUID invalide: {s}")))
}

/// Helper partage : convertit un `TauntEvent` domain en message proto.
/// Utilise par `CoudeCombatsService.ResolveCombatNow` (qui retourne
/// les TauntEvents emis pendant la resolution) et par
/// `CoudeSocialService.TrackStealVictim` (qui retourne un TauntEvent
/// optionnel si la streak vol de la victime franchit un palier).
pub(super) fn taunt_event_to_proto(
    e: crate::domain::entities::TauntEvent,
) -> proto::TauntEvent {
    proto::TauntEvent {
        channel_id: e.channel_id,
        target_user_id: e.target_user_id,
        message: e.message,
        nickname_suffix: e.nickname_suffix,
        streak_kind: e.streak_kind.to_string(),
        streak_value: e.streak_value,
    }
}

mod bets;
mod combats;
mod economy;
mod inventory;
mod players;
mod social;

pub use bets::CoudeBetsGrpc;
pub use combats::CoudeCombatsGrpc;
pub use economy::CoudeEconomyGrpc;
pub use inventory::CoudeInventoryGrpc;
pub use players::CoudePlayerGrpc;
pub use social::CoudeSocialGrpc;


// ══════════════════════════════════════════════════════════════════════
// Tests unitaires des converters proto <-> domain (Phase 7A.opt F.1)
// ══════════════════════════════════════════════════════════════════════
//
// Ces tests verifient que la traduction entre les entites de domaine et
// les messages protobuf est complete et correcte (aucun champ oublie ou
// melange). Ce sont des fonctions pures, donc pas de DB ni de mock.

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    use crate::domain::entities::{
        BetPayout, BetResolutionPlan, CoudeBet, CoudeCombat, CoudeCurrentSeason, CoudeEvent,
        CoudeFighterBetBonus, CoudeInsurance, CoudeInventoryItem, CoudeLeaderboardEntry,
        CoudePlayer, CoudePrime, LeaderboardCategory, RefundSummary, XpProgress,
    };
    use crate::domain::value_objects::CoudeClass;

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
        let b = CoudeBet {
            id: 42,
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
        assert_eq!(pr.id, 42);
        assert_eq!(pr.amount, 100);
        assert_eq!(pr.won, Some(true));
        assert_eq!(pr.payout, Some(200));
    }

    #[test]
    fn bet_payout_to_proto_mapping() {
        let p = BetPayout {
            bet_id: 1, bettor_id: "u".into(), bettor_name: "n".into(),
            backed_id: "a".into(), amount_bet: 100, payout: 250, won: true,
        };
        let pr = bets::bet_payout_to_proto(p);
        assert_eq!(pr.bet_id, 1);
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
}

