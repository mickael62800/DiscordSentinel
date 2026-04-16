use async_trait::async_trait;
use sqlx::PgPool;

use crate::domain::errors::DomainError;
use crate::ports::outbound::{Game, GameRepository};
use super::pg_err;

pub struct PgGameRepository { pool: PgPool }

impl PgGameRepository {
    pub fn new(pool: PgPool) -> Self { Self { pool } }
}

#[derive(sqlx::FromRow)]
struct GameRow { id: String, guild_id: String, game_name: String, created_by: String, created_at: String }

impl From<GameRow> for Game {
    fn from(r: GameRow) -> Self {
        Self { id: r.id, guild_id: r.guild_id, game_name: r.game_name, created_by: r.created_by, created_at: r.created_at }
    }
}

#[async_trait]
impl GameRepository for PgGameRepository {
    async fn list(&self, guild_id: &str) -> Result<Vec<Game>, DomainError> {
        let rows: Vec<GameRow> = sqlx::query_as(
            "SELECT id::text, guild_id, game_name, created_by, created_at::text FROM games WHERE guild_id = $1 ORDER BY game_name",
        ).bind(guild_id).fetch_all(&self.pool).await.map_err(pg_err)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn create(&self, guild_id: &str, game_name: &str, created_by: &str) -> Result<Game, DomainError> {
        let row: GameRow = sqlx::query_as(
            "INSERT INTO games (guild_id, game_name, created_by) VALUES ($1, $2, $3) \
             RETURNING id::text, guild_id, game_name, created_by, created_at::text",
        ).bind(guild_id).bind(game_name).bind(created_by)
        .fetch_one(&self.pool).await
        .map_err(|e| {
            if e.to_string().contains("idx_games_guild_name") {
                DomainError::Conflict("Un jeu avec ce nom existe deja".into())
            } else {
                pg_err(e)
            }
        })?;
        Ok(row.into())
    }

    async fn delete(&self, guild_id: &str, game_id: &str) -> Result<bool, DomainError> {
        let res = sqlx::query("DELETE FROM games WHERE guild_id = $1 AND id = $2::uuid")
            .bind(guild_id).bind(game_id)
            .execute(&self.pool).await.map_err(pg_err)?;
        Ok(res.rows_affected() > 0)
    }

    async fn find_by_name(&self, guild_id: &str, game_name: &str) -> Result<Option<Game>, DomainError> {
        let row: Option<GameRow> = sqlx::query_as(
            "SELECT id::text, guild_id, game_name, created_by, created_at::text FROM games WHERE guild_id = $1 AND LOWER(game_name) = LOWER($2)",
        ).bind(guild_id).bind(game_name).fetch_optional(&self.pool).await.map_err(pg_err)?;
        Ok(row.map(Into::into))
    }

    async fn subscribe(&self, guild_id: &str, game_id: &str, user_id: &str) -> Result<(), DomainError> {
        sqlx::query(
            "INSERT INTO game_subscriptions (guild_id, game_id, user_id) VALUES ($1, $2::uuid, $3) ON CONFLICT (game_id, user_id) DO NOTHING",
        ).bind(guild_id).bind(game_id).bind(user_id)
        .execute(&self.pool).await.map_err(pg_err)?;
        Ok(())
    }

    async fn unsubscribe(&self, guild_id: &str, game_id: &str, user_id: &str) -> Result<(), DomainError> {
        sqlx::query("DELETE FROM game_subscriptions WHERE guild_id = $1 AND game_id = $2::uuid AND user_id = $3")
            .bind(guild_id).bind(game_id).bind(user_id)
            .execute(&self.pool).await.map_err(pg_err)?;
        Ok(())
    }

    async fn get_subscribers(&self, game_id: &str) -> Result<Vec<String>, DomainError> {
        let rows: Vec<(String,)> = sqlx::query_as(
            "SELECT user_id FROM game_subscriptions WHERE game_id = $1::uuid",
        ).bind(game_id).fetch_all(&self.pool).await.map_err(pg_err)?;
        Ok(rows.into_iter().map(|r| r.0).collect())
    }

    async fn get_user_games(&self, guild_id: &str, user_id: &str) -> Result<Vec<Game>, DomainError> {
        let rows: Vec<GameRow> = sqlx::query_as(
            "SELECT g.id::text, g.guild_id, g.game_name, g.created_by, g.created_at::text \
             FROM games g INNER JOIN game_subscriptions gs ON gs.game_id = g.id \
             WHERE g.guild_id = $1 AND gs.user_id = $2 ORDER BY g.game_name",
        ).bind(guild_id).bind(user_id).fetch_all(&self.pool).await.map_err(pg_err)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }
}
