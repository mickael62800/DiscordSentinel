//! Hierarchie des roles RBAC, partagee entre middleware HTTP, gRPC et tout
//! consommateur du domain.

/// Hierarchie des roles RBAC (le plus fort en premier).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Role {
    /// Read-only.
    Viewer = 0,
    /// Sanctions, tickets, notes.
    Moderator = 1,
    /// Full CRUD sauf RBAC.
    Admin = 2,
    /// Full access + gestion du RBAC.
    Owner = 3,
}

impl Role {
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "viewer" => Some(Role::Viewer),
            "moderator" => Some(Role::Moderator),
            "admin" => Some(Role::Admin),
            "owner" => Some(Role::Owner),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Role::Viewer => "viewer",
            Role::Moderator => "moderator",
            Role::Admin => "admin",
            Role::Owner => "owner",
        }
    }

    /// `true` si ce role peut faire une action necessitant au moins `required`.
    pub fn satisfies(&self, required: Role) -> bool {
        *self >= required
    }
}
