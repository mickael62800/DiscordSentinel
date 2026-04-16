use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::errors::DomainError;

#[derive(Debug, Clone, serde::Serialize)]
pub struct TempRole {
    pub id: Uuid,
    pub guild_id: String,
    pub user_id: String,
    pub role_id: String,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[async_trait]
pub trait TempRoleRepository: Send + Sync {
    async fn create(&self, guild_id: &str, user_id: &str, role_id: &str, expires_at: &str) -> Result<(), DomainError>;
    async fn list_active(&self, guild_id: &str) -> Result<Vec<TempRole>, DomainError>;
    async fn delete(&self, guild_id: &str, user_id: &str, role_id: &str) -> Result<(), DomainError>;
}
