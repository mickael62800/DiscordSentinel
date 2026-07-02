use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use super::super::pg_err_ctx;
use crate::ports::outbound::tamagotchi::pet_repository::PetRepository;
use sentinel_core::domain::entities::tamagotchi::pet::{Health, NewPet, Pet, PetEvent};
use sentinel_core::domain::errors::DomainError;

const TBL: &str = "pets";
fn pg_err(e: sqlx::Error) -> DomainError {
    pg_err_ctx(TBL, e)
}

#[derive(sqlx::FromRow)]
struct Row {
    id: Uuid,
    guild_id: String,
    owner_id: String,
    name: String,
    species: String,
    specialization: Option<String>,
    level: i32,
    xp: i64,
    born_at: DateTime<Utc>,
    hunger: i32,
    happiness: i32,
    energy: i32,
    status: String,
    hunger_zero_since: Option<DateTime<Utc>>,
    sick_since: Option<DateTime<Utc>>,
    died_at: Option<DateTime<Utc>>,
    str: i32,
    vit: i32,
    agi: i32,
    stat_points: i32,
    elo: i32,
    wins: i32,
    losses: i32,
    cooldowns: serde_json::Value,
    last_decay_at: DateTime<Utc>,
    card_channel_id: Option<String>,
    card_message_id: Option<String>,
}

impl From<Row> for Pet {
    fn from(r: Row) -> Self {
        Pet {
            id: r.id,
            guild_id: r.guild_id,
            owner_id: r.owner_id,
            name: r.name,
            species: r.species,
            specialization: r.specialization,
            level: r.level,
            xp: r.xp,
            born_at: r.born_at,
            hunger: r.hunger,
            happiness: r.happiness,
            energy: r.energy,
            status: Health::from_str(&r.status),
            hunger_zero_since: r.hunger_zero_since,
            sick_since: r.sick_since,
            died_at: r.died_at,
            str_: r.str,
            vit: r.vit,
            agi: r.agi,
            stat_points: r.stat_points,
            elo: r.elo,
            wins: r.wins,
            losses: r.losses,
            cooldowns: r.cooldowns,
            last_decay_at: r.last_decay_at,
            card_channel_id: r.card_channel_id,
            card_message_id: r.card_message_id,
        }
    }
}

pub struct PgPetRepository {
    pool: PgPool,
}

impl PgPetRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl PetRepository for PgPetRepository {
    async fn create(&self, p: NewPet) -> Result<Pet, DomainError> {
        // INSERT atomique : ON CONFLICT DO NOTHING garantit qu'un doublon
        // (guild_id, owner_id) ne peut pas etre cree, meme en cas de course
        // (la correctness ne depend plus uniquement de la contrainte UNIQUE
        // cote schema). Si le conflit se declenche, aucune ligne n'est
        // retournee : on renvoie alors le meme Conflict que le service.
        let row: Option<Row> = sqlx::query_as(
            "INSERT INTO pets (guild_id, owner_id, name, species, str, vit, agi) \
             VALUES ($1,$2,$3,$4,$5,$6,$7) \
             ON CONFLICT (guild_id, owner_id) DO NOTHING RETURNING *",
        )
        .bind(&p.guild_id)
        .bind(&p.owner_id)
        .bind(&p.name)
        .bind(&p.species)
        .bind(p.str_)
        .bind(p.vit)
        .bind(p.agi)
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_err)?;
        match row {
            Some(r) => Ok(r.into()),
            None => Err(DomainError::Conflict("tu as deja un compagnon".into())),
        }
    }

    async fn get(&self, id: Uuid) -> Result<Option<Pet>, DomainError> {
        let row: Option<Row> = sqlx::query_as("SELECT * FROM pets WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(row.map(Into::into))
    }

    async fn get_by_owner(
        &self,
        guild_id: &str,
        owner_id: &str,
    ) -> Result<Option<Pet>, DomainError> {
        let row: Option<Row> =
            sqlx::query_as("SELECT * FROM pets WHERE guild_id = $1 AND owner_id = $2")
                .bind(guild_id)
                .bind(owner_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(pg_err)?;
        Ok(row.map(Into::into))
    }

    async fn list_by_guild(&self, guild_id: &str) -> Result<Vec<Pet>, DomainError> {
        let rows: Vec<Row> =
            sqlx::query_as("SELECT * FROM pets WHERE guild_id = $1 ORDER BY level DESC, xp DESC")
                .bind(guild_id)
                .fetch_all(&self.pool)
                .await
                .map_err(pg_err)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn delete(&self, id: Uuid) -> Result<(), DomainError> {
        sqlx::query("DELETE FROM pets WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(())
    }

    async fn save(&self, p: &Pet) -> Result<Pet, DomainError> {
        let row: Row = sqlx::query_as(
            "UPDATE pets SET \
                name = $2, level = $3, xp = $4, hunger = $5, happiness = $6, energy = $7, \
                status = $8, hunger_zero_since = $9, sick_since = $10, died_at = $11, \
                str = $12, vit = $13, agi = $14, stat_points = $15, elo = $16, wins = $17, \
                losses = $18, cooldowns = $19, last_decay_at = $20, updated_at = NOW() \
             WHERE id = $1 RETURNING *",
        )
        .bind(p.id)
        .bind(&p.name)
        .bind(p.level)
        .bind(p.xp)
        .bind(p.hunger)
        .bind(p.happiness)
        .bind(p.energy)
        .bind(p.status.as_str())
        .bind(p.hunger_zero_since)
        .bind(p.sick_since)
        .bind(p.died_at)
        .bind(p.str_)
        .bind(p.vit)
        .bind(p.agi)
        .bind(p.stat_points)
        .bind(p.elo)
        .bind(p.wins)
        .bind(p.losses)
        .bind(&p.cooldowns)
        .bind(p.last_decay_at)
        .fetch_one(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(row.into())
    }

    async fn list_alive(
        &self,
        limit: i64,
        after_id: Option<Uuid>,
    ) -> Result<Vec<Pet>, DomainError> {
        // Tri par `id` (et non `last_decay_at`) : stable pour la pagination par
        // curseur meme si le tick met a jour `last_decay_at` ou tue des pets.
        let rows: Vec<Row> = sqlx::query_as(
            "SELECT * FROM pets \
             WHERE status <> 'dead' AND ($2::uuid IS NULL OR id > $2) \
             ORDER BY id ASC LIMIT $1",
        )
        .bind(limit)
        .bind(after_id)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn set_card_location(
        &self,
        guild_id: &str,
        owner_id: &str,
        channel_id: &str,
        message_id: &str,
    ) -> Result<(), DomainError> {
        sqlx::query(
            "UPDATE pets SET card_channel_id = $3, card_message_id = $4 \
             WHERE guild_id = $1 AND owner_id = $2",
        )
        .bind(guild_id)
        .bind(owner_id)
        .bind(channel_id)
        .bind(message_id)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(())
    }

    async fn list_with_card(
        &self,
        limit: i64,
        after_id: Option<Uuid>,
    ) -> Result<Vec<Pet>, DomainError> {
        // Compagnons vivants ayant une carte postee (a rafraichir), pagine par
        // curseur `id` croissant.
        let rows: Vec<Row> = sqlx::query_as(
            "SELECT * FROM pets \
             WHERE status <> 'dead' AND card_message_id IS NOT NULL \
               AND ($2::uuid IS NULL OR id > $2) \
             ORDER BY id ASC LIMIT $1",
        )
        .bind(limit)
        .bind(after_id)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn list_dead_with_channel(
        &self,
        limit: i64,
        after_id: Option<Uuid>,
    ) -> Result<Vec<Pet>, DomainError> {
        // Compagnons morts dont le salon prive existe encore : cible de la
        // reconciliation (fermeture des salons orphelins). Pagine par curseur
        // `id` croissant, comme `list_alive`.
        let rows: Vec<Row> = sqlx::query_as(
            "SELECT * FROM pets \
             WHERE status = 'dead' AND card_channel_id IS NOT NULL \
               AND ($2::uuid IS NULL OR id > $2) \
             ORDER BY id ASC LIMIT $1",
        )
        .bind(limit)
        .bind(after_id)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn clear_card_location(&self, pet_id: Uuid) -> Result<(), DomainError> {
        sqlx::query("UPDATE pets SET card_channel_id = NULL, card_message_id = NULL WHERE id = $1")
            .bind(pet_id)
            .execute(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(())
    }

    async fn add_event(&self, pet_id: Uuid, kind: &str, detail: &str) -> Result<(), DomainError> {
        sqlx::query("INSERT INTO pet_events (pet_id, kind, detail) VALUES ($1,$2,$3)")
            .bind(pet_id)
            .bind(kind)
            .bind(detail)
            .execute(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(())
    }

    async fn recent_events(&self, pet_id: Uuid, limit: i64) -> Result<Vec<PetEvent>, DomainError> {
        #[derive(sqlx::FromRow)]
        struct EvRow {
            kind: String,
            detail: String,
            created_at: DateTime<Utc>,
        }
        let rows: Vec<EvRow> = sqlx::query_as(
            "SELECT kind, detail, created_at FROM pet_events \
             WHERE pet_id = $1 ORDER BY created_at DESC LIMIT $2",
        )
        .bind(pet_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(rows
            .into_iter()
            .map(|e| PetEvent {
                kind: e.kind,
                detail: e.detail,
                created_at: e.created_at,
            })
            .collect())
    }
}
