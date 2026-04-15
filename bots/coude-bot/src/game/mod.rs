//! Catalogues d'affichage et pure math pour Coup de Coude.
//!
//! Ce module NE contient PAS de logique metier — uniquement des donnees
//! statiques (catalogue classes/shop) et des fonctions pures de progression
//! utilisees pour l'affichage cote Discord. Toute la logique metier
//! (combat, chaos, resolve, stats) vit dans l'API :
//! `services/api/src/domain/services/coude_combat_engine/`.

pub mod classes;
pub mod progression;
pub mod shop;
