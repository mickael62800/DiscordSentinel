//! Impl Postgres de `CashboxRepository`.

use async_trait::async_trait;
use chrono::DateTime;
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use crate::ports::outbound::coude::cashbox_repository::CashboxRepository;
use sentinel_core::domain::entities::coude::cashbox::Cashbox;
use sentinel_core::domain::entities::coude::cashbox::CashboxRedistribution;
use sentinel_core::domain::entities::coude::cashbox::CashboxRedistributionEntry;
use sentinel_core::domain::entities::coude::cashbox::CashboxSource;
use sentinel_core::domain::errors::DomainError;

use super::super::pg_err_ctx;
const TBL: &str = "cashbox";
fn pg_err(e: sqlx::Error) -> DomainError {
    pg_err_ctx(TBL, e)
}

pub struct PgCashboxRepository {
    pool: PgPool,
}

impl PgCashboxRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct CashboxRow {
    guild_id: String,
    balance: i64,
    total_collected: i64,
    total_redistributed: i64,
    last_redistribution_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<CashboxRow> for Cashbox {
    fn from(r: CashboxRow) -> Self {
        Self {
            guild_id: r.guild_id.into(),
            balance: r.balance,
            total_collected: r.total_collected,
            total_redistributed: r.total_redistributed,
            last_redistribution_at: r.last_redistribution_at,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct RedistributionRow {
    id: Uuid,
    guild_id: String,
    total_amount: i64,
    winners_count: i32,
    created_at: DateTime<Utc>,
}

impl From<RedistributionRow> for CashboxRedistribution {
    fn from(r: RedistributionRow) -> Self {
        Self {
            id: r.id,
            guild_id: r.guild_id.into(),
            total_amount: r.total_amount,
            winners_count: r.winners_count,
            created_at: r.created_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct EntryRow {
    id: Uuid,
    redistribution_id: Uuid,
    user_id: String,
    username: String,
    amount_won: i64,
    created_at: DateTime<Utc>,
}

impl From<EntryRow> for CashboxRedistributionEntry {
    fn from(r: EntryRow) -> Self {
        Self {
            id: r.id,
            redistribution_id: r.redistribution_id,
            user_id: r.user_id.into(),
            username: r.username,
            amount_won: r.amount_won,
            created_at: r.created_at,
        }
    }
}

#[async_trait]
impl CashboxRepository for PgCashboxRepository {
    async fn get_or_create(&self, guild_id: &str) -> Result<Cashbox, DomainError> {
        let row: CashboxRow = sqlx::query_as(
            r#"INSERT INTO coude_cashbox (guild_id)
               VALUES ($1)
               ON CONFLICT (guild_id) DO UPDATE SET updated_at = coude_cashbox.updated_at
               RETURNING *"#,
        )
        .bind(guild_id)
        .fetch_one(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(row.into())
    }

    async fn deposit(
        &self,
        guild_id: &str,
        amount: i64,
        _source: CashboxSource,
    ) -> Result<(), DomainError> {
        if amount <= 0 {
            return Ok(());
        }
        sqlx::query(
            r#"INSERT INTO coude_cashbox (guild_id, balance, total_collected)
               VALUES ($1, $2, $2)
               ON CONFLICT (guild_id) DO UPDATE SET
                   balance = coude_cashbox.balance + EXCLUDED.balance,
                   total_collected = coude_cashbox.total_collected + EXCLUDED.total_collected,
                   updated_at = NOW()"#,
        )
        .bind(guild_id)
        .bind(amount)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(())
    }

    async fn withdraw(&self, guild_id: &str, amount: i64) -> Result<i64, DomainError> {
        if amount <= 0 {
            return Ok(0);
        }
        // Transaction : SELECT FOR UPDATE puis UPDATE avec clamp a 0.
        // Retourne le montant effectivement retire (peut etre < amount
        // si la caisse etait trop petite).
        let mut tx = self.pool.begin().await.map_err(pg_err)?;

        let row: Option<(i64,)> =
            sqlx::query_as("SELECT balance FROM coude_cashbox WHERE guild_id = $1 FOR UPDATE")
                .bind(guild_id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(pg_err)?;

        let balance = row.map(|(v,)| v).unwrap_or(0);
        if balance <= 0 {
            tx.commit().await.map_err(pg_err)?;
            return Ok(0);
        }

        let actual = amount.min(balance);

        sqlx::query(
            r#"UPDATE coude_cashbox
               SET balance = balance - $2,
                   updated_at = NOW()
               WHERE guild_id = $1"#,
        )
        .bind(guild_id)
        .bind(actual)
        .execute(&mut *tx)
        .await
        .map_err(pg_err)?;

        tx.commit().await.map_err(pg_err)?;
        Ok(actual)
    }

    async fn claim_all_for_redistribution(&self, guild_id: &str) -> Result<i64, DomainError> {
        // Transaction : SELECT FOR UPDATE puis UPDATE. Garantit l'atomicite
        // entre le read de balance et le reset (empeche une double redistribution
        // si 2 workers tournent en parallele).
        let mut tx = self.pool.begin().await.map_err(pg_err)?;

        let row: Option<(i64,)> =
            sqlx::query_as("SELECT balance FROM coude_cashbox WHERE guild_id = $1 FOR UPDATE")
                .bind(guild_id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(pg_err)?;

        let balance = row.map(|(v,)| v).unwrap_or(0);
        if balance <= 0 {
            tx.commit().await.map_err(pg_err)?;
            return Ok(0);
        }

        sqlx::query(
            r#"UPDATE coude_cashbox
               SET balance = 0,
                   total_redistributed = total_redistributed + $2,
                   last_redistribution_at = NOW(),
                   updated_at = NOW()
               WHERE guild_id = $1"#,
        )
        .bind(guild_id)
        .bind(balance)
        .execute(&mut *tx)
        .await
        .map_err(pg_err)?;

        tx.commit().await.map_err(pg_err)?;
        Ok(balance)
    }

    async fn record_redistribution(
        &self,
        guild_id: &str,
        total_amount: i64,
        entries: Vec<(String, String, i64)>,
    ) -> Result<Uuid, DomainError> {
        let mut tx = self.pool.begin().await.map_err(pg_err)?;

        let winners_count = i32::try_from(entries.len())
            .map_err(|_| DomainError::ValidationError("Trop de gagnants".into()))?;

        let row: (Uuid,) = sqlx::query_as(
            r#"INSERT INTO coude_cashbox_redistributions
                 (guild_id, total_amount, winners_count)
               VALUES ($1, $2, $3)
               RETURNING id"#,
        )
        .bind(guild_id)
        .bind(total_amount)
        .bind(winners_count)
        .fetch_one(&mut *tx)
        .await
        .map_err(pg_err)?;

        let redistribution_id = row.0;

        // Batch insert via UNNEST — une seule requete au lieu de N.
        if !entries.is_empty() {
            let (user_ids, usernames, amounts): (Vec<_>, Vec<_>, Vec<_>) = entries
                .iter()
                .map(|(u, n, a)| (u.as_str(), n.as_str(), *a))
                .fold(
                    (Vec::new(), Vec::new(), Vec::new()),
                    |mut acc, (u, n, a)| {
                        acc.0.push(u);
                        acc.1.push(n);
                        acc.2.push(a);
                        acc
                    },
                );
            sqlx::query(
                r#"INSERT INTO coude_cashbox_redistribution_entries
                     (redistribution_id, user_id, username, amount_won)
                   SELECT $1, u.user_id, u.username, u.amount_won
                   FROM UNNEST($2::text[], $3::text[], $4::bigint[])
                        AS u(user_id, username, amount_won)"#,
            )
            .bind(redistribution_id)
            .bind(&user_ids)
            .bind(&usernames)
            .bind(&amounts)
            .execute(&mut *tx)
            .await
            .map_err(pg_err)?;
        }

        tx.commit().await.map_err(pg_err)?;
        Ok(redistribution_id)
    }

    async fn list_redistributions(
        &self,
        guild_id: &str,
        limit: i64,
    ) -> Result<Vec<CashboxRedistribution>, DomainError> {
        let rows: Vec<RedistributionRow> = sqlx::query_as(
            r#"SELECT id, guild_id, total_amount, winners_count, created_at
               FROM coude_cashbox_redistributions
               WHERE guild_id = $1
               ORDER BY created_at DESC
               LIMIT $2"#,
        )
        .bind(guild_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn list_entries(
        &self,
        redistribution_id: Uuid,
    ) -> Result<Vec<CashboxRedistributionEntry>, DomainError> {
        let rows: Vec<EntryRow> = sqlx::query_as(
            r#"SELECT id, redistribution_id, user_id, username, amount_won, created_at
               FROM coude_cashbox_redistribution_entries
               WHERE redistribution_id = $1
               ORDER BY amount_won DESC"#,
        )
        .bind(redistribution_id)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn list_active_players(
        &self,
        guild_id: &str,
        days: i64,
    ) -> Result<Vec<(String, String)>, DomainError> {
        // Un joueur est actif si son `updated_at` sur `coude_players` est
        // dans la fenetre ET qu'il a au moins 1 combat joue (wins + losses
        // + draws > 0) — evite d'inclure les lurkers qui ont juste consulte
        // leur profil.
        let rows: Vec<(String, String)> = sqlx::query_as(
            r#"SELECT user_id, username
               FROM coude_players
               WHERE guild_id = $1
                 AND updated_at > NOW() - ($2::bigint * INTERVAL '1 day')
                 AND (total_wins + total_losses + total_draws) > 0"#,
        )
        .bind(guild_id)
        .bind(days)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(rows)
    }

    async fn list_guilds_due_for_redistribution(
        &self,
        min_days_since_last: i64,
    ) -> Result<Vec<String>, DomainError> {
        // Caisse non vide ET (jamais redistribuee OU dernier passage > min_days).
        let rows: Vec<(String,)> = sqlx::query_as(
            r#"SELECT guild_id
               FROM coude_cashbox
               WHERE balance > 0
                 AND (
                     last_redistribution_at IS NULL
                     OR last_redistribution_at < NOW() - ($1::bigint * INTERVAL '1 day')
                 )"#,
        )
        .bind(min_days_since_last)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(rows.into_iter().map(|(g,)| g).collect())
    }
}
