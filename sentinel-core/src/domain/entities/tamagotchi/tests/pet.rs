use super::*;
use chrono::{Duration, TimeZone, Utc};

fn base_pet(now: chrono::DateTime<Utc>) -> Pet {
    Pet {
        id: uuid::Uuid::nil(),
        guild_id: "g".into(),
        owner_id: "u".into(),
        name: "Gaviscon".into(),
        species: "sanglier".into(),
        specialization: None,
        level: 1,
        xp: 0,
        born_at: now,
        hunger: 100,
        happiness: 100,
        energy: 100,
        status: Health::Healthy,
        hunger_zero_since: None,
        sick_since: None,
        died_at: None,
        str_: 16,
        vit: 10,
        agi: 4,
        stat_points: 0,
        elo: 1000,
        wins: 0,
        losses: 0,
        cooldowns: serde_json::json!({}),
        last_decay_at: now,
    }
}

fn cfg() -> TickConfig {
    TickConfig {
        hunger_decay_per_hour: 10,
        happiness_decay_per_hour: 5,
        energy_decay_per_hour: 6,
        sick_after_secs: 12 * 3600,
        death_after_sick_secs: 24 * 3600,
        low_threshold: 20,
    }
}

#[test]
fn decay_reduces_gauges_over_time() {
    let t0 = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
    let mut p = base_pet(t0);
    let out = p.apply_tick(t0 + Duration::hours(2), &cfg());
    assert_eq!(out, TickOutcome::Decayed);
    assert_eq!(p.hunger, 80); // 100 - 10*2
    assert_eq!(p.happiness, 90);
    assert_eq!(p.energy, 88);
}

#[test]
fn gauges_clamp_at_zero() {
    let t0 = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
    let mut p = base_pet(t0);
    p.apply_tick(t0 + Duration::hours(100), &cfg());
    assert_eq!(p.hunger, 0);
    assert_eq!(p.happiness, 0);
    assert_eq!(p.energy, 0);
    assert!(p.hunger_zero_since.is_some());
}

#[test]
fn hunger_zero_long_enough_makes_sick() {
    let t0 = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
    let mut p = base_pet(t0);
    // Faim a 0 immediatement.
    p.hunger = 0;
    p.hunger_zero_since = Some(t0);
    let out = p.apply_tick(t0 + Duration::hours(13), &cfg());
    assert_eq!(out, TickOutcome::FellSick);
    assert_eq!(p.status, Health::Sick);
    assert!(p.sick_since.is_some());
}

#[test]
fn sick_too_long_dies() {
    let t0 = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
    let mut p = base_pet(t0);
    p.status = Health::Sick;
    p.sick_since = Some(t0);
    p.hunger = 0;
    p.hunger_zero_since = Some(t0);
    let out = p.apply_tick(t0 + Duration::hours(25), &cfg());
    assert_eq!(out, TickOutcome::Died);
    assert_eq!(p.status, Health::Dead);
    assert!(p.died_at.is_some());
}

#[test]
fn caring_recovers_sick_pet() {
    let t0 = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
    let mut p = base_pet(t0);
    p.status = Health::Sick;
    p.sick_since = Some(t0);
    // Jauges hautes (le joueur a nourri/joue avant le tick).
    p.hunger = 80;
    p.happiness = 80;
    p.energy = 80;
    let out = p.apply_tick(t0 + Duration::hours(1), &cfg());
    assert_eq!(out, TickOutcome::Recovered);
    assert_eq!(p.status, Health::Healthy);
    assert!(p.sick_since.is_none());
}

#[test]
fn dead_pet_unchanged() {
    let t0 = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
    let mut p = base_pet(t0);
    p.status = Health::Dead;
    let out = p.apply_tick(t0 + Duration::hours(10), &cfg());
    assert_eq!(out, TickOutcome::Unchanged);
}

#[test]
fn level_curve() {
    assert_eq!(level_from_xp(0), 1);
    assert_eq!(level_from_xp(99), 1);
    assert_eq!(level_from_xp(100), 2); // 100 pour passer niv1->2
    assert_eq!(level_from_xp(300), 3); // +200 pour niv2->3
    let (in_lvl, needed) = xp_progress(150);
    assert_eq!((in_lvl, needed), (50, 200));
}
