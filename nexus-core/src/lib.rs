//! # nexus-core — coeur hexagonal de la plateforme jeux Nexus
//!
//! Lib pure (domain + application + ports), calquee sur `sentinel-core`.
//!
//! ## Regles d'architecture (identiques a sentinel-core)
//! - AUCUNE dependance infra : pas de `sqlx`, `axum`, `reqwest`, `redis`,
//!   ni `serenity`. Seules les deps "pures" sont admises (serde, thiserror,
//!   chrono, uuid).
//! - `domain` n'importe NI `ports` NI `application` : entites, services de
//!   domaine et enums purs uniquement.
//! - `application` orchestre le domaine via les `ports` (traits).
//! - `ports::inbound` = cas d'usage exposes ; `ports::outbound` = besoins
//!   d'infra abstraits (repos, gateways), implementes par les adapters des
//!   binaires (`nexus-api`, `nexus-bot`, `nexus-worker`, `nexus-gateway`).

pub mod application;
pub mod domain;
pub mod ports;
