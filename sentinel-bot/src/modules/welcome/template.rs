//! Rendu des messages de bienvenue/depart : la logique (placeholders +
//! parsing couleur) vit dans le core hexagonal.

pub use sentinel_core::domain::services::community::welcome_template::{parse_color, render};
