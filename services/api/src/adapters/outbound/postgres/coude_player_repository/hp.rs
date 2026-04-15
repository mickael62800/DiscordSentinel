//! Gestion des HP : update, full_heal (repos), regen_hp_tick.

use crate::domain::errors::DomainError;

use super::{pg_err, PgCoudePlayerRepository};

pub(super) async fn update_hp(
    repo: &PgCoudePlayerRepository,
    guild_id: &str,
    user_id: &str,
    hp_current: i32,
    hp_max: i32,
) -> Result<(), DomainError> {
    sqlx::query(
        "UPDATE coude_players
         SET hp_current = $3, hp_max = $4, hp_last_regen = NOW(), updated_at = NOW()
         WHERE guild_id = $1 AND user_id = $2",
    )
    .bind(guild_id)
    .bind(user_id)
    .bind(hp_current)
    .bind(hp_max)
    .execute(&repo.pool)
    .await
    .map_err(pg_err)?;
    Ok(())
}

pub(super) async fn full_heal(
    repo: &PgCoudePlayerRepository,
    guild_id: &str,
    user_id: &str,
) -> Result<(), DomainError> {
    sqlx::query(
        "UPDATE coude_players
         SET hp_current = hp_max,
             repos_last_used = NOW(),
             hp_last_regen = NOW(),
             updated_at = NOW()
         WHERE guild_id = $1 AND user_id = $2",
    )
    .bind(guild_id)
    .bind(user_id)
    .execute(&repo.pool)
    .await
    .map_err(pg_err)?;
    Ok(())
}

pub(super) async fn regen_hp_tick(
    repo: &PgCoudePlayerRepository,
    rate_0_25: f64,
    rate_25_50: f64,
    rate_50_75: f64,
    rate_75_100: f64,
) -> Result<u64, DomainError> {
    // Phase 4 : SQL migree depuis coude-worker/src/jobs/hp_regen.rs.
    // Exclut les joueurs avec un combat actif via NOT EXISTS.
    let result = sqlx::query(
        r#"
        WITH regen AS (
            SELECT
                guild_id,
                user_id,
                FLOOR(
                    (CASE
                        WHEN hp_current * 4 < hp_max THEN $1::float8
                        WHEN hp_current * 2 < hp_max THEN $2::float8
                        WHEN hp_current * 4 < hp_max * 3 THEN $3::float8
                        ELSE $4::float8
                    END) * EXTRACT(EPOCH FROM (NOW() - hp_last_regen)) / 3600.0
                )::int AS amount
            FROM coude_players p
            WHERE hp_current < hp_max
              AND hp_last_regen IS NOT NULL
              AND NOT EXISTS (
                  SELECT 1 FROM coude_combats c
                  WHERE c.guild_id = p.guild_id
                    AND (c.attacker_id = p.user_id OR c.defender_id = p.user_id)
                    AND c.status IN ('pending', 'betting', 'resolving')
              )
        )
        UPDATE coude_players p
        SET hp_current = LEAST(p.hp_max, p.hp_current + r.amount),
            hp_last_regen = NOW(),
            updated_at = NOW()
        FROM regen r
        WHERE p.guild_id = r.guild_id
          AND p.user_id = r.user_id
          AND r.amount > 0
        "#,
    )
    .bind(rate_0_25)
    .bind(rate_25_50)
    .bind(rate_50_75)
    .bind(rate_75_100)
    .execute(&repo.pool)
    .await
    .map_err(pg_err)?;
    Ok(result.rows_affected())
}
