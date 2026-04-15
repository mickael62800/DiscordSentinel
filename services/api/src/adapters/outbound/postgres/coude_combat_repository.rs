use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::entities::{CombatResolution, CoudeCombat, NewCoudeCombat};
use crate::domain::errors::DomainError;
use crate::ports::outbound::CoudeCombatRepository;

pub struct PgCoudeCombatRepository {
    pool: PgPool,
}

impl PgCoudeCombatRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

const COMBAT_COLUMNS: &str = r#"
    id, guild_id, channel_id, attacker_id, attacker_name, defender_id, defender_name,
    mise, status, winner_id, attacker_roll, defender_roll,
    chaos_event, special_attack, defender_special, coins_transferred,
    result_message, message_id,
    created_at, accepted_at, resolved_at
"#;

#[derive(sqlx::FromRow)]
struct CombatRow {
    id: Uuid,
    guild_id: String,
    channel_id: Option<String>,
    attacker_id: String,
    attacker_name: String,
    defender_id: String,
    defender_name: String,
    mise: i64,
    status: String,
    winner_id: Option<String>,
    attacker_roll: Option<i32>,
    defender_roll: Option<i32>,
    chaos_event: Option<String>,
    special_attack: Option<String>,
    defender_special: Option<String>,
    coins_transferred: Option<i64>,
    result_message: Option<String>,
    message_id: Option<String>,
    created_at: DateTime<Utc>,
    accepted_at: Option<DateTime<Utc>>,
    resolved_at: Option<DateTime<Utc>>,
}

impl From<CombatRow> for CoudeCombat {
    fn from(r: CombatRow) -> Self {
        Self {
            id: r.id,
            guild_id: r.guild_id,
            channel_id: r.channel_id,
            attacker_id: r.attacker_id,
            attacker_name: r.attacker_name,
            defender_id: r.defender_id,
            defender_name: r.defender_name,
            mise: r.mise,
            status: r.status,
            winner_id: r.winner_id,
            attacker_roll: r.attacker_roll,
            defender_roll: r.defender_roll,
            chaos_event: r.chaos_event,
            special_attack: r.special_attack,
            defender_special: r.defender_special,
            coins_transferred: r.coins_transferred,
            result_message: r.result_message,
            message_id: r.message_id,
            created_at: r.created_at,
            accepted_at: r.accepted_at,
            resolved_at: r.resolved_at,
        }
    }
}

fn pg_err(e: sqlx::Error) -> DomainError {
    DomainError::Internal(e.to_string())
}

#[async_trait]
impl CoudeCombatRepository for PgCoudeCombatRepository {
    async fn list(
        &self,
        guild_id: &str,
        status: Option<&str>,
        limit: i64,
    ) -> Result<Vec<CoudeCombat>, DomainError> {
        // On lit directement les colonnes gelees a la creation du combat.
        // Pas de JOIN sur coude_players : le fallback COALESCE causait la
        // disparition des noms lorsque la ligne player etait absente ou
        // avait un username vide.
        let rows: Vec<CombatRow> = match status {
            Some(s) => {
                let sql = format!(
                    "SELECT {cols} FROM coude_combats \
                     WHERE guild_id = $1 AND status = $2 \
                     ORDER BY created_at DESC LIMIT $3",
                    cols = COMBAT_COLUMNS
                );
                sqlx::query_as(&sql)
                    .bind(guild_id)
                    .bind(s)
                    .bind(limit)
                    .fetch_all(&self.pool)
                    .await
                    .map_err(pg_err)?
            }
            None => {
                let sql = format!(
                    "SELECT {cols} FROM coude_combats \
                     WHERE guild_id = $1 \
                     ORDER BY created_at DESC LIMIT $2",
                    cols = COMBAT_COLUMNS
                );
                sqlx::query_as(&sql)
                    .bind(guild_id)
                    .bind(limit)
                    .fetch_all(&self.pool)
                    .await
                    .map_err(pg_err)?
            }
        };
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn get(&self, id: Uuid) -> Result<Option<CoudeCombat>, DomainError> {
        let sql = format!(
            "SELECT {cols} FROM coude_combats WHERE id = $1",
            cols = COMBAT_COLUMNS
        );
        let row: Option<CombatRow> = sqlx::query_as(&sql)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(row.map(Into::into))
    }

    async fn get_pending_for_attacker(
        &self,
        guild_id: &str,
        attacker_id: &str,
    ) -> Result<Option<CoudeCombat>, DomainError> {
        let sql = format!(
            r#"SELECT {cols}
               FROM coude_combats
               WHERE guild_id = $1 AND attacker_id = $2 AND status = 'pending'
               ORDER BY created_at DESC
               LIMIT 1"#,
            cols = COMBAT_COLUMNS
        );
        let row: Option<CombatRow> = sqlx::query_as(&sql)
            .bind(guild_id)
            .bind(attacker_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(row.map(Into::into))
    }

    async fn get_pending_for_defender(
        &self,
        guild_id: &str,
        defender_id: &str,
    ) -> Result<Option<CoudeCombat>, DomainError> {
        let sql = format!(
            r#"SELECT {cols}
               FROM coude_combats
               WHERE guild_id = $1 AND defender_id = $2 AND status = 'pending'
               ORDER BY created_at DESC
               LIMIT 1"#,
            cols = COMBAT_COLUMNS
        );
        let row: Option<CombatRow> = sqlx::query_as(&sql)
            .bind(guild_id)
            .bind(defender_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(row.map(Into::into))
    }

    async fn get_betting_for_participant(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<CoudeCombat>, DomainError> {
        let sql = format!(
            r#"SELECT {cols}
               FROM coude_combats
               WHERE guild_id = $1
                 AND (attacker_id = $2 OR defender_id = $2)
                 AND status = 'betting'
               ORDER BY created_at DESC
               LIMIT 1"#,
            cols = COMBAT_COLUMNS
        );
        let row: Option<CombatRow> = sqlx::query_as(&sql)
            .bind(guild_id)
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(row.map(Into::into))
    }

    async fn list_expired_pending(&self) -> Result<Vec<CoudeCombat>, DomainError> {
        let sql = format!(
            r#"SELECT {cols}
               FROM coude_combats
               WHERE status = 'pending' AND created_at < NOW() - INTERVAL '24 hours'"#,
            cols = COMBAT_COLUMNS
        );
        let rows: Vec<CombatRow> = sqlx::query_as(&sql)
            .fetch_all(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn claim_due_betting_combats(
        &self,
        default_delay_secs: i64,
    ) -> Result<Vec<CoudeCombat>, DomainError> {
        // Phase 2 : SQL deplacee depuis coude-worker/src/jobs/resolve_betting.rs.
        // Le delai par guild est lu depuis bot_guild_config (bot_name =
        // 'coude-worker', config_key = 'bet_delay_secs'), avec fallback sur
        // le parametre default_delay_secs (typiquement 300s).
        // FOR UPDATE SKIP LOCKED → deux batchs concurrents ne traiteront
        // jamais le meme combat.
        let sql = format!(
            r#"UPDATE coude_combats SET status = 'resolving'
               WHERE id IN (
                   SELECT c.id FROM coude_combats c
                   LEFT JOIN bot_guild_config cfg
                       ON cfg.guild_id = c.guild_id
                       AND cfg.bot_name = 'coude-worker'
                       AND cfg.config_key = 'bet_delay_secs'
                   WHERE c.status = 'betting'
                     AND c.accepted_at < NOW() - (COALESCE(
                           CASE WHEN cfg.config_value ~ '^\d+$' THEN cfg.config_value::int ELSE NULL END,
                           $1
                         ) * INTERVAL '1 second')
                   FOR UPDATE OF c SKIP LOCKED
               )
               RETURNING {cols}"#,
            cols = COMBAT_COLUMNS
        );
        let rows: Vec<CombatRow> = sqlx::query_as(&sql)
            .bind(default_delay_secs)
            .fetch_all(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn claim_stuck_resolving_combats(
        &self,
        stuck_threshold_secs: i64,
    ) -> Result<Vec<CoudeCombat>, DomainError> {
        // Phase 2 : SQL deplacee depuis coude-worker/src/jobs/resolve_betting.rs.
        // Touche accepted_at a NOW() pour empecher qu'un tick ulterieur
        // reprenne le combat avant que le tick courant ait termine.
        let sql = format!(
            r#"UPDATE coude_combats SET accepted_at = NOW()
               WHERE id IN (
                   SELECT id FROM coude_combats
                   WHERE status = 'resolving'
                     AND accepted_at < NOW() - ($1 * INTERVAL '1 second')
                   FOR UPDATE SKIP LOCKED
               )
               RETURNING {cols}"#,
            cols = COMBAT_COLUMNS
        );
        let rows: Vec<CombatRow> = sqlx::query_as(&sql)
            .bind(stuck_threshold_secs)
            .fetch_all(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn create(&self, new: NewCoudeCombat) -> Result<CoudeCombat, DomainError> {
        let sql = format!(
            r#"INSERT INTO coude_combats
                 (guild_id, channel_id, attacker_id, attacker_name, defender_id, defender_name, mise, special_attack)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
               RETURNING {cols}"#,
            cols = COMBAT_COLUMNS
        );
        let row: CombatRow = sqlx::query_as(&sql)
            .bind(&new.guild_id)
            .bind(&new.channel_id)
            .bind(&new.attacker_id)
            .bind(&new.attacker_name)
            .bind(&new.defender_id)
            .bind(&new.defender_name)
            .bind(new.mise)
            .bind(&new.special_attack)
            .fetch_one(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(row.into())
    }

    async fn resolve(
        &self,
        id: Uuid,
        resolution: CombatResolution,
    ) -> Result<bool, DomainError> {
        // Garde : ne résout que les combats encore actifs.
        let result = sqlx::query(
            r#"UPDATE coude_combats
               SET status = $2,
                   winner_id = $3,
                   attacker_roll = $4,
                   defender_roll = $5,
                   chaos_event = $6,
                   result_message = $7,
                   coins_transferred = $8,
                   resolved_at = NOW()
               WHERE id = $1 AND status IN ('pending', 'accepted', 'betting')"#,
        )
        .bind(id)
        .bind(&resolution.status)
        .bind(&resolution.winner_id)
        .bind(resolution.attacker_roll)
        .bind(resolution.defender_roll)
        .bind(&resolution.chaos_event)
        .bind(&resolution.result_message)
        .bind(resolution.coins_transferred)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(result.rows_affected() > 0)
    }

    async fn set_betting(&self, id: Uuid, message_id: &str) -> Result<bool, DomainError> {
        let result = sqlx::query(
            r#"UPDATE coude_combats
               SET status = 'betting', accepted_at = NOW(), message_id = $1
               WHERE id = $2 AND status = 'pending'"#,
        )
        .bind(message_id)
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(result.rows_affected() > 0)
    }

    async fn expire(&self, id: Uuid) -> Result<bool, DomainError> {
        let result = sqlx::query(
            "UPDATE coude_combats SET status = 'expired', resolved_at = NOW() WHERE id = $1",
        )
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(result.rows_affected() > 0)
    }

    async fn cancel_pending(&self, id: Uuid) -> Result<bool, DomainError> {
        let result = sqlx::query(
            r#"UPDATE coude_combats
               SET status = 'expired', resolved_at = NOW()
               WHERE id = $1 AND status = 'pending'"#,
        )
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(result.rows_affected() > 0)
    }

    async fn set_defender_special(
        &self,
        id: Uuid,
        item_key: &str,
    ) -> Result<bool, DomainError> {
        let result = sqlx::query(
            "UPDATE coude_combats SET defender_special = $1 WHERE id = $2",
        )
        .bind(item_key)
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(result.rows_affected() > 0)
    }

    async fn mark_unresolved_bets_lost(&self, combat_id: Uuid) -> Result<(), DomainError> {
        sqlx::query(
            "UPDATE coude_bets SET won = false WHERE combat_id = $1 AND won IS NULL",
        )
        .bind(combat_id)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(())
    }
}
