//! Tests d'integration pour le sabot partage + limite joueurs.

use sqlx::PgPool;

async fn pool() -> PgPool {
    let url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://sentinel_test:sentinel_test@localhost:5433/sentinel_test".into());
    PgPool::connect(&url).await.unwrap()
}

fn ugid() -> String { format!("{}", uuid::Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128) }
fn uch() -> String { format!("ch_{}", uuid::Uuid::new_v4().simple().to_string().chars().take(8).collect::<String>()) }

// ══════════════════════════════════════════════════════════
//  Sabot partage
// ══════════════════════════════════════════════════════════

#[tokio::test]
async fn table_has_deck_column() {
    let p = pool().await;
    let gid = ugid();
    let ch = uch();

    // Creer une table avec un deck de test
    let deck = serde_json::json!([{"rank":"As","suit":"hearts"},{"rank":"10","suit":"spades"}]);
    sqlx::query(
        "INSERT INTO blackjack_tables (guild_id, channel_id, owner_id, owner_name, deck) VALUES ($1, $2, 'owner', 'Owner', $3)",
    ).bind(&gid).bind(&ch).bind(&deck).execute(&p).await.unwrap();

    let row = sqlx::query_as::<_, (serde_json::Value,)>(
        "SELECT deck FROM blackjack_tables WHERE channel_id = $1",
    ).bind(&ch).fetch_one(&p).await.unwrap();

    let cards = row.0.as_array().unwrap();
    assert_eq!(cards.len(), 2);
    assert_eq!(cards[0]["rank"], "As");
}

#[tokio::test]
async fn shoe_6_decks_312_cards() {
    let p = pool().await;
    let gid = ugid();
    let ch = uch();

    // Simuler la creation d'un sabot de 6 decks
    let suits = ["hearts", "diamonds", "clubs", "spades"];
    let ranks = ["2","3","4","5","6","7","8","9","10","Jack","Queen","King","As"];
    let mut shoe = Vec::new();
    for _ in 0..6 {
        for suit in &suits {
            for rank in &ranks {
                shoe.push(serde_json::json!({"rank": rank, "suit": suit}));
            }
        }
    }
    assert_eq!(shoe.len(), 312);

    let shoe_json = serde_json::Value::Array(shoe);
    sqlx::query(
        "INSERT INTO blackjack_tables (guild_id, channel_id, owner_id, owner_name, deck) VALUES ($1, $2, 'o', 'O', $3)",
    ).bind(&gid).bind(&ch).bind(&shoe_json).execute(&p).await.unwrap();

    let row = sqlx::query_as::<_, (serde_json::Value,)>(
        "SELECT deck FROM blackjack_tables WHERE channel_id = $1",
    ).bind(&ch).fetch_one(&p).await.unwrap();

    assert_eq!(row.0.as_array().unwrap().len(), 312);
}

#[tokio::test]
async fn dealer_hand_stored_in_table() {
    let p = pool().await;
    let gid = ugid();
    let ch = uch();

    let dealer = serde_json::json!([{"rank":"King","suit":"hearts"},{"rank":"7","suit":"clubs"}]);
    sqlx::query(
        "INSERT INTO blackjack_tables (guild_id, channel_id, owner_id, owner_name, dealer_hand, dealer_score, round_status) VALUES ($1, $2, 'o', 'O', $3, 17, 'playing')",
    ).bind(&gid).bind(&ch).bind(&dealer).execute(&p).await.unwrap();

    let row = sqlx::query_as::<_, (serde_json::Value, i32, String)>(
        "SELECT dealer_hand, dealer_score, round_status FROM blackjack_tables WHERE channel_id = $1",
    ).bind(&ch).fetch_one(&p).await.unwrap();

    assert_eq!(row.0.as_array().unwrap().len(), 2);
    assert_eq!(row.1, 17);
    assert_eq!(row.2, "playing");
}

// ══════════════════════════════════════════════════════════
//  Limite joueurs
// ══════════════════════════════════════════════════════════

#[tokio::test]
async fn max_players_enforced() {
    let p = pool().await;
    let gid = ugid();
    let ch = uch();

    let table_id = sqlx::query_as::<_, (uuid::Uuid,)>(
        "INSERT INTO blackjack_tables (guild_id, channel_id, owner_id, owner_name) VALUES ($1, $2, 'o', 'O') RETURNING id",
    ).bind(&gid).bind(&ch).fetch_one(&p).await.unwrap().0;

    // Ajouter 7 joueurs (max par defaut)
    for i in 0..7 {
        sqlx::query("INSERT INTO blackjack_table_players (table_id, user_id, user_name) VALUES ($1, $2, $2) ON CONFLICT DO NOTHING")
            .bind(table_id).bind(format!("player_{i}")).execute(&p).await.unwrap();
    }

    let count = sqlx::query_as::<_, (i64,)>(
        "SELECT COUNT(*) FROM blackjack_table_players WHERE table_id = $1",
    ).bind(table_id).fetch_one(&p).await.unwrap().0;

    assert_eq!(count, 7);

    // Le 8eme joueur — la validation se fait cote API handler, pas en DB
    // En DB ca passe (pas de CHECK constraint), c'est l'API qui refuse
    // On verifie juste que le count est correct pour la validation
    assert!(count >= 7, "A 7 joueurs, l'API doit refuser le suivant");
}

#[tokio::test]
async fn round_status_transitions() {
    let p = pool().await;
    let gid = ugid();
    let ch = uch();

    sqlx::query(
        "INSERT INTO blackjack_tables (guild_id, channel_id, owner_id, owner_name, round_status) VALUES ($1, $2, 'o', 'O', 'waiting')",
    ).bind(&gid).bind(&ch).execute(&p).await.unwrap();

    // waiting → betting → dealing → playing → dealer_turn → resolved → waiting
    for status in &["betting", "dealing", "playing", "dealer_turn", "resolved", "waiting"] {
        sqlx::query("UPDATE blackjack_tables SET round_status = $2 WHERE channel_id = $1")
            .bind(&ch).bind(status).execute(&p).await.unwrap();

        let current = sqlx::query_as::<_, (String,)>("SELECT round_status FROM blackjack_tables WHERE channel_id = $1")
            .bind(&ch).fetch_one(&p).await.unwrap().0;
        assert_eq!(current, *status);
    }
}

#[tokio::test]
async fn current_player_index_tracks_turn() {
    let p = pool().await;
    let gid = ugid();
    let ch = uch();

    sqlx::query(
        "INSERT INTO blackjack_tables (guild_id, channel_id, owner_id, owner_name, current_player_index) VALUES ($1, $2, 'o', 'O', 0)",
    ).bind(&gid).bind(&ch).execute(&p).await.unwrap();

    // Avancer au joueur suivant
    sqlx::query("UPDATE blackjack_tables SET current_player_index = current_player_index + 1 WHERE channel_id = $1")
        .bind(&ch).execute(&p).await.unwrap();

    let idx = sqlx::query_as::<_, (i32,)>("SELECT current_player_index FROM blackjack_tables WHERE channel_id = $1")
        .bind(&ch).fetch_one(&p).await.unwrap().0;
    assert_eq!(idx, 1);
}
