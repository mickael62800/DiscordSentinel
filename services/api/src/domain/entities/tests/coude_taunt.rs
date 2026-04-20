use super::*;

fn cfg_with_channel() -> CoudeTauntsConfig {
    CoudeTauntsConfig {
        guild_id: "g1".into(),
        channel_id: Some("123".into()),
        enabled: true,
    }
}

#[test]
fn thresholds_contain_3_5_10() {
    assert_eq!(TAUNT_THRESHOLDS, &[3, 5, 10]);
}

#[test]
fn crossed_threshold_detects_exact_matches_only() {
    assert_eq!(crossed_threshold(3), Some(3));
    assert_eq!(crossed_threshold(5), Some(5));
    assert_eq!(crossed_threshold(10), Some(10));
    assert_eq!(crossed_threshold(1), None);
    assert_eq!(crossed_threshold(2), None);
    assert_eq!(crossed_threshold(4), None);
    assert_eq!(crossed_threshold(6), None);
    assert_eq!(crossed_threshold(11), None);
}

#[test]
fn build_none_when_user_opted_out() {
    let ev = build_taunt_event_deterministic(&cfg_with_channel(), "u1", StreakKind::Win, 3, true);
    assert!(ev.is_none());
}

#[test]
fn build_none_when_feature_disabled() {
    let mut cfg = cfg_with_channel();
    cfg.enabled = false;
    let ev = build_taunt_event_deterministic(&cfg, "u1", StreakKind::Win, 3, false);
    assert!(ev.is_none());
}

#[test]
fn build_none_when_no_channel_configured() {
    let mut cfg = cfg_with_channel();
    cfg.channel_id = None;
    let ev = build_taunt_event_deterministic(&cfg, "u1", StreakKind::Win, 3, false);
    assert!(ev.is_none());
}

#[test]
fn build_none_when_below_threshold() {
    let ev = build_taunt_event_deterministic(&cfg_with_channel(), "u1", StreakKind::Win, 2, false);
    assert!(ev.is_none());
}

#[test]
fn build_success_substitutes_user_mention() {
    let ev = build_taunt_event_deterministic(&cfg_with_channel(), "u1", StreakKind::Win, 3, false)
        .expect("should build event");
    assert!(ev.message.contains("<@u1>"));
    assert!(!ev.message.contains("{user}"));
    assert_eq!(ev.channel_id, "123");
    assert_eq!(ev.target_user_id, "u1");
    assert_eq!(ev.streak_kind, "win");
    assert_eq!(ev.streak_value, 3);
}

#[test]
fn all_combat_kind_threshold_combinations_have_messages_and_suffix() {
    for kind in [StreakKind::Win, StreakKind::Loss, StreakKind::StealVictim] {
        for &t in TAUNT_THRESHOLDS {
            let msgs = messages_for(kind, t);
            assert!(!msgs.is_empty(), "missing messages for {:?}/{}", kind, t);
            let suffix = nickname_suffix_for(kind, t);
            assert!(!suffix.is_empty(), "missing suffix for {:?}/{}", kind, t);
            assert!(suffix.len() <= 24, "suffix too long: {:?}/{}", kind, t);
        }
    }
}

#[test]
fn all_bj_threshold_kinds_have_messages_and_suffix() {
    for kind in [StreakKind::BjBustStreak, StreakKind::BjWinStreak] {
        for &t in TAUNT_THRESHOLDS {
            let msgs = messages_for(kind, t);
            assert!(!msgs.is_empty(), "missing bj messages {:?}/{}", kind, t);
            let suffix = nickname_suffix_for(kind, t);
            assert!(!suffix.is_empty(), "missing bj suffix {:?}/{}", kind, t);
        }
    }
}

#[test]
fn one_shot_kinds_have_catalog_and_suffix() {
    for kind in [
        StreakKind::BjNatural21,
        StreakKind::EcoBankruptcy,
        StreakKind::EcoJackpot,
        StreakKind::EcoGenerousDonor,
    ] {
        assert!(!kind.is_threshold_based());
        let msgs = messages_for(kind, 0);
        assert!(!msgs.is_empty(), "missing one-shot messages {:?}", kind);
        let suffix = nickname_suffix_for(kind, 0);
        assert!(!suffix.is_empty(), "missing one-shot suffix {:?}", kind);
    }
}

#[test]
fn random_selection_picks_from_catalog() {
    let ev = build_taunt_event(&cfg_with_channel(), "u42", StreakKind::Loss, 5, false);
    assert!(ev.is_some());
}

#[test]
fn build_single_one_shot_success() {
    let ev = build_taunt_event_single(&cfg_with_channel(), "u1", StreakKind::BjNatural21, false)
        .expect("one-shot should build");
    assert!(ev.message.contains("<@u1>"));
    assert_eq!(ev.streak_kind, "bj_natural21");
}

#[test]
fn build_single_rejects_threshold_kind() {
    let ev = build_taunt_event_single(&cfg_with_channel(), "u1", StreakKind::Win, false);
    assert!(ev.is_none());
}

// ── Tests couvrant les branches non-triviales ──

#[test]
fn build_taunt_event_opt_out_returns_none() {
    // Teste la vraie fn `build_taunt_event` (pas la deterministe) avec opt_out.
    let ev = build_taunt_event(&cfg_with_channel(), "u1", StreakKind::Win, 3, true);
    assert!(ev.is_none());
}

#[test]
fn build_taunt_event_disabled_returns_none() {
    let mut cfg = cfg_with_channel();
    cfg.enabled = false;
    let ev = build_taunt_event(&cfg, "u1", StreakKind::Win, 3, false);
    assert!(ev.is_none());
}

#[test]
fn build_taunt_event_no_channel_returns_none() {
    let mut cfg = cfg_with_channel();
    cfg.channel_id = None;
    let ev = build_taunt_event(&cfg, "u1", StreakKind::Win, 3, false);
    assert!(ev.is_none());
}

#[test]
fn build_taunt_event_deterministic_one_shot_kinds() {
    // Couvre la branche `else { 0 }` pour les kinds non threshold-based.
    let ev = build_taunt_event_deterministic(&cfg_with_channel(), "u1", StreakKind::BjNatural21, 0, false);
    assert!(ev.is_some());
}

#[test]
fn streak_kind_as_str_all_variants() {
    // Couvre les branches non touchees de StreakKind::as_str.
    assert_eq!(StreakKind::Win.as_str(), "win");
    assert_eq!(StreakKind::Loss.as_str(), "loss");
    assert_eq!(StreakKind::StealVictim.as_str(), "steal_victim");
    assert_eq!(StreakKind::BjNatural21.as_str(), "bj_natural21");
    assert_eq!(StreakKind::BjBustStreak.as_str(), "bj_bust_streak");
    assert_eq!(StreakKind::BjWinStreak.as_str(), "bj_win_streak");
    assert_eq!(StreakKind::EcoBankruptcy.as_str(), "eco_bankruptcy");
    assert_eq!(StreakKind::EcoJackpot.as_str(), "eco_jackpot");
    assert_eq!(StreakKind::EcoGenerousDonor.as_str(), "eco_generous_donor");
}

#[test]
fn is_threshold_based_all_variants() {
    assert!(StreakKind::Win.is_threshold_based());
    assert!(StreakKind::Loss.is_threshold_based());
    assert!(StreakKind::StealVictim.is_threshold_based());
    assert!(StreakKind::BjBustStreak.is_threshold_based());
    assert!(StreakKind::BjWinStreak.is_threshold_based());
    assert!(!StreakKind::BjNatural21.is_threshold_based());
    assert!(!StreakKind::EcoBankruptcy.is_threshold_based());
    assert!(!StreakKind::EcoJackpot.is_threshold_based());
    assert!(!StreakKind::EcoGenerousDonor.is_threshold_based());
}

#[test]
fn bj_bust_catalogs_have_at_least_15_variants() {
    assert!(BJ_BUST_3.len() >= 15);
    assert!(BJ_BUST_5.len() >= 15);
    assert!(BJ_BUST_10.len() >= 15);
    assert!(BJ_WIN_3.len() >= 15);
    assert!(BJ_WIN_5.len() >= 15);
    assert!(BJ_WIN_10.len() >= 15);
    assert!(BJ_NATURAL_MESSAGES.len() >= 15);
    assert!(ECO_BANKRUPTCY_MESSAGES.len() >= 15);
    assert!(ECO_JACKPOT_MESSAGES.len() >= 15);
    assert!(ECO_DONOR_MESSAGES.len() >= 15);
}
