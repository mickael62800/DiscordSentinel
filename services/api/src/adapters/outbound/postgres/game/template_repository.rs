use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::domain::entities::game::template::{ConfigField, GameTemplate};
use crate::domain::errors::DomainError;
use crate::ports::outbound::game::game_template_repository::GameTemplateRepository;

pub struct PgGameTemplateRepository {
    pool: PgPool,
}

impl PgGameTemplateRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(FromRow)]
struct TemplateRow {
    id: Uuid,
    slug: String,
    name: String,
    description: Option<String>,
    image: String,
    category: Option<String>,
    icon: Option<String>,
    accent_color: Option<String>,
    container_port: i32,
    default_memory_mb: i32,
    min_memory_mb: i32,
    max_memory_mb: i32,
    default_env: serde_json::Value,
    config_schema: serde_json::Value,
    supports_rcon: bool,
    supports_mods: bool,
    idle_shutdown_days: i32,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl TryFrom<TemplateRow> for GameTemplate {
    type Error = DomainError;
    fn try_from(r: TemplateRow) -> Result<Self, DomainError> {
        let config_schema: Vec<ConfigField> = serde_json::from_value(r.config_schema)
            .map_err(|e| DomainError::Internal(format!("config_schema parse: {e}")))?;
        let port = u16::try_from(r.container_port)
            .map_err(|_| DomainError::Internal("container_port hors range u16".into()))?;
        Ok(GameTemplate {
            id: r.id,
            slug: r.slug,
            name: r.name,
            description: r.description,
            image: r.image,
            category: r.category,
            icon: r.icon,
            accent_color: r.accent_color,
            container_port: port,
            default_memory_mb: r.default_memory_mb,
            min_memory_mb: r.min_memory_mb,
            max_memory_mb: r.max_memory_mb,
            default_env: r.default_env,
            config_schema,
            supports_rcon: r.supports_rcon,
            supports_mods: r.supports_mods,
            idle_shutdown_days: r.idle_shutdown_days,
            created_at: r.created_at,
            updated_at: r.updated_at,
        })
    }
}

const SELECT_COLS: &str =
    "id, slug, name, description, image, category, icon, accent_color, \
     container_port, default_memory_mb, min_memory_mb, max_memory_mb, \
     default_env, config_schema, supports_rcon, supports_mods, idle_shutdown_days, \
     created_at, updated_at";

#[async_trait]
impl GameTemplateRepository for PgGameTemplateRepository {
    async fn list(&self) -> Result<Vec<GameTemplate>, DomainError> {
        let rows: Vec<TemplateRow> = sqlx::query_as(&format!(
            "SELECT {SELECT_COLS} FROM game_templates \
             WHERE deleted_at IS NULL ORDER BY name"
        ))
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(format!("list templates: {e}")))?;
        rows.into_iter().map(GameTemplate::try_from).collect()
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<GameTemplate>, DomainError> {
        let row: Option<TemplateRow> = sqlx::query_as(&format!(
            "SELECT {SELECT_COLS} FROM game_templates \
             WHERE id = $1 AND deleted_at IS NULL"
        ))
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(format!("find template by id: {e}")))?;
        row.map(GameTemplate::try_from).transpose()
    }

    async fn find_by_slug(&self, slug: &str) -> Result<Option<GameTemplate>, DomainError> {
        let row: Option<TemplateRow> = sqlx::query_as(&format!(
            "SELECT {SELECT_COLS} FROM game_templates \
             WHERE slug = $1 AND deleted_at IS NULL"
        ))
        .bind(slug)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(format!("find template by slug: {e}")))?;
        row.map(GameTemplate::try_from).transpose()
    }
}
