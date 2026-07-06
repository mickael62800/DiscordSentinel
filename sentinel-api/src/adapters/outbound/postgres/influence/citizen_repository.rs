//! Impl Postgres de `CitizenRepository` (cf. coude player_repository).

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use sentinel_core::domain::entities::influence::capital::{Capital, Capitals};
use sentinel_core::domain::entities::influence::citizen::Citizen;
use sentinel_core::domain::errors::DomainError;
use sentinel_core::ports::outbound::influence::citizen_repository::CitizenRepository;

use super::super::pg_err_ctx;

const TBL: &str = "influence_citizens";
fn pg_err(e: sqlx::Error) -> DomainError {
    pg_err_ctx(TBL, e)
}

pub struct PgCitizenRepository {
    pool: PgPool,
}

impl PgCitizenRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct Row {
    id: Uuid,
    guild_id: String,
    user_id: String,
    username: String,
    influence: i64,
    money: i64,
    reputation: i64,
    information: i64,
    network: i64,
    joined_at: DateTime<Utc>,
}

impl From<Row> for Citizen {
    fn from(r: Row) -> Self {
        Self {
            id: r.id,
            guild_id: r.guild_id,
            user_id: r.user_id,
            username: r.username,
            capitals: Capitals {
                influence: r.influence,
                money: r.money,
                reputation: r.reputation,
                information: r.information,
                network: r.network,
            },
            joined_at: r.joined_at,
        }
    }
}

const SELECT_COLS: &str = "id, guild_id, user_id, username, influence, money, \
    reputation, information, network, joined_at";

#[async_trait]
impl CitizenRepository for PgCitizenRepository {
    async fn get_or_create(
        &self,
        guild_id: &str,
        user_id: &str,
        username: &str,
        start_money: i64,
    ) -> Result<Citizen, DomainError> {
        // Upsert idempotent : cree a la volee avec l'argent de depart, ou met a
        // jour le username si le citoyen existe deja.
        let sql = format!(
            "INSERT INTO influence_citizens (guild_id, user_id, username, money) \
             VALUES ($1, $2, $3, $4) \
             ON CONFLICT (guild_id, user_id) DO UPDATE \
               SET username = EXCLUDED.username, updated_at = NOW() \
             RETURNING {SELECT_COLS}"
        );
        let row: Row = sqlx::query_as(&sql)
            .bind(guild_id)
            .bind(user_id)
            .bind(username)
            .bind(start_money)
            .fetch_one(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(row.into())
    }

    async fn get(&self, guild_id: &str, user_id: &str) -> Result<Option<Citizen>, DomainError> {
        let sql = format!(
            "SELECT {SELECT_COLS} FROM influence_citizens \
             WHERE guild_id = $1 AND user_id = $2"
        );
        let row: Option<Row> = sqlx::query_as(&sql)
            .bind(guild_id)
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(row.map(Into::into))
    }

    async fn adjust_money(&self, citizen_id: Uuid, delta: i64) -> Result<i64, DomainError> {
        // Garde atomique : l'Argent ne descend jamais sous 0. Un credit (delta>0)
        // passe toujours ; un debit concurrent qui rendrait le solde negatif
        // n'affecte aucune ligne -> "Solde insuffisant" (evite le solde negatif
        // que laissait un check lu-puis-ecrit non atomique).
        let new_value: Option<i64> = sqlx::query_scalar(
            "UPDATE influence_citizens SET money = money + $2, updated_at = NOW() \
             WHERE id = $1 AND money + $2 >= 0 RETURNING money",
        )
        .bind(citizen_id)
        .bind(delta)
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_err)?;
        new_value.ok_or_else(|| DomainError::Forbidden("Solde insuffisant.".into()))
    }

    async fn adjust_capital(
        &self,
        citizen_id: Uuid,
        capital: Capital,
        delta: i64,
    ) -> Result<i64, DomainError> {
        // Le nom de colonne vient d'un enum ferme (jamais une entree externe) :
        // pas d'injection possible.
        let col = capital.as_str();
        let sql = format!(
            "UPDATE influence_citizens SET {col} = {col} + $2, updated_at = NOW() \
             WHERE id = $1 RETURNING {col}"
        );
        let new_value: i64 = sqlx::query_scalar(&sql)
            .bind(citizen_id)
            .bind(delta)
            .fetch_one(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(new_value)
    }
}
