use async_trait::async_trait;
use sqlx::PgPool;

use crate::ports::outbound::casino::blackjack_table_repository::BlackjackTable;
use crate::ports::outbound::casino::blackjack_table_repository::BlackjackTablePlayer;
use crate::ports::outbound::casino::blackjack_table_repository::BlackjackTableRepository;
use super::super::pg_err;

pub struct PgBlackjackTableRepository { pool: PgPool }

impl PgBlackjackTableRepository {
    pub fn new(pool: PgPool) -> Self { Self { pool } }
}

#[async_trait]
impl BlackjackTableRepository for PgBlackjackTableRepository {
    async fn create(&self, guild_id: &str, channel_id: &str, owner_id: &str, owner_name: &str, deck_json: &serde_json::Value) -> Result<BlackjackTable, crate::domain::errors::DomainError> {
        let table: BlackjackTable = sqlx::query_as(
            "INSERT INTO blackjack_tables (guild_id, channel_id, owner_id, owner_name, deck) \
             VALUES ($1, $2, $3, $4, $5) \
             RETURNING id::text, guild_id, channel_id, owner_id, owner_name, status, created_at::text",
        ).bind(guild_id).bind(channel_id).bind(owner_id).bind(owner_name).bind(deck_json)
        .fetch_one(&self.pool).await.map_err(pg_err)?;
        // Owner = auto-joueur
        sqlx::query("INSERT INTO blackjack_table_players (table_id, user_id, user_name) VALUES ($1::uuid, $2, $3) ON CONFLICT DO NOTHING")
            .bind(&table.id).bind(owner_id).bind(owner_name)
            .execute(&self.pool).await.ok();
        Ok(table)
    }

    async fn get_status_and_guild(&self, table_id: &str) -> Result<Option<(String, String)>, crate::domain::errors::DomainError> {
        let row: Option<(String, String)> = sqlx::query_as("SELECT status, guild_id FROM blackjack_tables WHERE id = $1::uuid")
            .bind(table_id).fetch_optional(&self.pool).await.map_err(pg_err)?;
        Ok(row)
    }

    async fn count_players(&self, table_id: &str) -> Result<i64, crate::domain::errors::DomainError> {
        let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM blackjack_table_players WHERE table_id = $1::uuid")
            .bind(table_id).fetch_one(&self.pool).await.map_err(pg_err)?;
        Ok(count)
    }

    async fn add_player(&self, table_id: &str, user_id: &str, user_name: &str) -> Result<(), crate::domain::errors::DomainError> {
        sqlx::query("INSERT INTO blackjack_table_players (table_id, user_id, user_name) VALUES ($1::uuid, $2, $3) ON CONFLICT DO NOTHING")
            .bind(table_id).bind(user_id).bind(user_name)
            .execute(&self.pool).await.map_err(pg_err)?;
        Ok(())
    }

    async fn touch_activity(&self, table_id: &str) -> Result<(), crate::domain::errors::DomainError> {
        sqlx::query("UPDATE blackjack_tables SET last_activity = NOW() WHERE id = $1::uuid")
            .bind(table_id).execute(&self.pool).await.map_err(pg_err)?;
        Ok(())
    }

    async fn touch_activity_by_player(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<(), crate::domain::errors::DomainError> {
        // Bug fix : avant, seul `JoinTable` mettait a jour `last_activity`.
        // Une partie >30min faisait fermer la table par le cleanup-worker
        // alors qu'elle etait active. On touche desormais a chaque action
        // de jeu (start/hit/stand/double) via cette methode qui retrouve
        // la table ouverte du joueur. No-op si pas de table.
        sqlx::query(
            "UPDATE blackjack_tables \
             SET last_activity = NOW() \
             WHERE guild_id = $1 \
               AND status = 'open' \
               AND id IN (SELECT table_id FROM blackjack_table_players WHERE user_id = $2)",
        )
        .bind(guild_id)
        .bind(user_id)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(())
    }

    async fn list_players(&self, table_id: &str) -> Result<Vec<BlackjackTablePlayer>, crate::domain::errors::DomainError> {
        let rows: Vec<BlackjackTablePlayer> = sqlx::query_as(
            "SELECT user_id, user_name, joined_at::text FROM blackjack_table_players WHERE table_id = $1::uuid ORDER BY joined_at",
        ).bind(table_id).fetch_all(&self.pool).await.map_err(pg_err)?;
        Ok(rows)
    }

    async fn find_open_by_channel(&self, channel_id: &str) -> Result<Option<BlackjackTable>, crate::domain::errors::DomainError> {
        let row: Option<BlackjackTable> = sqlx::query_as(
            "SELECT id::text, guild_id, channel_id, owner_id, owner_name, status, created_at::text \
             FROM blackjack_tables WHERE channel_id = $1 AND status = 'open'",
        ).bind(channel_id).fetch_optional(&self.pool).await.map_err(pg_err)?;
        Ok(row)
    }

    async fn get_guild_id(&self, table_id: &str) -> Result<Option<String>, crate::domain::errors::DomainError> {
        let row: Option<(String,)> = sqlx::query_as("SELECT guild_id FROM blackjack_tables WHERE id = $1::uuid")
            .bind(table_id).fetch_optional(&self.pool).await.map_err(pg_err)?;
        Ok(row.map(|r| r.0))
    }

    async fn close(&self, table_id: &str) -> Result<(), crate::domain::errors::DomainError> {
        sqlx::query("UPDATE blackjack_tables SET status = 'closed' WHERE id = $1::uuid AND status = 'open'")
            .bind(table_id).execute(&self.pool).await.map_err(pg_err)?;
        Ok(())
    }

    async fn list_games(&self, table_id: &str) -> Result<Vec<serde_json::Value>, crate::domain::errors::DomainError> {
        let rows: Vec<(String, String, String, String, i64, i64)> = sqlx::query_as(
            "SELECT id::text, user_id, username, status, bet, payout FROM blackjack_games WHERE table_id = $1::uuid ORDER BY created_at DESC",
        ).bind(table_id).fetch_all(&self.pool).await.map_err(pg_err)?;
        Ok(rows.iter().map(|(id, uid, name, status, bet, payout)| {
            serde_json::json!({"id": id, "user_id": uid, "username": name, "status": status, "bet": bet, "payout": payout})
        }).collect())
    }

    async fn list_open_by_guild(
        &self,
        guild_id: &str,
    ) -> Result<Vec<BlackjackTable>, crate::domain::errors::DomainError> {
        let tables: Vec<BlackjackTable> = sqlx::query_as(
            "SELECT id::text, guild_id, channel_id, owner_id, owner_name, status, created_at::text \
             FROM blackjack_tables \
             WHERE guild_id = $1 AND status = 'open' \
             ORDER BY created_at DESC",
        )
        .bind(guild_id)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(tables)
    }
}
