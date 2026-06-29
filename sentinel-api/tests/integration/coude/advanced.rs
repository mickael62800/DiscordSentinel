//! Tests d'integration pour les systemes coude avances :
//! bets, cooldowns, inventory, primes, insurances, events, seasons, dons.

use sqlx::PgPool;

async fn pool() -> PgPool {
    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://sentinel_test:sentinel_test@localhost:5433/sentinel_test".into()
    });
    PgPool::connect(&url).await.unwrap()
}

fn ugid() -> String {
    format!(
        "{}",
        uuid::Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128
    )
}

async fn create_player(p: &PgPool, gid: &str, uid: &str, coins: i64) {
    sqlx::query("INSERT INTO coude_players (guild_id, user_id, username, coins) VALUES ($1, $2, $2, $3) ON CONFLICT DO NOTHING")
        .bind(gid).bind(uid).bind(coins).execute(p).await.unwrap();
}

async fn create_combat(p: &PgPool, gid: &str) -> uuid::Uuid {
    sqlx::query_as::<_, (uuid::Uuid,)>(
        "INSERT INTO coude_combats (guild_id, channel_id, attacker_id, attacker_name, defender_id, defender_name) VALUES ($1, '555', '111', 'A', '222', 'B') RETURNING id",
    ).bind(gid).fetch_one(p).await.unwrap().0
}

// ── Bets ──

#[tokio::test]
async fn bet_on_combat() {
    let p = pool().await;
    let gid = ugid();
    let combat_id = create_combat(&p, &gid).await;

    sqlx::query(
        "INSERT INTO coude_bets (guild_id, combat_id, bettor_id, bettor_name, backed_id, amount) VALUES ($1, $2, '333', 'Bettor', '111', 100)",
    ).bind(&gid).bind(combat_id).execute(&p).await.unwrap();

    let count = sqlx::query_as::<_, (i64,)>("SELECT COUNT(*) FROM coude_bets WHERE combat_id = $1")
        .bind(combat_id)
        .fetch_one(&p)
        .await
        .unwrap()
        .0;
    assert_eq!(count, 1);
}

#[tokio::test]
async fn multiple_bets_on_same_combat() {
    let p = pool().await;
    let gid = ugid();
    let combat_id = create_combat(&p, &gid).await;

    for (bettor, backed) in &[("333", "111"), ("444", "222"), ("555", "111")] {
        sqlx::query(
            "INSERT INTO coude_bets (guild_id, combat_id, bettor_id, bettor_name, backed_id, amount) VALUES ($1, $2, $3, $3, $4, 50)",
        ).bind(&gid).bind(combat_id).bind(bettor).bind(backed).execute(&p).await.unwrap();
    }

    let total = sqlx::query_as::<_, (i64,)>(
        "SELECT COALESCE(SUM(amount), 0)::bigint FROM coude_bets WHERE combat_id = $1",
    )
    .bind(combat_id)
    .fetch_one(&p)
    .await
    .unwrap()
    .0;
    assert_eq!(total, 150);
}

// ── Cooldowns ──

#[tokio::test]
async fn cooldown_set_and_check() {
    let p = pool().await;
    let gid = ugid();

    sqlx::query(
        r#"INSERT INTO coude_cooldowns (guild_id, user_id, action, expires_at)
           VALUES ($1, '444', 'voler', NOW() + INTERVAL '30 minutes')
           ON CONFLICT (guild_id, user_id, action) DO UPDATE SET expires_at = EXCLUDED.expires_at"#,
    )
    .bind(&gid)
    .execute(&p)
    .await
    .unwrap();

    let active = sqlx::query_as::<_, (i64,)>(
        "SELECT COUNT(*) FROM coude_cooldowns WHERE guild_id = $1 AND user_id = '444' AND action = 'voler' AND expires_at > NOW()",
    ).bind(&gid).fetch_one(&p).await.unwrap().0;
    assert_eq!(active, 1);
}

#[tokio::test]
async fn cooldown_expired_not_returned() {
    let p = pool().await;
    let gid = ugid();

    sqlx::query(
        "INSERT INTO coude_cooldowns (guild_id, user_id, action, expires_at) VALUES ($1, '444', 'casino', NOW() - INTERVAL '1 hour') ON CONFLICT (guild_id, user_id, action) DO UPDATE SET expires_at = EXCLUDED.expires_at",
    ).bind(&gid).execute(&p).await.unwrap();

    let active = sqlx::query_as::<_, (i64,)>(
        "SELECT COUNT(*) FROM coude_cooldowns WHERE guild_id = $1 AND user_id = '444' AND action = 'casino' AND expires_at > NOW()",
    ).bind(&gid).fetch_one(&p).await.unwrap().0;
    assert_eq!(active, 0);
}

// ── Inventory ──

#[tokio::test]
async fn inventory_add_item() {
    let p = pool().await;
    let gid = ugid();
    create_player(&p, &gid, "444", 100).await;

    sqlx::query(
        "INSERT INTO coude_inventory (guild_id, user_id, item_key, quantity) VALUES ($1, '444', 'explosion', 2) ON CONFLICT (guild_id, user_id, item_key) DO UPDATE SET quantity = coude_inventory.quantity + EXCLUDED.quantity",
    ).bind(&gid).execute(&p).await.unwrap();

    // Ajouter encore
    sqlx::query(
        "INSERT INTO coude_inventory (guild_id, user_id, item_key, quantity) VALUES ($1, '444', 'explosion', 1) ON CONFLICT (guild_id, user_id, item_key) DO UPDATE SET quantity = coude_inventory.quantity + EXCLUDED.quantity",
    ).bind(&gid).execute(&p).await.unwrap();

    let qty = sqlx::query_as::<_, (i32,)>(
        "SELECT quantity FROM coude_inventory WHERE guild_id = $1 AND user_id = '444' AND item_key = 'explosion'",
    ).bind(&gid).fetch_one(&p).await.unwrap().0;
    assert_eq!(qty, 3);
}

#[tokio::test]
async fn inventory_use_item() {
    let p = pool().await;
    let gid = ugid();
    create_player(&p, &gid, "444", 100).await;

    sqlx::query("INSERT INTO coude_inventory (guild_id, user_id, item_key, quantity) VALUES ($1, '444', 'poison', 3) ON CONFLICT DO NOTHING")
        .bind(&gid).execute(&p).await.unwrap();

    sqlx::query("UPDATE coude_inventory SET quantity = quantity - 1 WHERE guild_id = $1 AND user_id = '444' AND item_key = 'poison' AND quantity > 0")
        .bind(&gid).execute(&p).await.unwrap();

    let qty = sqlx::query_as::<_, (i32,)>(
        "SELECT quantity FROM coude_inventory WHERE guild_id = $1 AND user_id = '444' AND item_key = 'poison'",
    ).bind(&gid).fetch_one(&p).await.unwrap().0;
    assert_eq!(qty, 2);
}

// ── Primes (bounties) ──

#[tokio::test]
async fn prime_create_and_claim() {
    let p = pool().await;
    let gid = ugid();

    let id = sqlx::query_as::<_, (uuid::Uuid,)>(
        "INSERT INTO coude_primes (guild_id, target_id, target_name, placed_by_id, placed_by_name, amount) VALUES ($1, '222', 'Target', '111', 'Placer', 500) RETURNING id",
    ).bind(&gid).fetch_one(&p).await.unwrap().0;

    // Claim
    sqlx::query("UPDATE coude_primes SET claimed = true, claimed_by_id = '333', claimed_by_name = 'Hunter', claimed_at = NOW() WHERE id = $1")
        .bind(id).execute(&p).await.unwrap();

    let row = sqlx::query_as::<_, (bool, Option<String>)>(
        "SELECT claimed, claimed_by_name FROM coude_primes WHERE id = $1",
    )
    .bind(id)
    .fetch_one(&p)
    .await
    .unwrap();
    assert!(row.0);
    assert_eq!(row.1.unwrap(), "Hunter");
}

// ── Insurances ──

#[tokio::test]
async fn insurance_buy_and_expire() {
    let p = pool().await;
    let gid = ugid();

    sqlx::query(
        "INSERT INTO coude_insurances (guild_id, user_id, expires_at) VALUES ($1, '444', NOW() + INTERVAL '1 hour')",
    ).bind(&gid).execute(&p).await.unwrap();

    let active = sqlx::query_as::<_, (i64,)>(
        "SELECT COUNT(*) FROM coude_insurances WHERE guild_id = $1 AND user_id = '444' AND active = true AND expires_at > NOW()",
    ).bind(&gid).fetch_one(&p).await.unwrap().0;
    assert_eq!(active, 1);
}

// ── Events (chaos) ──

#[tokio::test]
async fn chaos_event_create() {
    let p = pool().await;
    let gid = ugid();

    sqlx::query(
        "INSERT INTO coude_events (id, guild_id, event_type, active, started_at, expires_at) VALUES (gen_random_uuid(), $1, 'happy_hour', true, NOW(), NOW() + INTERVAL '1 hour')",
    ).bind(&gid).execute(&p).await.unwrap();

    let active = sqlx::query_as::<_, (String, bool)>(
        "SELECT event_type, active FROM coude_events WHERE guild_id = $1",
    )
    .bind(&gid)
    .fetch_one(&p)
    .await
    .unwrap();
    assert_eq!(active.0, "happy_hour");
    assert!(active.1);
}

// ── Seasons ──

#[tokio::test]
async fn season_create() {
    let p = pool().await;
    let gid = ugid();

    sqlx::query("INSERT INTO coude_seasons (guild_id, season_number) VALUES ($1, 1)")
        .bind(&gid)
        .execute(&p)
        .await
        .unwrap();

    let row =
        sqlx::query_as::<_, (i32,)>("SELECT season_number FROM coude_seasons WHERE guild_id = $1")
            .bind(&gid)
            .fetch_one(&p)
            .await
            .unwrap();
    assert_eq!(row.0, 1);
}
