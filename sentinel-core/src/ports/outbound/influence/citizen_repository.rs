//! Port outbound : persistance des citoyens (cf. player_repository coude).

use async_trait::async_trait;

use crate::domain::entities::influence::citizen::Citizen;
use crate::domain::errors::DomainError;

#[async_trait]
pub trait CitizenRepository: Send + Sync {
    /// Recupere le citoyen, en le creant a la volee s'il n'existe pas encore
    /// (enregistrement citoyen automatique, cf. ARCHITECTURE.md Phase 1).
    /// `start_money` = capital Argent initial d'un nouveau citoyen.
    async fn get_or_create(
        &self,
        guild_id: &str,
        user_id: &str,
        username: &str,
        start_money: i64,
    ) -> Result<Citizen, DomainError>;

    /// Recupere le citoyen s'il existe (sans le creer).
    async fn get(&self, guild_id: &str, user_id: &str) -> Result<Option<Citizen>, DomainError>;
}
