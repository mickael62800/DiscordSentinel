use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::entities::BlackjackGame;
use crate::domain::errors::DomainError;

#[async_trait]
pub trait BlackjackRepository: Send + Sync {
    async fn create(&self, game: &BlackjackGame) -> Result<(), DomainError>;
    async fn get_active(&self, guild_id: &str, user_id: &str) -> Result<Option<BlackjackGame>, DomainError>;
    async fn update(&self, game: &BlackjackGame) -> Result<(), DomainError>;
    async fn get_by_id(&self, id: Uuid) -> Result<Option<BlackjackGame>, DomainError>;

    /// Liste les parties d'un serveur, optionnellement filtrees par `status`.
    /// Utilise par la page Blackjack du desktop.
    async fn list_by_guild(&self, guild_id: &str, status: Option<&str>) -> Result<Vec<BlackjackGame>, DomainError>;

    /// Annule une partie en cours : marque `status = cancelled` + rembourse
    /// la mise au joueur via le wallet. Renvoie le solde rembourse.
    /// Erreur 409 si la partie est deja terminee.
    async fn cancel_game(&self, id: Uuid) -> Result<(), DomainError>;
}
