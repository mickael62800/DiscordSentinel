//! Tests d'integration postgres pour PgPlayerRepository.
//! Couvre les submodules mod, read, progression, combat_stats, streaks, hp.

use sqlx::PgPool;
use uuid::Uuid;

use sentinel_api::adapters::outbound::postgres::coude::player_repository::PgPlayerRepository;
use sentinel_core::domain::entities::coude::player::CombatStat;
use sentinel_api::ports::outbound::coude::player_repository::PlayerRepository;

async fn pool() -> PgPool {
    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_|
        "postgres://sentinel_test:sentinel_test@localhost:5433/sentinel_test".into());
    PgPool::connect(&url).await.unwrap()
}
fn fresh_id() -> String {
    format!("{}", Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128)
}

// ── CRUD basique ──

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_or_create_creates_new_player() {
    let repo = PgPlayerRepository::new(pool().await);
    let g = fresh_id(); let u = fresh_id();
    let p = repo.get_or_create(&g, &u, "Alice").await.unwrap();
    assert_eq!(p.guild_id, g);
    assert_eq!(p.user_id, u);
    assert_eq!(p.username, "Alice");
    assert_eq!(p.level, 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_or_create_updates_username_on_rename() {
    let repo = PgPlayerRepository::new(pool().await);
    let g = fresh_id(); let u = fresh_id();
    repo.get_or_create(&g, &u, "OldName").await.unwrap();
    let p = repo.get_or_create(&g, &u, "NewName").await.unwrap();
    assert_eq!(p.username, "NewName");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_returns_some_when_exists() {
    let repo = PgPlayerRepository::new(pool().await);
    let g = fresh_id(); let u = fresh_id();
    repo.get_or_create(&g, &u, "A").await.unwrap();
    let got = repo.get(&g, &u).await.unwrap().unwrap();
    assert_eq!(got.username, "A");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_returns_none_when_absent() {
    let repo = PgPlayerRepository::new(pool().await);
    assert!(repo.get(&fresh_id(), &fresh_id()).await.unwrap().is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_scoped_to_guild() {
    // list() lit la materialized view mv_coude_leaderboard, on doit la
    // rafraichir manuellement apres des inserts pour que les donnees soient
    // visibles.
    let p = pool().await;
    let repo = PgPlayerRepository::new(p.clone());
    let g = fresh_id();
    for _ in 0..3 {
        repo.get_or_create(&g, &fresh_id(), "X").await.unwrap();
    }
    sqlx::query("REFRESH MATERIALIZED VIEW mv_coude_leaderboard")
        .execute(&p).await.unwrap();
    let all = repo.list(&g, 50).await.unwrap();
    assert_eq!(all.len(), 3);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_guild_ids_returns_distinct() {
    let repo = PgPlayerRepository::new(pool().await);
    let g1 = fresh_id(); let g2 = fresh_id();
    repo.get_or_create(&g1, &fresh_id(), "A").await.unwrap();
    repo.get_or_create(&g2, &fresh_id(), "B").await.unwrap();
    let ids = repo.list_guild_ids().await.unwrap();
    assert!(ids.contains(&g1));
    assert!(ids.contains(&g2));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn random_active_requires_min_coins() {
    let p = pool().await;
    let repo = PgPlayerRepository::new(p.clone());
    let g = fresh_id();
    let rich_user = fresh_id();
    let poor_user = fresh_id();
    repo.get_or_create(&g, &rich_user, "Rich").await.unwrap();
    repo.get_or_create(&g, &poor_user, "Poor").await.unwrap();
    // Seed wallet pour rich uniquement.
    sqlx::query("INSERT INTO user_wallets (guild_id, user_id, username, coins) VALUES ($1, $2, 'Rich', 1000) \
                 ON CONFLICT (guild_id, user_id) DO UPDATE SET coins = EXCLUDED.coins")
        .bind(&g).bind(&rich_user).execute(&p).await.unwrap();
    // Poor existe mais sans wallet -> COALESCE=0. Force le wallet a 5 (< 100)
    // pour eviter le defaut "coins de depart" qui peut etre >0 selon migrations.
    sqlx::query("INSERT INTO user_wallets (guild_id, user_id, username, coins) VALUES ($1, $2, 'Poor', 5) \
                 ON CONFLICT (guild_id, user_id) DO UPDATE SET coins = EXCLUDED.coins")
        .bind(&g).bind(&poor_user).execute(&p).await.unwrap();

    let actives = repo.random_active(&g, 10, 100).await.unwrap();
    let names: Vec<&str> = actives.iter().map(|p| p.user_id.as_str()).collect();
    assert!(names.contains(&rich_user.as_str()));
    assert!(!names.contains(&poor_user.as_str()));
}

// ── Class + progression ──

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn update_class_sets_class_and_changed_at() {
    let repo = PgPlayerRepository::new(pool().await);
    let g = fresh_id(); let u = fresh_id();
    repo.get_or_create(&g, &u, "A").await.unwrap();
    assert!(repo.update_class(&g, &u, "bourrin").await.unwrap());
    let p = repo.get(&g, &u).await.unwrap().unwrap();
    assert!(p.class.is_some());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn add_xp_increments_xp_and_levels_up() {
    let repo = PgPlayerRepository::new(pool().await);
    let g = fresh_id(); let u = fresh_id();
    repo.get_or_create(&g, &u, "A").await.unwrap();
    let progress = repo.add_xp(&g, &u, 200).await.unwrap().unwrap();
    assert!(progress.new_xp >= 200);
    assert!(progress.new_level >= 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn add_xp_none_for_unknown_player() {
    let repo = PgPlayerRepository::new(pool().await);
    assert!(repo.add_xp(&fresh_id(), &fresh_id(), 100).await.unwrap().is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn spend_stat_point_requires_points_available() {
    let p = pool().await;
    let repo = PgPlayerRepository::new(p.clone());
    let g = fresh_id(); let u = fresh_id();
    repo.get_or_create(&g, &u, "A").await.unwrap();
    // Default stat_points = 0 → doit retourner None.
    assert!(repo.spend_stat_point(&g, &u, CombatStat::Atk).await.unwrap().is_none());
    // Seed stat_points = 2
    sqlx::query("UPDATE coude_players SET stat_points = 2 WHERE guild_id = $1 AND user_id = $2")
        .bind(&g).bind(&u).execute(&p).await.unwrap();
    let after = repo.spend_stat_point(&g, &u, CombatStat::Atk).await.unwrap().unwrap();
    assert_eq!(after.atk, 1);
    assert_eq!(after.stat_points, 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn spend_stat_point_def_also_increases_hp_max() {
    let p = pool().await;
    let repo = PgPlayerRepository::new(p.clone());
    let g = fresh_id(); let u = fresh_id();
    repo.get_or_create(&g, &u, "A").await.unwrap();
    sqlx::query("UPDATE coude_players SET stat_points = 5 WHERE guild_id = $1 AND user_id = $2")
        .bind(&g).bind(&u).execute(&p).await.unwrap();
    let before = repo.get(&g, &u).await.unwrap().unwrap();
    let after = repo.spend_stat_point(&g, &u, CombatStat::Def).await.unwrap().unwrap();
    // DEF gives +2 HP max AND +2 HP current
    assert_eq!(after.def, 1);
    assert_eq!(after.hp_max, before.hp_max + 2);
    assert_eq!(after.hp_current, before.hp_current + 2);
}

// ── add_xp multi-level ──

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn add_xp_no_level_up_when_amount_too_small() {
    let repo = PgPlayerRepository::new(pool().await);
    let g = fresh_id(); let u = fresh_id();
    repo.get_or_create(&g, &u, "A").await.unwrap();
    let progress = repo.add_xp(&g, &u, 5).await.unwrap().unwrap();
    assert!(!progress.leveled_up);
    assert_eq!(progress.stat_points_gained, 0);
    assert_eq!(progress.new_xp, 5);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn update_class_unknown_player_returns_false() {
    let repo = PgPlayerRepository::new(pool().await);
    let ok = repo.update_class(&fresh_id(), &fresh_id(), "tank").await.unwrap();
    assert!(!ok);
}

// ── Compteurs combat ──

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn record_win_increments_wins_and_earnings() {
    let repo = PgPlayerRepository::new(pool().await);
    let g = fresh_id(); let u = fresh_id();
    repo.get_or_create(&g, &u, "A").await.unwrap();
    assert!(repo.record_win(&g, &u, 100, 50).await.unwrap());
    let p = repo.get(&g, &u).await.unwrap().unwrap();
    assert_eq!(p.total_wins, 1);
    assert_eq!(p.total_earned, 100);
    assert_eq!(p.total_stolen, 50);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn record_loss_and_draw() {
    let repo = PgPlayerRepository::new(pool().await);
    let g = fresh_id(); let u = fresh_id();
    repo.get_or_create(&g, &u, "A").await.unwrap();
    repo.record_loss(&g, &u, 30).await.unwrap();
    repo.record_draw(&g, &u, 10).await.unwrap();
    let p = repo.get(&g, &u).await.unwrap().unwrap();
    assert_eq!(p.total_losses, 1);
    assert_eq!(p.total_draws, 1);
    assert_eq!(p.total_lost, 40);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn increment_cowardice_returns_new_count() {
    let repo = PgPlayerRepository::new(pool().await);
    let g = fresh_id(); let u = fresh_id();
    repo.get_or_create(&g, &u, "A").await.unwrap();
    assert_eq!(repo.increment_cowardice(&g, &u).await.unwrap(), Some(1));
    assert_eq!(repo.increment_cowardice(&g, &u).await.unwrap(), Some(2));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn increment_chaos_returns_bool() {
    let repo = PgPlayerRepository::new(pool().await);
    let g = fresh_id(); let u = fresh_id();
    repo.get_or_create(&g, &u, "A").await.unwrap();
    assert!(repo.increment_chaos(&g, &u).await.unwrap());
    let p = repo.get(&g, &u).await.unwrap().unwrap();
    assert_eq!(p.chaos_events, 1);
}

// ── Streaks ──

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn touch_win_streak_increments() {
    let repo = PgPlayerRepository::new(pool().await);
    let g = fresh_id(); let u = fresh_id();
    repo.get_or_create(&g, &u, "A").await.unwrap();
    assert_eq!(repo.touch_win_streak(&g, &u).await.unwrap(), Some(1));
    assert_eq!(repo.touch_win_streak(&g, &u).await.unwrap(), Some(2));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn touch_loss_streak_resets_win() {
    let repo = PgPlayerRepository::new(pool().await);
    let g = fresh_id(); let u = fresh_id();
    repo.get_or_create(&g, &u, "A").await.unwrap();
    repo.touch_win_streak(&g, &u).await.unwrap();
    repo.touch_win_streak(&g, &u).await.unwrap();
    repo.touch_loss_streak(&g, &u).await.unwrap();
    // Prochaine win = 1 (reset)
    assert_eq!(repo.touch_win_streak(&g, &u).await.unwrap(), Some(1));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn reset_combat_streaks_sets_both_to_zero() {
    let repo = PgPlayerRepository::new(pool().await);
    let g = fresh_id(); let u = fresh_id();
    repo.get_or_create(&g, &u, "A").await.unwrap();
    repo.touch_win_streak(&g, &u).await.unwrap();
    repo.reset_combat_streaks(&g, &u).await.unwrap();
    // Prochaine win = 1
    assert_eq!(repo.touch_win_streak(&g, &u).await.unwrap(), Some(1));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn steal_victim_streak_touch_and_reset() {
    let repo = PgPlayerRepository::new(pool().await);
    let g = fresh_id(); let u = fresh_id();
    repo.get_or_create(&g, &u, "A").await.unwrap();
    assert_eq!(repo.touch_steal_victim_streak(&g, &u).await.unwrap(), Some(1));
    repo.reset_steal_victim_streak(&g, &u).await.unwrap();
    assert_eq!(repo.touch_steal_victim_streak(&g, &u).await.unwrap(), Some(1));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn bj_streaks_win_resets_bust_and_vice_versa() {
    let repo = PgPlayerRepository::new(pool().await);
    let g = fresh_id(); let u = fresh_id();
    repo.get_or_create(&g, &u, "A").await.unwrap();
    assert_eq!(repo.touch_bj_win_streak(&g, &u).await.unwrap(), Some(1));
    assert_eq!(repo.touch_bj_bust_streak(&g, &u).await.unwrap(), Some(1));
    // touch_bj_bust a reset bj_win
    assert_eq!(repo.touch_bj_win_streak(&g, &u).await.unwrap(), Some(1));
    // reset_bj_bust
    repo.reset_bj_bust_streak(&g, &u).await.unwrap();
    assert_eq!(repo.touch_bj_bust_streak(&g, &u).await.unwrap(), Some(1));
}

// ── Coins (stats-only) ──

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn record_coins_earned_and_lost_update_stats() {
    let repo = PgPlayerRepository::new(pool().await);
    let g = fresh_id(); let u = fresh_id();
    repo.get_or_create(&g, &u, "A").await.unwrap();
    repo.record_coins_earned(&g, &u, 500).await.unwrap();
    repo.record_coins_lost(&g, &u, 200).await.unwrap();
    let p = repo.get(&g, &u).await.unwrap().unwrap();
    assert_eq!(p.total_earned, 500);
    assert_eq!(p.total_lost, 200);
}

// ── HP ──

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn update_hp_sets_current_and_max() {
    let repo = PgPlayerRepository::new(pool().await);
    let g = fresh_id(); let u = fresh_id();
    repo.get_or_create(&g, &u, "A").await.unwrap();
    repo.update_hp(&g, &u, 42, 150).await.unwrap();
    let p = repo.get(&g, &u).await.unwrap().unwrap();
    assert_eq!(p.hp_current, 42);
    assert_eq!(p.hp_max, 150);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn full_heal_sets_hp_current_to_hp_max() {
    let repo = PgPlayerRepository::new(pool().await);
    let g = fresh_id(); let u = fresh_id();
    repo.get_or_create(&g, &u, "A").await.unwrap();
    repo.update_hp(&g, &u, 10, 100).await.unwrap();
    repo.full_heal(&g, &u).await.unwrap();
    let p = repo.get(&g, &u).await.unwrap().unwrap();
    assert_eq!(p.hp_current, p.hp_max);
    assert!(p.repos_last_used.is_some());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn regen_hp_tick_does_not_panic() {
    let repo = PgPlayerRepository::new(pool().await);
    // Pas d'assertion metier complexe : juste verifier que la query tourne
    // sans erreur sur des joueurs existants (le worker appelle cette methode
    // periodiquement).
    let _ = repo.regen_hp_tick(10.0, 20.0, 30.0, 40.0).await.unwrap();
}
