//! Entite domaine : override de visibilite d'un composant UI par role
//! (`rbac_component_visibility`). Decrit si un `component_key` est visible
//! pour un `role` donne dans une guild.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisibilityEntry {
    pub component_key: String,
    pub role: String,
    pub visible: bool,
}
