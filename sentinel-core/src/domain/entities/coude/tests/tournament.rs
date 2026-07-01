use super::*;
use chrono::TimeZone;

// ── week_bounds_for ──

#[test]
fn week_bounds_for_a_monday_returns_same_day_midnight_and_sunday_eod() {
    // 2024-01-01 etait un lundi
    let monday = Utc.with_ymd_and_hms(2024, 1, 1, 15, 30, 0).unwrap();
    let (start, end) = week_bounds_for(monday);
    assert_eq!(start.to_rfc3339(), "2024-01-01T00:00:00+00:00");
    assert_eq!(end.to_rfc3339(), "2024-01-07T23:59:59+00:00");
}

#[test]
fn week_bounds_for_a_sunday_returns_previous_monday_and_this_sunday() {
    // 2024-01-07 etait un dimanche
    let sunday = Utc.with_ymd_and_hms(2024, 1, 7, 12, 0, 0).unwrap();
    let (start, end) = week_bounds_for(sunday);
    assert_eq!(start.to_rfc3339(), "2024-01-01T00:00:00+00:00");
    assert_eq!(end.to_rfc3339(), "2024-01-07T23:59:59+00:00");
}

#[test]
fn week_bounds_for_a_wednesday_returns_monday_start() {
    // 2024-01-03 etait un mercredi
    let wed = Utc.with_ymd_and_hms(2024, 1, 3, 8, 0, 0).unwrap();
    let (start, end) = week_bounds_for(wed);
    assert_eq!(start.weekday().num_days_from_monday(), 0);
    assert_eq!(end.weekday().num_days_from_monday(), 6);
}

#[test]
fn week_bounds_spans_exactly_seven_days_minus_one_second() {
    let mon = Utc.with_ymd_and_hms(2024, 6, 10, 0, 0, 0).unwrap();
    let (start, end) = week_bounds_for(mon);
    let diff = end - start;
    assert_eq!(diff.num_days(), 6);
    assert_eq!(diff.num_seconds() % 86400, 86400 - 1);
}

// ── estimate_tournament_prize_pool ──

use crate::domain::entities::coude::economy_config::CoudeEconomyConfig;

fn ecfg() -> CoudeEconomyConfig {
    CoudeEconomyConfig::default()
}

#[test]
fn prize_pool_none_cashbox_gives_zero() {
    assert_eq!(estimate_tournament_prize_pool(None, &ecfg()), 0);
}

#[test]
fn prize_pool_is_ten_percent_of_cashbox() {
    assert_eq!(estimate_tournament_prize_pool(Some(1000), &ecfg()), 100);
    assert_eq!(estimate_tournament_prize_pool(Some(10_000), &ecfg()), 1000);
    assert_eq!(estimate_tournament_prize_pool(Some(123), &ecfg()), 12); // division entiere
}

#[test]
fn prize_pool_zero_cashbox_gives_zero() {
    assert_eq!(estimate_tournament_prize_pool(Some(0), &ecfg()), 0);
}

#[test]
fn prize_pool_small_cashbox_rounds_down() {
    assert_eq!(estimate_tournament_prize_pool(Some(9), &ecfg()), 0);
    assert_eq!(estimate_tournament_prize_pool(Some(1), &ecfg()), 0);
}

#[test]
fn prize_pool_custom_pct() {
    let cfg = CoudeEconomyConfig {
        tournament_prize_pool_pct: 25,
        ..CoudeEconomyConfig::default()
    };
    assert_eq!(estimate_tournament_prize_pool(Some(1000), &cfg), 250);
}

#[test]
fn tournament_commission_is_ten_percent() {
    assert_eq!(TOURNAMENT_PRIZE_POOL_PERCENT, 10);
}
