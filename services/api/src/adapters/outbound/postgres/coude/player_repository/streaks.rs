//! Streaks : win/loss combat + steal victim (Phase 9 Part D).

use crate::domain::errors::DomainError;

use super::super::super::pg_err;
use super::PgCoudePlayerRepository;
pub(super) async fn touch_win_streak(
    repo: &PgCoudePlayerRepository,
    guild_id: &str,
    user_id: &str,
) -> Result<Option<i32>, DomainError> {
    let row: Option<(i32,)> = sqlx::query_as(
        r#"UPDATE coude_players
           SET current_win_streak = current_win_streak + 1,
               current_loss_streak = 0,
               updated_at = NOW()
           WHERE guild_id = $1 AND user_id = $2
           RETURNING current_win_streak"#,
    )
    .bind(guild_id)
    .bind(user_id)
    .fetch_optional(&repo.pool)
    .await
    .map_err(pg_err)?;
    Ok(row.map(|r| r.0))
}

pub(super) async fn touch_loss_streak(
    repo: &PgCoudePlayerRepository,
    guild_id: &str,
    user_id: &str,
) -> Result<Option<i32>, DomainError> {
    let row: Option<(i32,)> = sqlx::query_as(
        r#"UPDATE coude_players
           SET current_loss_streak = current_loss_streak + 1,
               current_win_streak = 0,
               updated_at = NOW()
           WHERE guild_id = $1 AND user_id = $2
           RETURNING current_loss_streak"#,
    )
    .bind(guild_id)
    .bind(user_id)
    .fetch_optional(&repo.pool)
    .await
    .map_err(pg_err)?;
    Ok(row.map(|r| r.0))
}

pub(super) async fn reset_combat_streaks(
    repo: &PgCoudePlayerRepository,
    guild_id: &str,
    user_id: &str,
) -> Result<(), DomainError> {
    sqlx::query(
        r#"UPDATE coude_players
           SET current_win_streak = 0,
               current_loss_streak = 0,
               updated_at = NOW()
           WHERE guild_id = $1 AND user_id = $2"#,
    )
    .bind(guild_id)
    .bind(user_id)
    .execute(&repo.pool)
    .await
    .map_err(pg_err)?;
    Ok(())
}

pub(super) async fn touch_steal_victim_streak(
    repo: &PgCoudePlayerRepository,
    guild_id: &str,
    user_id: &str,
) -> Result<Option<i32>, DomainError> {
    let row: Option<(i32,)> = sqlx::query_as(
        r#"UPDATE coude_players
           SET current_steal_victim_streak = current_steal_victim_streak + 1,
               updated_at = NOW()
           WHERE guild_id = $1 AND user_id = $2
           RETURNING current_steal_victim_streak"#,
    )
    .bind(guild_id)
    .bind(user_id)
    .fetch_optional(&repo.pool)
    .await
    .map_err(pg_err)?;
    Ok(row.map(|r| r.0))
}

pub(super) async fn reset_steal_victim_streak(
    repo: &PgCoudePlayerRepository,
    guild_id: &str,
    user_id: &str,
) -> Result<(), DomainError> {
    sqlx::query(
        r#"UPDATE coude_players
           SET current_steal_victim_streak = 0, updated_at = NOW()
           WHERE guild_id = $1 AND user_id = $2"#,
    )
    .bind(guild_id)
    .bind(user_id)
    .execute(&repo.pool)
    .await
    .map_err(pg_err)?;
    Ok(())
}

// ── Blackjack streaks (migration 139) ──

pub(super) async fn touch_bj_win_streak(
    repo: &PgCoudePlayerRepository,
    guild_id: &str,
    user_id: &str,
) -> Result<Option<i32>, DomainError> {
    let row: Option<(i32,)> = sqlx::query_as(
        r#"UPDATE coude_players
           SET bj_win_streak = bj_win_streak + 1,
               bj_bust_streak = 0,
               updated_at = NOW()
           WHERE guild_id = $1 AND user_id = $2
           RETURNING bj_win_streak"#,
    )
    .bind(guild_id)
    .bind(user_id)
    .fetch_optional(&repo.pool)
    .await
    .map_err(pg_err)?;
    Ok(row.map(|r| r.0))
}

pub(super) async fn touch_bj_bust_streak(
    repo: &PgCoudePlayerRepository,
    guild_id: &str,
    user_id: &str,
) -> Result<Option<i32>, DomainError> {
    let row: Option<(i32,)> = sqlx::query_as(
        r#"UPDATE coude_players
           SET bj_bust_streak = bj_bust_streak + 1,
               bj_win_streak = 0,
               updated_at = NOW()
           WHERE guild_id = $1 AND user_id = $2
           RETURNING bj_bust_streak"#,
    )
    .bind(guild_id)
    .bind(user_id)
    .fetch_optional(&repo.pool)
    .await
    .map_err(pg_err)?;
    Ok(row.map(|r| r.0))
}

pub(super) async fn reset_bj_bust_streak(
    repo: &PgCoudePlayerRepository,
    guild_id: &str,
    user_id: &str,
) -> Result<(), DomainError> {
    sqlx::query(
        r#"UPDATE coude_players
           SET bj_bust_streak = 0, updated_at = NOW()
           WHERE guild_id = $1 AND user_id = $2"#,
    )
    .bind(guild_id)
    .bind(user_id)
    .execute(&repo.pool)
    .await
    .map_err(pg_err)?;
    Ok(())
}
