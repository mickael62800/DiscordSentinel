use super::*;

fn default_cfg() -> ConductConfig {
    ConductConfig::default_for_guild("g1")
}

#[test]
fn default_for_guild_sets_sensible_values() {
    let c = default_cfg();
    assert_eq!(c.guild_id, "g1");
    assert_eq!(c.max_points, 12);
    assert_eq!(c.regen_amount, 1);
    assert_eq!(c.regen_interval, "weekly");
    assert_eq!(c.penalty_warn, 1);
    assert_eq!(c.penalty_delete, 2);
    assert_eq!(c.penalty_mute, 3);
    assert_eq!(c.penalty_ban, 6);
}

#[test]
fn penalty_for_action_warn() {
    assert_eq!(default_cfg().penalty_for_action("warn"), 1);
}

#[test]
fn penalty_for_action_delete() {
    assert_eq!(default_cfg().penalty_for_action("delete"), 2);
}

#[test]
fn penalty_for_action_mute_variants() {
    let c = default_cfg();
    assert_eq!(c.penalty_for_action("mute"), 3);
    assert_eq!(c.penalty_for_action("mute_temp"), 3);
}

#[test]
fn penalty_for_action_ban_variants() {
    let c = default_cfg();
    assert_eq!(c.penalty_for_action("ban"), 6);
    assert_eq!(c.penalty_for_action("ban_permanent"), 6);
    assert_eq!(c.penalty_for_action("ban_temp"), 6);
}

#[test]
fn penalty_for_action_unknown_returns_zero() {
    let c = default_cfg();
    assert_eq!(c.penalty_for_action(""), 0);
    assert_eq!(c.penalty_for_action("kick"), 0);
    assert_eq!(c.penalty_for_action("WARN"), 0); // case-sensitive
}

#[test]
fn penalty_uses_configured_values_not_hardcoded() {
    let mut c = default_cfg();
    c.penalty_warn = 42;
    c.penalty_ban = 99;
    assert_eq!(c.penalty_for_action("warn"), 42);
    assert_eq!(c.penalty_for_action("ban"), 99);
}

#[test]
fn penalty_gradient_escalates() {
    // Invariant metier : warn < delete < mute < ban
    let c = default_cfg();
    assert!(c.penalty_warn < c.penalty_delete);
    assert!(c.penalty_delete < c.penalty_mute);
    assert!(c.penalty_mute < c.penalty_ban);
}
