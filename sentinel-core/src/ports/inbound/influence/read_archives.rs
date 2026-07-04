//! Use case : consultation de la memoire du serveur (archives / actu).

use async_trait::async_trait;

use crate::domain::entities::influence::archive::ArchiveEntry;
use crate::domain::errors::DomainError;

#[async_trait]
pub trait ReadArchivesUseCase: Send + Sync {
    /// Dernieres entrees de la memoire du serveur. `limit` None = taille config.
    async fn list(
        &self,
        guild_id: &str,
        limit: Option<i64>,
    ) -> Result<Vec<ArchiveEntry>, DomainError>;
}
