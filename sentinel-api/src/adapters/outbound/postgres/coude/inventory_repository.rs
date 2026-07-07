use async_trait::async_trait;
use chrono::DateTime;
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use sentinel_core::domain::entities::coude::inventory::Insurance;
use sentinel_core::domain::entities::coude::inventory::InventoryItem;
use sentinel_core::domain::entities::coude::inventory::NewCoudePrime;
use sentinel_core::domain::entities::coude::inventory::Prime;
use sentinel_core::domain::errors::DomainError;

use super::super::pg_err;
use crate::ports::outbound::coude::inventory_repository::InventoryRepository;
use crate::ports::outbound::coude::inventory_repository::UsePotionTxOutcome;

pub struct PgInventoryRepository {
    pool: PgPool,
}

impl PgInventoryRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct InventoryRow {
    guild_id: String,
    user_id: String,
    item_key: String,
    quantity: i32,
}

impl From<InventoryRow> for InventoryItem {
    fn from(r: InventoryRow) -> Self {
        Self {
            guild_id: r.guild_id.into(),
            user_id: r.user_id.into(),
            item_key: r.item_key,
            quantity: r.quantity,
        }
    }
}

#[derive(sqlx::FromRow)]
struct PrimeRow {
    id: Uuid,
    guild_id: String,
    target_id: String,
    target_name: String,
    placed_by_id: String,
    placed_by_name: String,
    amount: i64,
    claimed: bool,
    claimed_by_id: Option<String>,
    claimed_by_name: Option<String>,
    claimed_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

impl From<PrimeRow> for Prime {
    fn from(r: PrimeRow) -> Self {
        Self {
            id: r.id,
            guild_id: r.guild_id.into(),
            target_id: r.target_id,
            target_name: r.target_name,
            placed_by_id: r.placed_by_id,
            placed_by_name: r.placed_by_name,
            amount: r.amount,
            claimed: r.claimed,
            claimed_by_id: r.claimed_by_id,
            claimed_by_name: r.claimed_by_name,
            claimed_at: r.claimed_at,
            created_at: r.created_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct InsuranceRow {
    id: Uuid,
    is_scam: bool,
    expires_at: DateTime<Utc>,
}

impl From<InsuranceRow> for Insurance {
    fn from(r: InsuranceRow) -> Self {
        Self {
            id: r.id,
            is_scam: r.is_scam,
            expires_at: r.expires_at,
        }
    }
}

#[async_trait]
impl InventoryRepository for PgInventoryRepository {
    // ── Items ──

    async fn list_inventory(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Vec<InventoryItem>, DomainError> {
        let rows: Vec<InventoryRow> = sqlx::query_as(
            "SELECT guild_id, user_id, item_key, quantity FROM coude_inventory
             WHERE guild_id = $1 AND user_id = $2 AND quantity > 0",
        )
        .bind(guild_id)
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn add_item(
        &self,
        guild_id: &str,
        user_id: &str,
        item_key: &str,
    ) -> Result<(), DomainError> {
        sqlx::query(
            r#"INSERT INTO coude_inventory (guild_id, user_id, item_key, quantity)
               VALUES ($1, $2, $3, 1)
               ON CONFLICT (guild_id, user_id, item_key)
               DO UPDATE SET quantity = coude_inventory.quantity + 1"#,
        )
        .bind(guild_id)
        .bind(user_id)
        .bind(item_key)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(())
    }

    async fn use_item(
        &self,
        guild_id: &str,
        user_id: &str,
        item_key: &str,
    ) -> Result<bool, DomainError> {
        let result = sqlx::query(
            r#"UPDATE coude_inventory SET quantity = quantity - 1
               WHERE guild_id = $1 AND user_id = $2 AND item_key = $3 AND quantity > 0"#,
        )
        .bind(guild_id)
        .bind(user_id)
        .bind(item_key)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(result.rows_affected() > 0)
    }

    async fn use_potion_atomic(
        &self,
        guild_id: &str,
        user_id: &str,
        item_key: &str,
        heal_amount: i32,
    ) -> Result<UsePotionTxOutcome, DomainError> {
        let mut tx = self.pool.begin().await.map_err(pg_err)?;

        // Verrouille la ligne joueur pour lire l'etat HP courant.
        let player: Option<(i32, i32)> = sqlx::query_as(
            "SELECT hp_current, hp_max FROM coude_players
             WHERE guild_id = $1 AND user_id = $2 FOR UPDATE",
        )
        .bind(guild_id)
        .bind(user_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(pg_err)?;

        let Some((hp_current, hp_max)) = player else {
            tx.rollback().await.map_err(pg_err)?;
            return Ok(UsePotionTxOutcome::NoItem);
        };

        if hp_current >= hp_max {
            tx.rollback().await.map_err(pg_err)?;
            return Ok(UsePotionTxOutcome::AlreadyFull);
        }

        // Consomme l'item (atomique avec le heal).
        let consumed = sqlx::query(
            r#"UPDATE coude_inventory SET quantity = quantity - 1
               WHERE guild_id = $1 AND user_id = $2 AND item_key = $3 AND quantity > 0"#,
        )
        .bind(guild_id)
        .bind(user_id)
        .bind(item_key)
        .execute(&mut *tx)
        .await
        .map_err(pg_err)?;
        if consumed.rows_affected() == 0 {
            tx.rollback().await.map_err(pg_err)?;
            return Ok(UsePotionTxOutcome::NoItem);
        }

        let new_hp = (hp_current + heal_amount).min(hp_max);
        sqlx::query(
            "UPDATE coude_players SET hp_current = $3, updated_at = NOW()
             WHERE guild_id = $1 AND user_id = $2",
        )
        .bind(guild_id)
        .bind(user_id)
        .bind(new_hp)
        .execute(&mut *tx)
        .await
        .map_err(pg_err)?;

        tx.commit().await.map_err(pg_err)?;
        Ok(UsePotionTxOutcome::Healed {
            actually_healed: new_hp - hp_current,
            new_hp,
            hp_max,
        })
    }

    async fn has_item(
        &self,
        guild_id: &str,
        user_id: &str,
        item_key: &str,
    ) -> Result<bool, DomainError> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM coude_inventory
             WHERE guild_id = $1 AND user_id = $2 AND item_key = $3 AND quantity > 0",
        )
        .bind(guild_id)
        .bind(user_id)
        .bind(item_key)
        .fetch_one(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(row.0 > 0)
    }

    // ── Primes ──

    async fn create_prime(&self, new: NewCoudePrime) -> Result<Prime, DomainError> {
        let row: PrimeRow = sqlx::query_as(
            r#"INSERT INTO coude_primes
                 (guild_id, target_id, target_name, placed_by_id, placed_by_name, amount)
               VALUES ($1, $2, $3, $4, $5, $6)
               RETURNING id, guild_id, target_id, target_name, placed_by_id, placed_by_name,
                         amount, claimed, claimed_by_id, claimed_by_name, claimed_at, created_at"#,
        )
        .bind(new.guild_id.as_str())
        .bind(new.target_id.as_str())
        .bind(&new.target_name)
        .bind(&new.placed_by_id)
        .bind(&new.placed_by_name)
        .bind(new.amount)
        .fetch_one(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(row.into())
    }

    async fn list_active_primes(
        &self,
        guild_id: &str,
        target_id: &str,
    ) -> Result<Vec<Prime>, DomainError> {
        let rows: Vec<PrimeRow> = sqlx::query_as(
            r#"SELECT id, guild_id, target_id, target_name, placed_by_id, placed_by_name,
                      amount, claimed, claimed_by_id, claimed_by_name, claimed_at, created_at
               FROM coude_primes
               WHERE guild_id = $1 AND target_id = $2 AND claimed = FALSE"#,
        )
        .bind(guild_id)
        .bind(target_id)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn claim_primes(
        &self,
        guild_id: &str,
        target_id: &str,
        claimer_id: &str,
        claimer_name: &str,
    ) -> Result<i64, DomainError> {
        let row: (Option<i64>,) = sqlx::query_as(
            r#"WITH claimed AS (
                UPDATE coude_primes
                SET claimed = TRUE,
                    claimed_by_id = $3,
                    claimed_by_name = $4,
                    claimed_at = NOW()
                WHERE guild_id = $1 AND target_id = $2 AND claimed = FALSE
                RETURNING amount
            )
            SELECT COALESCE(SUM(amount), 0)::BIGINT FROM claimed"#,
        )
        .bind(guild_id)
        .bind(target_id)
        .bind(claimer_id)
        .bind(claimer_name)
        .fetch_one(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(row.0.unwrap_or(0))
    }

    // ── Assurances ──

    async fn buy_insurance(
        &self,
        guild_id: &str,
        user_id: &str,
        is_scam: bool,
        duration_seconds: i64,
    ) -> Result<bool, DomainError> {
        // 0 = defaut historique 1h (3600s) pour retrocompat.
        let secs = if duration_seconds <= 0 {
            3600
        } else {
            duration_seconds
        };
        // Atomique : INSERT uniquement si aucune assurance active. Evite la
        // race ou deux /assurance simultanees passent le check get_active et
        // creent chacune une ligne (coins perdus pour la 2e).
        let inserted = sqlx::query_scalar::<_, Uuid>(
            r#"INSERT INTO coude_insurances (guild_id, user_id, is_scam, expires_at)
               SELECT $1, $2, $3, NOW() + make_interval(secs => $4)
               WHERE NOT EXISTS (
                   SELECT 1 FROM coude_insurances
                    WHERE guild_id = $1 AND user_id = $2
                      AND active = TRUE AND expires_at > NOW()
               )
               RETURNING id"#,
        )
        .bind(guild_id)
        .bind(user_id)
        .bind(is_scam)
        .bind(secs as f64)
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(inserted.is_some())
    }

    async fn buy_insurance_with_max_slots(
        &self,
        guild_id: &str,
        user_id: &str,
        is_scam: bool,
        duration_seconds: i64,
        max_slots: i32,
    ) -> Result<bool, DomainError> {
        let secs = if duration_seconds <= 0 {
            3600
        } else {
            duration_seconds
        };
        let max = max_slots.max(1);
        // Insert seulement si le nombre d assurances actives < max_slots.
        let inserted = sqlx::query_scalar::<_, Uuid>(
            r#"INSERT INTO coude_insurances (guild_id, user_id, is_scam, expires_at)
               SELECT $1, $2, $3, NOW() + make_interval(secs => $4)
               WHERE (
                   SELECT COUNT(*) FROM coude_insurances
                    WHERE guild_id = $1 AND user_id = $2
                      AND active = TRUE AND expires_at > NOW()
               ) < $5
               RETURNING id"#,
        )
        .bind(guild_id)
        .bind(user_id)
        .bind(is_scam)
        .bind(secs as f64)
        .bind(max)
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(inserted.is_some())
    }

    async fn get_active_insurance(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<Insurance>, DomainError> {
        let row: Option<InsuranceRow> = sqlx::query_as(
            r#"SELECT id, is_scam, expires_at
               FROM coude_insurances
               WHERE guild_id = $1 AND user_id = $2
                 AND active = TRUE AND expires_at > NOW()
               ORDER BY expires_at DESC
               LIMIT 1"#,
        )
        .bind(guild_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(row.map(Into::into))
    }

    async fn expire_insurance(&self, insurance_id: Uuid) -> Result<bool, DomainError> {
        let result = sqlx::query(
            "UPDATE coude_insurances SET active = FALSE, expires_at = NOW() WHERE id = $1",
        )
        .bind(insurance_id)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(result.rows_affected() > 0)
    }
}
