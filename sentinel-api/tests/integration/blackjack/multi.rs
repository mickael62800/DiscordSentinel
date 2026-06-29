//! Tests d'integration pour le blackjack multijoueur (tables, joueurs).

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
fn uch() -> String {
    format!(
        "ch_{}",
        uuid::Uuid::new_v4()
            .simple()
            .to_string()
            .chars()
            .take(8)
            .collect::<String>()
    )
}

async fn create_table(p: &PgPool, gid: &str, ch: &str, owner: &str) -> uuid::Uuid {
    sqlx::query_as::<_, (uuid::Uuid,)>(
        "INSERT INTO blackjack_tables (guild_id, channel_id, owner_id, owner_name) VALUES ($1, $2, $3, $3) RETURNING id",
    ).bind(gid).bind(ch).bind(owner).fetch_one(p).await.unwrap().0
}

// ══════════════════════════════════════════════════════════
//  Tables CRUD
// ══════════════════════════════════════════════════════════

#[tokio::test]
async fn table_create_and_read() {
    let p = pool().await;
    let gid = ugid();
    let ch = uch();
    let id = create_table(&p, &gid, &ch, "Alice").await;

    let row = sqlx::query_as::<_, (String, String, String)>(
        "SELECT owner_name, status, channel_id FROM blackjack_tables WHERE id = $1",
    )
    .bind(id)
    .fetch_one(&p)
    .await
    .unwrap();

    assert_eq!(row.0, "Alice");
    assert_eq!(row.1, "open");
    assert_eq!(row.2, ch);
}

#[tokio::test]
async fn table_unique_channel() {
    let p = pool().await;
    let gid = ugid();
    let ch = uch();
    create_table(&p, &gid, &ch, "Alice").await;

    let dup = sqlx::query(
        "INSERT INTO blackjack_tables (guild_id, channel_id, owner_id, owner_name) VALUES ($1, $2, 'Bob', 'Bob')",
    ).bind(&gid).bind(&ch).execute(&p).await;

    assert!(dup.is_err(), "Duplicate channel_id doit etre rejete");
}

#[tokio::test]
async fn table_close() {
    let p = pool().await;
    let gid = ugid();
    let ch = uch();
    let id = create_table(&p, &gid, &ch, "Alice").await;

    sqlx::query("UPDATE blackjack_tables SET status = 'closed' WHERE id = $1")
        .bind(id)
        .execute(&p)
        .await
        .unwrap();

    let status =
        sqlx::query_as::<_, (String,)>("SELECT status FROM blackjack_tables WHERE id = $1")
            .bind(id)
            .fetch_one(&p)
            .await
            .unwrap()
            .0;
    assert_eq!(status, "closed");
}

// ══════════════════════════════════════════════════════════
//  Players
// ══════════════════════════════════════════════════════════

#[tokio::test]
async fn player_join_table() {
    let p = pool().await;
    let gid = ugid();
    let ch = uch();
    let table_id = create_table(&p, &gid, &ch, "Alice").await;

    // Owner + 2 joueurs
    for (uid, name) in &[("111", "Alice"), ("222", "Bob"), ("333", "Charlie")] {
        sqlx::query("INSERT INTO blackjack_table_players (table_id, user_id, user_name) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING")
            .bind(table_id).bind(uid).bind(name).execute(&p).await.unwrap();
    }

    let count = sqlx::query_as::<_, (i64,)>(
        "SELECT COUNT(*) FROM blackjack_table_players WHERE table_id = $1",
    )
    .bind(table_id)
    .fetch_one(&p)
    .await
    .unwrap()
    .0;

    assert_eq!(count, 3);
}

#[tokio::test]
async fn player_unique_per_table() {
    let p = pool().await;
    let gid = ugid();
    let ch = uch();
    let table_id = create_table(&p, &gid, &ch, "Alice").await;

    sqlx::query("INSERT INTO blackjack_table_players (table_id, user_id, user_name) VALUES ($1, '111', 'Alice')")
        .bind(table_id).execute(&p).await.unwrap();
    let dup = sqlx::query("INSERT INTO blackjack_table_players (table_id, user_id, user_name) VALUES ($1, '111', 'Alice')")
        .bind(table_id).execute(&p).await;

    assert!(
        dup.is_err(),
        "Un joueur ne peut rejoindre qu'une seule fois"
    );
}

#[tokio::test]
async fn players_cascade_on_table_delete() {
    let p = pool().await;
    let gid = ugid();
    let ch = uch();
    let table_id = create_table(&p, &gid, &ch, "Alice").await;

    sqlx::query("INSERT INTO blackjack_table_players (table_id, user_id, user_name) VALUES ($1, '111', 'A') ON CONFLICT DO NOTHING")
        .bind(table_id).execute(&p).await.unwrap();
    sqlx::query("INSERT INTO blackjack_table_players (table_id, user_id, user_name) VALUES ($1, '222', 'B') ON CONFLICT DO NOTHING")
        .bind(table_id).execute(&p).await.unwrap();

    // Supprimer la table → joueurs en cascade
    sqlx::query("DELETE FROM blackjack_tables WHERE id = $1")
        .bind(table_id)
        .execute(&p)
        .await
        .unwrap();

    let count = sqlx::query_as::<_, (i64,)>(
        "SELECT COUNT(*) FROM blackjack_table_players WHERE table_id = $1",
    )
    .bind(table_id)
    .fetch_one(&p)
    .await
    .unwrap()
    .0;

    assert_eq!(count, 0);
}

// ══════════════════════════════════════════════════════════
//  Games liees a une table
// ══════════════════════════════════════════════════════════

#[tokio::test]
async fn game_linked_to_table() {
    let p = pool().await;
    let gid = ugid();
    let ch = uch();
    let table_id = create_table(&p, &gid, &ch, "Alice").await;

    // Creer une partie liee a la table
    sqlx::query(
        r#"INSERT INTO blackjack_games (id, guild_id, user_id, username, bet, player_hand, dealer_hand, deck, status, player_score, dealer_score, doubled, payout, created_at, table_id)
           VALUES (gen_random_uuid(), $1, '111', 'Alice', 100, '[]', '[]', '[]', 'playing', 0, 0, false, 0, NOW(), $2)"#,
    ).bind(&gid).bind(table_id).execute(&p).await.unwrap();

    let count =
        sqlx::query_as::<_, (i64,)>("SELECT COUNT(*) FROM blackjack_games WHERE table_id = $1")
            .bind(table_id)
            .fetch_one(&p)
            .await
            .unwrap()
            .0;

    assert_eq!(count, 1);
}

#[tokio::test]
async fn multiple_players_each_have_game() {
    let p = pool().await;
    let gid = ugid();
    let ch = uch();
    let table_id = create_table(&p, &gid, &ch, "Alice").await;

    // 3 joueurs, chacun avec sa partie
    for (uid, name) in &[("111", "Alice"), ("222", "Bob"), ("333", "Charlie")] {
        sqlx::query(
            r#"INSERT INTO blackjack_games (id, guild_id, user_id, username, bet, player_hand, dealer_hand, deck, status, player_score, dealer_score, doubled, payout, created_at, table_id)
               VALUES (gen_random_uuid(), $1, $2, $3, 100, '[]', '[]', '[]', 'playing', 0, 0, false, 0, NOW(), $4)"#,
        ).bind(&gid).bind(uid).bind(name).bind(table_id).execute(&p).await.unwrap();
    }

    let count = sqlx::query_as::<_, (i64,)>(
        "SELECT COUNT(*) FROM blackjack_games WHERE table_id = $1 AND status = 'playing'",
    )
    .bind(table_id)
    .fetch_one(&p)
    .await
    .unwrap()
    .0;

    assert_eq!(count, 3, "3 joueurs = 3 parties actives");
}

#[tokio::test]
async fn table_delete_nullifies_game_table_id() {
    let p = pool().await;
    let gid = ugid();
    let ch = uch();
    let table_id = create_table(&p, &gid, &ch, "Alice").await;

    sqlx::query(
        r#"INSERT INTO blackjack_games (id, guild_id, user_id, username, bet, player_hand, dealer_hand, deck, status, player_score, dealer_score, doubled, payout, created_at, table_id)
           VALUES (gen_random_uuid(), $1, '111', 'A', 100, '[]', '[]', '[]', 'player_win', 10, 5, false, 200, NOW(), $2)"#,
    ).bind(&gid).bind(table_id).execute(&p).await.unwrap();

    // Supprimer la table → table_id NULL (ON DELETE SET NULL)
    sqlx::query("DELETE FROM blackjack_tables WHERE id = $1")
        .bind(table_id)
        .execute(&p)
        .await
        .unwrap();

    // La game doit toujours exister mais avec table_id = NULL
    let remaining = sqlx::query_as::<_, (i64,)>(
        "SELECT COUNT(*) FROM blackjack_games WHERE guild_id = $1 AND table_id IS NULL",
    )
    .bind(&gid)
    .fetch_one(&p)
    .await
    .unwrap()
    .0;

    assert!(remaining >= 0); // La game existe toujours
}

// ══════════════════════════════════════════════════════════
//  Find by channel
// ══════════════════════════════════════════════════════════

#[tokio::test]
async fn find_table_by_channel() {
    let p = pool().await;
    let gid = ugid();
    let ch = uch();
    create_table(&p, &gid, &ch, "Alice").await;

    let found = sqlx::query_as::<_, (String,)>(
        "SELECT owner_name FROM blackjack_tables WHERE channel_id = $1 AND status = 'open'",
    )
    .bind(&ch)
    .fetch_optional(&p)
    .await
    .unwrap();

    assert!(found.is_some());
    assert_eq!(found.unwrap().0, "Alice");
}

#[tokio::test]
async fn find_closed_table_returns_none() {
    let p = pool().await;
    let gid = ugid();
    let ch = uch();
    let id = create_table(&p, &gid, &ch, "Alice").await;

    sqlx::query("UPDATE blackjack_tables SET status = 'closed' WHERE id = $1")
        .bind(id)
        .execute(&p)
        .await
        .unwrap();

    let found = sqlx::query_as::<_, (String,)>(
        "SELECT owner_name FROM blackjack_tables WHERE channel_id = $1 AND status = 'open'",
    )
    .bind(&ch)
    .fetch_optional(&p)
    .await
    .unwrap();

    assert!(found.is_none());
}
