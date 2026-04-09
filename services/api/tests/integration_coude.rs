//! Tests d'integration REELS pour le systeme Coude (avec PostgreSQL).
//! Verifie les fixes de duplication de coins : transfer, steal, combat.

use sqlx::PgPool;

async fn setup_pool() -> PgPool {
    let url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://sentinel_test:sentinel_test@localhost:5433/sentinel_test".into());
    PgPool::connect(&url).await.expect("Impossible de se connecter a la base de test")
}

fn unique_guild() -> String {
    format!("test_{}", uuid::Uuid::new_v4().simple())
}

/// Cree un joueur coude avec un solde initial.
async fn create_player(pool: &PgPool, guild_id: &str, user_id: &str, username: &str, coins: i64) {
    sqlx::query(
        r#"INSERT INTO coude_players (id, guild_id, user_id, username, coins, total_wins, total_losses, total_draws,
           total_earned, total_lost, total_stolen, cowardice_count, chaos_events, casino_wins, casino_losses,
           level, xp, atk, def, stat_points, created_at, updated_at)
           VALUES (gen_random_uuid(), $1, $2, $3, $4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, NOW(), NOW())"#,
    )
    .bind(guild_id).bind(user_id).bind(username).bind(coins)
    .execute(pool).await.unwrap();
}

/// Lit les coins d'un joueur.
async fn get_coins(pool: &PgPool, guild_id: &str, user_id: &str) -> i64 {
    sqlx::query_as::<_, (i64,)>(
        "SELECT coins FROM coude_players WHERE guild_id = $1 AND user_id = $2",
    )
    .bind(guild_id).bind(user_id)
    .fetch_one(pool).await.unwrap().0
}

// ══════════════════════════════════════════════════════════
//  Transfer : coins conserves (fix duplication)
// ══════════════════════════════════════════════════════════

#[tokio::test]
async fn transfer_coins_conserved() {
    let pool = setup_pool().await;
    let gid = unique_guild();
    create_player(&pool, &gid, "alice", "Alice", 200).await;
    create_player(&pool, &gid, "bob", "Bob", 100).await;

    // Transfer 75 from alice to bob
    let mut tx = pool.begin().await.unwrap();
    sqlx::query(
        "UPDATE coude_players SET coins = coins - $3, updated_at = NOW() WHERE guild_id = $1 AND user_id = $2 AND coins >= $3",
    ).bind(&gid).bind("alice").bind(75i64).execute(&mut *tx).await.unwrap();
    sqlx::query(
        "UPDATE coude_players SET coins = coins + $3, updated_at = NOW() WHERE guild_id = $1 AND user_id = $2",
    ).bind(&gid).bind("bob").bind(75i64).execute(&mut *tx).await.unwrap();
    tx.commit().await.unwrap();

    let alice = get_coins(&pool, &gid, "alice").await;
    let bob = get_coins(&pool, &gid, "bob").await;
    assert_eq!(alice, 125);
    assert_eq!(bob, 175);
    assert_eq!(alice + bob, 300, "Total coins doit etre conserve");
}

#[tokio::test]
async fn transfer_insufficient_balance_does_nothing() {
    let pool = setup_pool().await;
    let gid = unique_guild();
    create_player(&pool, &gid, "alice", "Alice", 10).await;
    create_player(&pool, &gid, "bob", "Bob", 100).await;

    // Tentative de transfer 500 avec seulement 10
    let result = sqlx::query(
        "UPDATE coude_players SET coins = coins - $3, updated_at = NOW() WHERE guild_id = $1 AND user_id = $2 AND coins >= $3",
    ).bind(&gid).bind("alice").bind(500i64).execute(&pool).await.unwrap();

    // rows_affected = 0 car coins < 500
    assert_eq!(result.rows_affected(), 0);

    // Rien n'a bouge
    assert_eq!(get_coins(&pool, &gid, "alice").await, 10);
    assert_eq!(get_coins(&pool, &gid, "bob").await, 100);
}

// ══════════════════════════════════════════════════════════
//  Steal : ne vole que ce que la victime a
// ══════════════════════════════════════════════════════════

#[tokio::test]
async fn steal_caps_to_victim_balance() {
    let pool = setup_pool().await;
    let gid = unique_guild();
    create_player(&pool, &gid, "thief", "Thief", 100).await;
    create_player(&pool, &gid, "victim", "Victim", 30).await;

    // Tenter de voler 200, victime n'a que 30
    let victim_coins = get_coins(&pool, &gid, "victim").await;
    let actual_stolen = 200i64.min(victim_coins); // = 30

    let mut tx = pool.begin().await.unwrap();
    sqlx::query("UPDATE coude_players SET coins = coins - $3 WHERE guild_id = $1 AND user_id = $2")
        .bind(&gid).bind("victim").bind(actual_stolen).execute(&mut *tx).await.unwrap();
    sqlx::query("UPDATE coude_players SET coins = coins + $3 WHERE guild_id = $1 AND user_id = $2")
        .bind(&gid).bind("thief").bind(actual_stolen).execute(&mut *tx).await.unwrap();
    tx.commit().await.unwrap();

    let thief = get_coins(&pool, &gid, "thief").await;
    let victim = get_coins(&pool, &gid, "victim").await;
    assert_eq!(victim, 0);
    assert_eq!(thief, 130);
    assert_eq!(thief + victim, 130, "Total = 100+30 = 130, pas de creation de coins");
}

// ══════════════════════════════════════════════════════════
//  Record loss : ne perd que ce qu'on a
// ══════════════════════════════════════════════════════════

#[tokio::test]
async fn record_loss_caps_to_balance() {
    let pool = setup_pool().await;
    let gid = unique_guild();
    create_player(&pool, &gid, "loser", "Loser", 50).await;

    // Perdre 200 avec seulement 50
    sqlx::query(
        r#"UPDATE coude_players SET total_losses = total_losses + 1,
           coins = coins - LEAST(coins, $3),
           total_lost = total_lost + LEAST(coins, $3)
           WHERE guild_id = $1 AND user_id = $2"#,
    ).bind(&gid).bind("loser").bind(200i64).execute(&pool).await.unwrap();

    let coins = get_coins(&pool, &gid, "loser").await;
    assert_eq!(coins, 0, "Coins ne doit pas etre negatif");
}

// ══════════════════════════════════════════════════════════
//  Casino log — tracking des gains quotidiens
// ══════════════════════════════════════════════════════════

#[tokio::test]
async fn casino_log_tracks_gains_today() {
    let pool = setup_pool().await;
    let gid = unique_guild();

    // Inserer quelques logs casino
    sqlx::query("INSERT INTO coude_casino_log (guild_id, user_id, amount) VALUES ($1, $2, $3)")
        .bind(&gid).bind("player1").bind(100i64).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO coude_casino_log (guild_id, user_id, amount) VALUES ($1, $2, $3)")
        .bind(&gid).bind("player1").bind(-50i64).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO coude_casino_log (guild_id, user_id, amount) VALUES ($1, $2, $3)")
        .bind(&gid).bind("player1").bind(200i64).execute(&pool).await.unwrap();

    // Somme des gains positifs dans les 24h
    let total: i64 = sqlx::query_as::<_, (i64,)>(
        r#"SELECT COALESCE(SUM(amount), 0)::bigint FROM coude_casino_log
           WHERE guild_id = $1 AND user_id = $2
           AND amount > 0 AND created_at > NOW() - INTERVAL '24 hours'"#,
    ).bind(&gid).bind("player1").fetch_one(&pool).await.unwrap().0;

    assert_eq!(total, 300); // 100 + 200 (les -50 sont ignores)
}

// ══════════════════════════════════════════════════════════
//  Blackjack unique active game index
// ══════════════════════════════════════════════════════════

#[tokio::test]
async fn blackjack_unique_active_game_enforced_by_db() {
    let pool = setup_pool().await;
    let gid = unique_guild();

    // Premier jeu actif
    sqlx::query(
        r#"INSERT INTO blackjack_games (id, guild_id, user_id, username, bet, player_hand, dealer_hand, deck, status, player_score, dealer_score, doubled, payout, created_at)
           VALUES (gen_random_uuid(), $1, $2, 'Alice', 100, '[]', '[]', '[]', 'playing', 0, 0, false, 0, NOW())"#,
    ).bind(&gid).bind("player1").execute(&pool).await.unwrap();

    // Deuxieme jeu actif pour le meme joueur — doit echouer (unique index)
    let result = sqlx::query(
        r#"INSERT INTO blackjack_games (id, guild_id, user_id, username, bet, player_hand, dealer_hand, deck, status, player_score, dealer_score, doubled, payout, created_at)
           VALUES (gen_random_uuid(), $1, $2, 'Alice', 100, '[]', '[]', '[]', 'playing', 0, 0, false, 0, NOW())"#,
    ).bind(&gid).bind("player1").execute(&pool).await;

    assert!(result.is_err(), "La DB doit refuser un deuxieme jeu actif pour le meme joueur");
}
