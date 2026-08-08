//! Adapter Postgres du port `OAuthSessionRepository` : sessions web OAuth
//! (`web_oauth_sessions`) et trace des logins reussis (`successful_logins`).
//! Tout le SQL du flux OAuth web vit ici.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use super::super::pg_err_ctx;
use sentinel_core::domain::entities::system::oauth::{
    LoginTrace, NewOAuthSession, OAuthSession, SessionTokenUpdate,
};
use sentinel_core::domain::errors::DomainError;
use sentinel_core::ports::outbound::system::oauth_session_repository::OAuthSessionRepository;

const TBL: &str = "web_oauth_sessions";
fn pg_err(e: sqlx::Error) -> DomainError {
    pg_err_ctx(TBL, e)
}

#[derive(sqlx::FromRow)]
struct SessionRow {
    discord_user_id: String,
    username: String,
    global_name: Option<String>,
    avatar: Option<String>,
    access_token: String,
    refresh_token: String,
    access_expires_at: DateTime<Utc>,
}

pub struct PgOAuthSessionRepository {
    pool: PgPool,
}

impl PgOAuthSessionRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl OAuthSessionRepository for PgOAuthSessionRepository {
    async fn record_login(&self, trace: LoginTrace) -> Result<(), DomainError> {
        sqlx::query(
            "INSERT INTO successful_logins (discord_user_id, username, client_ip, user_agent) \
             VALUES ($1, $2, $3, $4)",
        )
        .bind(&trace.discord_user_id)
        .bind(&trace.username)
        .bind(&trace.client_ip)
        .bind(&trace.user_agent)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(())
    }

    async fn create_session(&self, session: NewOAuthSession) -> Result<(), DomainError> {
        sqlx::query(
            "INSERT INTO web_oauth_sessions \
                (id, discord_user_id, username, global_name, avatar, access_token, refresh_token, access_expires_at) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8)",
        )
        .bind(session.id)
        .bind(&session.discord_user_id)
        .bind(&session.username)
        .bind(&session.global_name)
        .bind(&session.avatar)
        .bind(&session.access_token)
        .bind(&session.refresh_token)
        .bind(session.access_expires_at)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(())
    }

    async fn get_session(&self, id: Uuid) -> Result<Option<OAuthSession>, DomainError> {
        let row: Option<SessionRow> = sqlx::query_as(
            "SELECT discord_user_id, username, global_name, avatar, access_token, refresh_token, access_expires_at \
             FROM web_oauth_sessions WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_err)?;

        Ok(row.map(|s| OAuthSession {
            discord_user_id: s.discord_user_id,
            username: s.username,
            global_name: s.global_name,
            avatar: s.avatar,
            access_token: s.access_token,
            refresh_token: s.refresh_token,
            access_expires_at: s.access_expires_at,
        }))
    }

    async fn touch_session(&self, id: Uuid) -> Result<(), DomainError> {
        sqlx::query("UPDATE web_oauth_sessions SET last_used_at = NOW() WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(())
    }

    async fn update_tokens(&self, update: SessionTokenUpdate) -> Result<(), DomainError> {
        sqlx::query(
            "UPDATE web_oauth_sessions SET access_token = $2, refresh_token = $3, \
                access_expires_at = $4, last_used_at = NOW() WHERE id = $1",
        )
        .bind(update.id)
        .bind(&update.access_token)
        .bind(&update.refresh_token)
        .bind(update.access_expires_at)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(())
    }

    async fn delete_session(&self, id: Uuid) -> Result<(), DomainError> {
        sqlx::query("DELETE FROM web_oauth_sessions WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(())
    }
}
