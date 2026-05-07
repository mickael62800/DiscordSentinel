//! Port outbound pour le catalogue `coude_flavor_templates`
//! (Phase 3 #9 audit).

use async_trait::async_trait;

use sentinel_core::domain::errors::DomainError;

#[async_trait]
pub trait FlavorTemplatesRepository: Send + Sync {
    /// Tire un template aleatoire pour `(key, locale)`. Renvoie `None` si
    /// aucun template ne matche (le bot fallback sur ses arrays locales).
    /// Le tirage est pondere par le champ `weight` cote DB.
    async fn random_by_key(
        &self,
        key: &str,
        locale: &str,
    ) -> Result<Option<String>, DomainError>;
}
