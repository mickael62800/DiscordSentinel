//! Entite domaine : override du role minimum par composant sensible
//! (table `rbac_component_min_role`). Un override associe une cle de composant
//! (`db.purge.*`, `db.reset.*`, ...) au role minimum requis pour une guild.

/// Override explicite du min_role d'un composant pour une guild.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComponentMinRoleOverride {
    /// Cle du composant gate-able (ex: `db.purge.audit_logs`).
    pub component_key: String,
    /// Role minimum stocke (string brute ; clampe au floor cote API au gate).
    pub min_role: String,
}
