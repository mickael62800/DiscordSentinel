//! Tests d'integration pour le systeme de jeux (table `games`).
//!
//! Note : la table `game_subscriptions` a ete supprimee (migration 137),
//! donc plus de tests sur les souscriptions ici.

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

async fn create_game(p: &PgPool, gid: &str, name: &str) -> uuid::Uuid {
    sqlx::query_as::<_, (uuid::Uuid,)>(
        "INSERT INTO games (guild_id, game_name, created_by) VALUES ($1, $2, '333') RETURNING id",
    )
    .bind(gid)
    .bind(name)
    .fetch_one(p)
    .await
    .unwrap()
    .0
}

// ══════════════════════════════════════════════════════════
//  Games CRUD
// ══════════════════════════════════════════════════════════

#[tokio::test]
async fn game_create_and_list() {
    let p = pool().await;
    let gid = ugid();
    create_game(&p, &gid, "Fortnite").await;
    create_game(&p, &gid, "Valorant").await;

    let games = sqlx::query_as::<_, (String,)>(
        "SELECT game_name FROM games WHERE guild_id = $1 ORDER BY game_name",
    )
    .bind(&gid)
    .fetch_all(&p)
    .await
    .unwrap();
    assert_eq!(games.len(), 2);
    assert_eq!(games[0].0, "Fortnite");
    assert_eq!(games[1].0, "Valorant");
}

#[tokio::test]
async fn game_unique_name_per_guild() {
    let p = pool().await;
    let gid = ugid();
    create_game(&p, &gid, "Fortnite").await;
    let dup = sqlx::query(
        "INSERT INTO games (guild_id, game_name, created_by) VALUES ($1, 'fortnite', '333')",
    )
    .bind(&gid)
    .execute(&p)
    .await;
    assert!(
        dup.is_err(),
        "Duplicate game name (case-insensitive) doit etre rejete"
    );
}

#[tokio::test]
async fn game_same_name_different_guild() {
    let p = pool().await;
    let gid1 = ugid();
    let gid2 = ugid();
    create_game(&p, &gid1, "Fortnite").await;
    let result = sqlx::query(
        "INSERT INTO games (guild_id, game_name, created_by) VALUES ($1, 'Fortnite', '333')",
    )
    .bind(&gid2)
    .execute(&p)
    .await;
    assert!(
        result.is_ok(),
        "Meme nom dans une autre guild doit etre autorise"
    );
}

#[tokio::test]
async fn find_game_by_name_case_insensitive() {
    let p = pool().await;
    let gid = ugid();
    create_game(&p, &gid, "Rocket League").await;

    let game = sqlx::query_as::<_, (String,)>(
        "SELECT game_name FROM games WHERE guild_id = $1 AND LOWER(game_name) = LOWER($2)",
    )
    .bind(&gid)
    .bind("rocket league")
    .fetch_optional(&p)
    .await
    .unwrap();

    assert!(game.is_some());
    assert_eq!(game.unwrap().0, "Rocket League");
}

// ══════════════════════════════════════════════════════════
//  Bot definition
// ══════════════════════════════════════════════════════════

#[tokio::test]
async fn game_bot_definition_exists() {
    let p = pool().await;
    let def = sqlx::query_as::<_, (String, String)>(
        "SELECT display_name, description FROM bot_definitions WHERE bot_name = 'game-bot'",
    )
    .fetch_optional(&p)
    .await
    .unwrap();

    assert!(
        def.is_some(),
        "game-bot doit etre seed dans bot_definitions"
    );
    let (name, desc) = def.unwrap();
    assert_eq!(name, "Games");
    assert!(desc.to_lowercase().contains("jeu"));
}
