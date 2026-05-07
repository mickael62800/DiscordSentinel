//! Tests d'integration pour ManageConductService avec vrais repos PG.
//! Couvre get_config persiste, deduct_points + logs, add_points, run_regen.
//! mute_user est exercee via le short-circuit token vide.

use std::sync::Arc;

use sqlx::PgPool;
use uuid::Uuid;

use sentinel_api::adapters::inbound::ws::broadcaster::EventBroadcaster;
use sentinel_api::adapters::outbound::postgres::community::conduct_repository::PgConductRepository;
use sentinel_api::adapters::outbound::postgres::moderation::infraction_repository::PgInfractionRepository;
use sentinel_api::adapters::outbound::discord_api::DiscordApiService;
use sentinel_api::application::community::manage_conduct_service::ManageConductService;
use sentinel_api::ports::inbound::community::manage_conduct::AddPointsCommand;
use sentinel_api::ports::inbound::community::manage_conduct::DeductPointsCommand;
use sentinel_api::ports::inbound::community::manage_conduct::ManageConductUseCase;
use sentinel_api::ports::inbound::community::manage_conduct::SaveConductConfigCommand;
async fn pool() -> PgPool {
    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://sentinel_test:sentinel_test@localhost:5433/sentinel_test".into()
    });
    PgPool::connect(&url).await.unwrap()
}

fn fresh_id() -> String {
    format!("{}", Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128)
}

async fn build() -> ManageConductService {
    let p = pool().await;
    let repo = Arc::new(PgConductRepository::new(p.clone()));
    let inf = Arc::new(PgInfractionRepository::new(p));
    // DiscordApi avec token vide → apply_timeout renverra une erreur configuree,
    // qui exerce la branche d'erreur de mute_user.
    let discord_api = Arc::new(DiscordApiService::new(String::new()));
    ManageConductService::new(repo, inf, Arc::new(EventBroadcaster::new()), discord_api)
}

// ── get_config / save_config ──

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_config_returns_defaults_for_new_guild() {
    let svc = build().await;
    let g = fresh_id();
    let cfg = svc.get_config(&g).await.unwrap();
    assert_eq!(cfg.guild_id, g);
    assert_eq!(cfg.max_points, 12);
    assert_eq!(cfg.regen_interval, "weekly");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn save_config_persists_and_get_returns_it() {
    let svc = build().await;
    let g = fresh_id();
    svc.save_config(SaveConductConfigCommand {
        guild_id: g.clone(), max_points: 30,
        regen_amount: 3, regen_interval: "monthly".into(),
        penalty_warn: 2, penalty_delete: 4, penalty_mute: 6, penalty_ban: 15,
    }).await.unwrap();
    let cfg = svc.get_config(&g).await.unwrap();
    assert_eq!(cfg.max_points, 30);
    assert_eq!(cfg.regen_interval, "monthly");
}

// ── deduct_points : toutes les actions + passage a 0 ──

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn deduct_points_warn_reduces_by_1() {
    let svc = build().await;
    let g = fresh_id();
    let u = fresh_id();
    let out = svc.deduct_points(DeductPointsCommand {
        guild_id: g.clone(), user_id: u.clone(), username: "Alice".into(),
        action: "warn".into(),
    }).await.unwrap();
    assert_eq!(out.points, 11); // 12 - 1
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn deduct_points_delete_reduces_by_2() {
    let svc = build().await;
    let out = svc.deduct_points(DeductPointsCommand {
        guild_id: fresh_id(), user_id: fresh_id(), username: "Alice".into(),
        action: "delete".into(),
    }).await.unwrap();
    assert_eq!(out.points, 10);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn deduct_points_mute_reduces_by_3() {
    let svc = build().await;
    let out = svc.deduct_points(DeductPointsCommand {
        guild_id: fresh_id(), user_id: fresh_id(), username: "Alice".into(),
        action: "mute".into(),
    }).await.unwrap();
    assert_eq!(out.points, 9);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn deduct_points_ban_reduces_by_6_then_zero_triggers_mute_path() {
    let svc = build().await;
    let g = fresh_id();
    let u = fresh_id();
    // 1er ban : 12 - 6 = 6
    let out = svc.deduct_points(DeductPointsCommand {
        guild_id: g.clone(), user_id: u.clone(), username: "Alice".into(),
        action: "ban".into(),
    }).await.unwrap();
    assert_eq!(out.points, 6);

    // 2eme ban : 6 - 6 = 0 → mute_user short-circuit (token vide) + infraction auto-ban saved
    let out = svc.deduct_points(DeductPointsCommand {
        guild_id: g.clone(), user_id: u.clone(), username: "Alice".into(),
        action: "ban".into(),
    }).await.unwrap();
    assert_eq!(out.points, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn deduct_points_unknown_action_is_noop() {
    let svc = build().await;
    let out = svc.deduct_points(DeductPointsCommand {
        guild_id: fresh_id(), user_id: fresh_id(), username: "A".into(),
        action: "wibble".into(),
    }).await.unwrap();
    assert_eq!(out.points, 12); // pas de penalty → just ensure row exists
}

// ── add_points : clamp max, bas → incrémente ──

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn add_points_clamps_at_max() {
    let svc = build().await;
    let g = fresh_id();
    let u = fresh_id();
    let out = svc.add_points(AddPointsCommand {
        guild_id: g, user_id: u,
        amount: 100, reason: "amnistie".into(),
    }).await.unwrap();
    assert_eq!(out.points, 12);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn add_points_from_depressed_state_increments() {
    let svc = build().await;
    let g = fresh_id();
    let u = fresh_id();
    // Deduct pour baisser le solde
    svc.deduct_points(DeductPointsCommand {
        guild_id: g.clone(), user_id: u.clone(), username: "A".into(),
        action: "ban".into(),
    }).await.unwrap();
    // Add 2
    let out = svc.add_points(AddPointsCommand {
        guild_id: g, user_id: u, amount: 2, reason: "good".into(),
    }).await.unwrap();
    assert_eq!(out.points, 8); // 6 + 2
}

// ── get_leaderboard / get_points_log ──

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_leaderboard_returns_users_with_points() {
    let svc = build().await;
    let g = fresh_id();
    // Cree deux users
    svc.deduct_points(DeductPointsCommand {
        guild_id: g.clone(), user_id: fresh_id(), username: "A".into(),
        action: "warn".into(),
    }).await.unwrap();
    svc.deduct_points(DeductPointsCommand {
        guild_id: g.clone(), user_id: fresh_id(), username: "B".into(),
        action: "mute".into(),
    }).await.unwrap();
    let lb = svc.get_leaderboard(&g, 10).await.unwrap();
    assert!(lb.len() >= 2);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_points_log_returns_entries_after_deduct() {
    let svc = build().await;
    let g = fresh_id();
    let u = fresh_id();
    svc.deduct_points(DeductPointsCommand {
        guild_id: g.clone(), user_id: u.clone(), username: "A".into(),
        action: "warn".into(),
    }).await.unwrap();
    svc.add_points(AddPointsCommand {
        guild_id: g.clone(), user_id: u.clone(),
        amount: 1, reason: "bonus".into(),
    }).await.unwrap();
    let logs = svc.get_points_log(&g, &u, 10).await.unwrap();
    assert_eq!(logs.len(), 2);
}

// ── run_regen ──

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn run_regen_returns_counter() {
    let svc = build().await;
    // Juste s'assurer que l'appel fonctionne (pas de panic sur DB réelle).
    let _total = svc.run_regen().await.unwrap();
}
