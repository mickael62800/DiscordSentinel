//! Gates d'autorisation granulaires par "composant" (cle string), avec
//! override par-guild stocke en DB (table `rbac_component_min_role`).
//!
//! Le default + floor sont hardcodes ici (source de verite cote API). Ils
//! sont mirrores cote frontend dans `sentinel-web/src/rbac/componentRegistry.ts`
//! pour cacher les boutons coherents avec ce que l'API autorise.
//!
//! Modele de resolution :
//!   1. Si la guild a un override en DB pour cette cle :
//!      effective_role = max(override, floor)   // le floor protege
//!   2. Sinon :
//!      effective_role = default
//!   3. On gate avec `check_role_for_guild(effective_role)`.
//!
//! Cache Redis 60s sur le lookup DB pour eviter une query par appel.

use axum::Extension;
use redis::AsyncCommands;
use std::collections::HashMap;
use std::sync::OnceLock;

use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::middleware::rbac::check_role_for_guild;
use crate::adapters::inbound::http::middleware::rbac::RoleContext;
use crate::adapters::inbound::http::state::AppState;
use sentinel_core::domain::enums::system::role::Role;

/// Definition d'un composant gate-able par RBAC granulaire.
#[derive(Debug, Clone, Copy)]
pub struct GateDef {
    /// Role minimum par defaut (utilise si aucun override en DB).
    pub default_role: Role,
    /// Borne plancher : un override ne peut JAMAIS descendre en-dessous.
    /// Protege contre un owner qui par megarde ouvrirait une purge a tous.
    pub floor: Role,
}

/// Registry statique des gates supportes. Toute cle utilisee par
/// `check_component_role` doit etre listee ici (sinon panic en debug, fail
/// fermeture en prod = retour Owner).
fn registry() -> &'static HashMap<&'static str, GateDef> {
    static REG: OnceLock<HashMap<&'static str, GateDef>> = OnceLock::new();
    REG.get_or_init(|| {
        let mut m = HashMap::new();

        // ── Nettoyages base de donnees ─────────────────────────────────
        // default = Owner (par defaut tout owner-only),
        // floor   = Admin (un owner peut descendre a Admin mais pas plus).
        m.insert(
            "db.purge.audit_logs",
            GateDef {
                default_role: Role::Owner,
                floor: Role::Admin,
            },
        );
        m.insert(
            "db.purge.security_events",
            GateDef {
                default_role: Role::Owner,
                floor: Role::Admin,
            },
        );
        m.insert(
            "db.purge.voice_history",
            GateDef {
                default_role: Role::Owner,
                floor: Role::Admin,
            },
        );

        // ── Rotation d'admin ───────────────────────────────────────────
        // Ecrire l'etat de rotation revient a designer le detenteur d'un role
        // privilegie -> owner-only STRICT (floor Owner, pas de descente possible).
        // La cle etait absente du registry (fallback Owner par accident + spam de
        // logs) ; on la fige ici pour eviter qu'une resync avec le front ("admin")
        // ne l'abaisse a Admin -> escalade.
        m.insert(
            "rotation.dashboard",
            GateDef {
                default_role: Role::Owner,
                floor: Role::Owner,
            },
        );
        m.insert(
            "db.purge.voice_channel",
            GateDef {
                default_role: Role::Owner,
                floor: Role::Moderator,
            },
        );

        // ── Game Portal ──────────────────────────────────────────────
        m.insert(
            "game.server.create",
            GateDef {
                default_role: Role::Admin,
                floor: Role::Moderator,
            },
        );
        m.insert(
            "game.server.delete",
            GateDef {
                default_role: Role::Owner,
                floor: Role::Admin,
            },
        );
        m.insert(
            "game.server.start_stop",
            GateDef {
                default_role: Role::Moderator,
                floor: Role::Moderator,
            },
        );
        m.insert(
            "game.server.config_edit",
            GateDef {
                default_role: Role::Admin,
                floor: Role::Moderator,
            },
        );
        // RCON = console admin avec /op, /whitelist, /kick. Strictement
        // owner par defaut, descendable a Admin maximum.
        m.insert(
            "game.server.command_rcon",
            GateDef {
                default_role: Role::Owner,
                floor: Role::Admin,
            },
        );
        // Lecture serveur (get/logs/stats/list) : les logs peuvent contenir des
        // secrets (RCON) et des IP -> reserve aux moderateurs.
        m.insert(
            "game.server.view",
            GateDef {
                default_role: Role::Moderator,
                floor: Role::Moderator,
            },
        );
        // Lecture des sessions / inscriptions.
        m.insert(
            "game.session.view",
            GateDef {
                default_role: Role::Moderator,
                floor: Role::Moderator,
            },
        );
        // Gestion des inscriptions (inscrire/desinscrire cote web). L'auto-
        // inscription du bot passe en Internal (bypass RBAC).
        m.insert(
            "game.session.register",
            GateDef {
                default_role: Role::Moderator,
                floor: Role::Moderator,
            },
        );
        // Reglages de session (role de template, salons) = potentielle escalade
        // (assignation de role) -> Admin.
        m.insert(
            "game.session.settings_edit",
            GateDef {
                default_role: Role::Admin,
                floor: Role::Moderator,
            },
        );

        m
    })
}

/// Resout le role effectif pour (guild, component_key). Cache Redis 60s.
async fn resolve_min_role(state: &AppState, guild_id: &str, component_key: &str) -> Role {
    let def = match registry().get(component_key) {
        Some(d) => *d,
        None => {
            tracing::error!(
                component_key,
                "component_gates: cle inconnue, fallback Owner"
            );
            return Role::Owner;
        }
    };

    let cache_key = format!("rbac:min_role:{}:{}", guild_id, component_key);
    if let Ok(mut conn) = state.redis_client.get_multiplexed_async_connection().await {
        if let Ok(Some(s)) = conn.get::<_, Option<String>>(&cache_key).await {
            if let Some(r) = Role::from_str(&s) {
                return clamp_to_floor(r, def);
            }
        }
    }

    // Lecture de l'override via le use case (SQL confine cote adapter Postgres).
    // On preserve la semantique historique : une erreur DB est traitee comme
    // "pas d'override" (`.ok().flatten()`), donc fallback vers `default_role`.
    let row: Option<String> = state
        .component_min_role_uc
        .get_override(guild_id, component_key)
        .await
        .ok()
        .flatten();

    let role = match row.and_then(|s| Role::from_str(&s)) {
        Some(override_role) => clamp_to_floor(override_role, def),
        None => def.default_role,
    };

    if let Ok(mut conn) = state.redis_client.get_multiplexed_async_connection().await {
        let _ = conn.set_ex::<_, _, ()>(&cache_key, role.as_str(), 60).await;
    }
    role
}

fn clamp_to_floor(r: Role, def: GateDef) -> Role {
    if r < def.floor {
        def.floor
    } else {
        r
    }
}

/// Helper public : resout le min_role et delegue a `check_role_for_guild`.
///
/// Usage handler:
/// ```ignore
/// check_component_role(&state, &rbac, &guild_id, "db.purge.audit_logs",
///   "owner par defaut pour purger les audit logs").await?;
/// ```
pub async fn check_component_role(
    state: &AppState,
    rbac: &Option<Extension<RoleContext>>,
    guild_id: &str,
    component_key: &'static str,
    label: &'static str,
) -> Result<(), ApiError> {
    let role = resolve_min_role(state, guild_id, component_key).await;
    check_role_for_guild(state, rbac, guild_id, role, label).await
}

/// Invalide le cache Redis pour (guild, component_key). A appeler apres
/// upsert/delete dans la table `rbac_component_min_role`.
pub async fn invalidate_cache(state: &AppState, guild_id: &str, component_key: &str) {
    if let Ok(mut conn) = state.redis_client.get_multiplexed_async_connection().await {
        let key = format!("rbac:min_role:{}:{}", guild_id, component_key);
        let _ = conn.del::<_, ()>(&key).await;
    }
}

/// Liste publique des gates (pour endpoint d'introspection cote frontend).
pub fn list_gates() -> Vec<(&'static str, GateDef)> {
    let mut v: Vec<_> = registry().iter().map(|(k, v)| (*k, *v)).collect();
    v.sort_by_key(|(k, _)| *k);
    v
}
