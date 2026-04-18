use async_trait::async_trait;
use sqlx::PgPool;

use crate::domain::errors::DomainError;
use crate::ports::outbound::{Game, GamePanel, GameRepository};
use super::pg_err;

pub struct PgGameRepository { pool: PgPool }

impl PgGameRepository {
    pub fn new(pool: PgPool) -> Self { Self { pool } }
}

#[derive(sqlx::FromRow)]
struct GameRow {
    id: String,
    guild_id: String,
    game_name: String,
    created_by: String,
    created_at: String,
    emoji: Option<String>,
    category: Option<String>,
}

impl From<GameRow> for Game {
    fn from(r: GameRow) -> Self {
        Self {
            id: r.id,
            guild_id: r.guild_id,
            game_name: r.game_name,
            created_by: r.created_by,
            created_at: r.created_at,
            emoji: r.emoji,
            category: r.category,
        }
    }
}

#[derive(sqlx::FromRow)]
struct PanelRow {
    id: String,
    guild_id: String,
    channel_id: String,
    message_id: String,
    category: Option<String>,
}

impl From<PanelRow> for GamePanel {
    fn from(r: PanelRow) -> Self {
        Self {
            id: r.id,
            guild_id: r.guild_id,
            channel_id: r.channel_id,
            message_id: r.message_id,
            category: r.category,
        }
    }
}

const GAME_COLS: &str =
    "id::text, guild_id, game_name, created_by, created_at::text, emoji, category";

#[async_trait]
impl GameRepository for PgGameRepository {
    async fn list(&self, guild_id: &str) -> Result<Vec<Game>, DomainError> {
        let sql = format!(
            "SELECT {GAME_COLS} FROM games WHERE guild_id = $1 ORDER BY game_name"
        );
        let rows: Vec<GameRow> = sqlx::query_as(&sql)
            .bind(guild_id).fetch_all(&self.pool).await.map_err(pg_err)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn list_by_category(&self, guild_id: &str, category: Option<&str>) -> Result<Vec<Game>, DomainError> {
        let rows: Vec<GameRow> = match category {
            Some(cat) => {
                let sql = format!(
                    "SELECT {GAME_COLS} FROM games WHERE guild_id = $1 AND LOWER(category) = LOWER($2) ORDER BY game_name"
                );
                sqlx::query_as(&sql).bind(guild_id).bind(cat)
                    .fetch_all(&self.pool).await.map_err(pg_err)?
            }
            None => {
                let sql = format!(
                    "SELECT {GAME_COLS} FROM games WHERE guild_id = $1 AND category IS NULL ORDER BY game_name"
                );
                sqlx::query_as(&sql).bind(guild_id)
                    .fetch_all(&self.pool).await.map_err(pg_err)?
            }
        };
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn create(&self, guild_id: &str, game_name: &str, created_by: &str, emoji: Option<&str>, category: Option<&str>) -> Result<Game, DomainError> {
        let sql = format!(
            "INSERT INTO games (guild_id, game_name, created_by, emoji, category) VALUES ($1, $2, $3, $4, $5) \
             RETURNING {GAME_COLS}"
        );
        let row: GameRow = sqlx::query_as(&sql)
            .bind(guild_id).bind(game_name).bind(created_by).bind(emoji).bind(category)
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

    async fn update(&self, guild_id: &str, game_id: &str, game_name: Option<&str>, emoji: Option<Option<&str>>, category: Option<Option<&str>>) -> Result<Option<Game>, DomainError> {
        // Utilise COALESCE pour ne mettre a jour que les champs fournis.
        // Pour emoji/category, on distingue "pas touche" (None) vs "mettre a NULL" (Some(None)).
        let update_name = game_name.is_some();
        let update_emoji = emoji.is_some();
        let update_category = category.is_some();
        if !update_name && !update_emoji && !update_category {
            // Rien a mettre a jour — on relit juste le jeu.
            let sql = format!(
                "SELECT {GAME_COLS} FROM games WHERE guild_id = $1 AND id = $2::uuid"
            );
            let row: Option<GameRow> = sqlx::query_as(&sql)
                .bind(guild_id).bind(game_id).fetch_optional(&self.pool).await.map_err(pg_err)?;
            return Ok(row.map(Into::into));
        }
        let sql = format!(
            "UPDATE games SET \
                game_name = CASE WHEN $3::bool THEN $4 ELSE game_name END, \
                emoji = CASE WHEN $5::bool THEN $6 ELSE emoji END, \
                category = CASE WHEN $7::bool THEN $8 ELSE category END \
             WHERE guild_id = $1 AND id = $2::uuid \
             RETURNING {GAME_COLS}"
        );
        let row: Option<GameRow> = sqlx::query_as(&sql)
            .bind(guild_id)
            .bind(game_id)
            .bind(update_name)
            .bind(game_name.unwrap_or(""))
            .bind(update_emoji)
            .bind(emoji.flatten())
            .bind(update_category)
            .bind(category.flatten())
            .fetch_optional(&self.pool).await
            .map_err(|e| {
                if e.to_string().contains("idx_games_guild_name") {
                    DomainError::Conflict("Un jeu avec ce nom existe deja".into())
                } else {
                    pg_err(e)
                }
            })?;
        Ok(row.map(Into::into))
    }

    async fn delete(&self, guild_id: &str, game_id: &str) -> Result<bool, DomainError> {
        let res = sqlx::query("DELETE FROM games WHERE guild_id = $1 AND id = $2::uuid")
            .bind(guild_id).bind(game_id)
            .execute(&self.pool).await.map_err(pg_err)?;
        Ok(res.rows_affected() > 0)
    }

    async fn find_by_name(&self, guild_id: &str, game_name: &str) -> Result<Option<Game>, DomainError> {
        let sql = format!(
            "SELECT {GAME_COLS} FROM games WHERE guild_id = $1 AND LOWER(game_name) = LOWER($2)"
        );
        let row: Option<GameRow> = sqlx::query_as(&sql)
            .bind(guild_id).bind(game_name).fetch_optional(&self.pool).await.map_err(pg_err)?;
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
        let sql = format!(
            "SELECT g.id::text, g.guild_id, g.game_name, g.created_by, g.created_at::text, g.emoji, g.category \
             FROM games g INNER JOIN game_subscriptions gs ON gs.game_id = g.id \
             WHERE g.guild_id = $1 AND gs.user_id = $2 ORDER BY g.game_name"
        );
        let rows: Vec<GameRow> = sqlx::query_as(&sql)
            .bind(guild_id).bind(user_id).fetch_all(&self.pool).await.map_err(pg_err)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn save_panel(&self, guild_id: &str, channel_id: &str, message_id: &str, category: Option<&str>) -> Result<GamePanel, DomainError> {
        // Upsert sur (guild_id, COALESCE(category, '')).
        let row: PanelRow = sqlx::query_as(
            "INSERT INTO game_panels (guild_id, channel_id, message_id, category) \
             VALUES ($1, $2, $3, $4) \
             ON CONFLICT (guild_id, COALESCE(category, '')) DO UPDATE SET \
               channel_id = EXCLUDED.channel_id, \
               message_id = EXCLUDED.message_id \
             RETURNING id::text, guild_id, channel_id, message_id, category",
        )
        .bind(guild_id).bind(channel_id).bind(message_id).bind(category)
        .fetch_one(&self.pool).await.map_err(pg_err)?;
        Ok(row.into())
    }

    async fn find_panel_by_message(&self, guild_id: &str, message_id: &str) -> Result<Option<GamePanel>, DomainError> {
        let row: Option<PanelRow> = sqlx::query_as(
            "SELECT id::text, guild_id, channel_id, message_id, category FROM game_panels WHERE guild_id = $1 AND message_id = $2",
        )
        .bind(guild_id).bind(message_id)
        .fetch_optional(&self.pool).await.map_err(pg_err)?;
        Ok(row.map(Into::into))
    }

    async fn list_panels(&self, guild_id: &str) -> Result<Vec<GamePanel>, DomainError> {
        let rows: Vec<PanelRow> = sqlx::query_as(
            "SELECT id::text, guild_id, channel_id, message_id, category FROM game_panels WHERE guild_id = $1 ORDER BY category NULLS FIRST",
        )
        .bind(guild_id)
        .fetch_all(&self.pool).await.map_err(pg_err)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }
}
