//! Port inbound : use case CRUD du RBAC applicatif (Phase 7 B).
//!
//! Le gate d'autorisation HTTP (`require_role`) et le parsing des DTO restent
//! au handler (concern HTTP). Ce use case porte les garde-fous metier
//! (anti-lockout, dernier owner, troncature du display_name) et la persistance
//! (le SQL vit dans `RbacRepository`).

use async_trait::async_trait;

use crate::domain::entities::system::rbac::{GuildUserEntry, UserRoleGrant};
use crate::domain::enums::system::role::Role;
use crate::domain::errors::DomainError;

/// Attribution d'un role a un membre d'une guild.
pub struct GrantRoleCommand {
    pub guild_id: String,
    pub user_id: String,
    pub role: Role,
    /// Discord user id du caller (owner) qui accorde le role.
    pub granted_by: String,
    /// Nom d'affichage seedé dans `api_users` a la premiere attribution.
    pub display_name: Option<String>,
}

/// Modification du role d'un membre existant.
pub struct UpdateRoleCommand {
    pub guild_id: String,
    pub user_id: String,
    /// Discord user id du caller (owner) — sert au garde-fou anti-lockout.
    pub caller_id: String,
    pub role: Role,
}

/// Revocation du role d'un membre.
pub struct RevokeRoleCommand {
    pub guild_id: String,
    pub user_id: String,
}

#[async_trait]
pub trait ManageRbacUseCase: Send + Sync {
    /// Accorde un role a un membre. `ValidationError` si le membre a deja un
    /// role sur la guild (utiliser `update_role`).
    async fn grant_role(&self, cmd: GrantRoleCommand) -> Result<UserRoleGrant, DomainError>;

    /// Modifie le role d'un membre existant. `ValidationError` sur
    /// auto-retrogradation d'owner (lockout), `NotFound` si le membre n'a pas
    /// de role sur la guild.
    async fn update_role(&self, cmd: UpdateRoleCommand) -> Result<(), DomainError>;

    /// Revoque le role d'un membre. `ValidationError` si c'est le dernier owner
    /// de la guild, `NotFound` si le membre n'a pas de role.
    async fn revoke_role(&self, cmd: RevokeRoleCommand) -> Result<(), DomainError>;

    /// Liste les membres ayant un role sur la guild (tri par role puis nom).
    async fn list_guild_users(&self, guild_id: &str)
        -> Result<Vec<GuildUserEntry>, DomainError>;

    /// `true` si le user possede au moins une attribution RBAC (n'importe quelle
    /// guild). Alimente le gate de whitelist global du middleware d'auth Discord.
    async fn is_whitelisted(&self, user_id: &str) -> Result<bool, DomainError>;

    /// Role applicatif du user sur la guild d'apres `api_user_guilds`.
    /// `Ok(None)` si le user n'a AUCUNE attribution sur la guild (le caller
    /// decide du fallback, ex. `Viewer` moindre privilege) ; `Err` reservee aux
    /// VRAIES erreurs (DB indisponible, role DB invalide). Alimente le lookup de
    /// role du middleware RBAC.
    async fn role_for_guild(
        &self,
        user_id: &str,
        guild_id: &str,
    ) -> Result<Option<Role>, DomainError>;

    /// Enregistre le passage d'un user (upsert `api_users`) : cree la ligne si
    /// absente, sinon rafraichit `display_name` et `last_seen_at`. Best-effort
    /// cote appelant (le middleware ignore l'erreur). Alimente la resolution
    /// d'identite du middleware RBAC.
    async fn record_user_seen(
        &self,
        user_id: &str,
        display_name: &str,
    ) -> Result<(), DomainError>;

    /// Auto-grant idempotent du proprietaire Discord comme `owner` RBAC au
    /// premier enregistrement de la guild. N'ecrase aucun role existant.
    async fn ensure_owner_grant(
        &self,
        guild_id: &str,
        owner_id: &str,
    ) -> Result<(), DomainError>;
}
