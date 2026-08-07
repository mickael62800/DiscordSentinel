//! `sentinel-api` — adaptateurs et composition root de l'API Sentinel.
//!
//! Le metier vit dans `sentinel-core` : `domain`, `application` et `ports` se
//! referencent directement par `sentinel_core::…`. Ce crate re-exportait
//! auparavant `sentinel_core::{ports, application}` sous `crate::`, ce qui
//! donnait deux chemins valides pour le meme type — au point qu'un meme
//! fichier melangeait les deux formes. Les re-exports ont ete retires.

pub mod adapters;
pub mod bootstrap;
pub mod config;
