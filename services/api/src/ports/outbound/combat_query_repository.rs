//! Port narrow lecture-seule sur les combats Coup de Coude.
//!
//! Extrait depuis `CoudeCombatRepository` (P0 #2 de l'audit architecture)
//! pour permettre aux use cases qui ne font QUE lire un combat (ex.
//! `ManageCoudeBetsService` -> `place`/`resolve`/`refund`) de declarer une
//! dependance minimale, plutot que :
//!  - le port complet (~30 methodes ecriture incluse), ou
//!  - un autre use case (`ManageCoudeCombatsUseCase`) — ce qui creait un
//!    couplage use-case-to-use-case interdit par l'architecture
//!    hexagonale.
//!
//! Convention : `get` renvoie `NotFound` (et pas `Option`) car les call sites
//! traitaient deja le `None` -> `NotFound("Combat introuvable")` manuellement.

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::entities::CoudeCombat;
use crate::domain::errors::DomainError;

#[async_trait]
pub trait CombatQueryRepository: Send + Sync {
    /// Charge un combat par son id. `NotFound` si absent.
    async fn get(&self, id: Uuid) -> Result<CoudeCombat, DomainError>;
}
