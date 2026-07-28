//! Groupes de roles mutuellement exclusifs : la logique (parsing config +
//! resolution des conflits) vit dans le core hexagonal.

pub use sentinel_core::domain::services::community::exclusive_groups::{
    get_conflicting_roles, parse_groups,
};
