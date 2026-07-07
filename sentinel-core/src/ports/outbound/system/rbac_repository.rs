//! Port outbound : persistance du RBAC applicatif (`api_users`,
//! `api_user_guilds`). Tout le SQL vit dans l'adapter Postgres.

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::domain::entities::system::rbac::GuildUserEntry;
use crate::domain::errors::DomainError;

#[async_trait]
pub trait RbacRepository: Send + Sync {
    /// Upsert de la ligne `api_users` (garantit la FK avant l'attribution).
    /// Ne touche pas au display_name existant (ON CONFLICT DO NOTHING).
    async fn upsert_user(&self, user_id: &str, display_name: &str) -> Result<(), DomainError>;

    /// Insere une attribution `api_user_guilds`. Renvoie `Some(granted_at)` si
    /// insere, `None` si le membre a deja un role sur la guild (unique
    /// violation) — le use case mappe ce cas en conflit.
    async fn insert_grant(
        &self,
        user_id: &str,
        guild_id: &str,
        role: &str,
        granted_by: &str,
    ) -> Result<Option<DateTime<Utc>>, DomainError>;

    /// Met a jour le role d'un membre. Renvoie le nombre de lignes affectees
    /// (0 = le membre n'a pas de role sur la guild).
    async fn update_role(
        &self,
        user_id: &str,
        guild_id: &str,
        role: &str,
    ) -> Result<u64, DomainError>;

    /// Nombre d'owners de la guild (garde-fou dernier owner).
    async fn count_owners(&self, guild_id: &str) -> Result<i64, DomainError>;

    /// `true` si le membre est owner de la guild.
    async fn is_owner(&self, user_id: &str, guild_id: &str) -> Result<bool, DomainError>;

    /// Role brut (chaine DB) du user sur la guild, ou `None` si aucune ligne
    /// `api_user_guilds` n'existe pour ce couple (user, guild). Le use case
    /// decide du fallback ; le repo ne fait que remonter la valeur telle quelle.
    async fn role_for_guild(
        &self,
        user_id: &str,
        guild_id: &str,
    ) -> Result<Option<String>, DomainError>;

    /// `true` si le user possede au moins une attribution (n'importe quelle
    /// guild, n'importe quel role) dans `api_user_guilds`. Sert au gate de
    /// whitelist global (defense en profondeur sur l'auth Discord).
    async fn is_whitelisted(&self, user_id: &str) -> Result<bool, DomainError>;

    /// Supprime l'attribution. Renvoie le nombre de lignes affectees
    /// (0 = le membre n'a pas de role sur la guild).
    async fn delete_grant(&self, user_id: &str, guild_id: &str) -> Result<u64, DomainError>;

    /// Liste les membres ayant un role sur la guild (tri par role puis nom).
    async fn list_guild_users(
        &self,
        guild_id: &str,
    ) -> Result<Vec<GuildUserEntry>, DomainError>;

    /// Upsert de la ligne `api_users` avec rafraichissement : cree la ligne si
    /// absente, sinon met a jour `display_name` (= valeur fournie) et
    /// `last_seen_at = NOW()`. Distinct de `upsert_user` (qui fait `DO NOTHING`).
    async fn record_user_seen(
        &self,
        user_id: &str,
        display_name: &str,
    ) -> Result<(), DomainError>;

    /// Auto-grant idempotent du proprietaire Discord comme `owner` RBAC au
    /// premier enregistrement de la guild (`ON CONFLICT DO NOTHING`) : si un
    /// role existe deja (meme viewer), on ne l'ecrase pas.
    async fn grant_owner_if_absent(
        &self,
        user_id: &str,
        guild_id: &str,
    ) -> Result<(), DomainError>;
}
