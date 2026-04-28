//! Implementation gRPC complete du domaine Coup de Coude.
//!
//! Phase 7A : `CoudePlayerService` — 6 methodes hot path joueurs.
//! Phase 7A.opt F.1 : 5 services supplementaires wrappant les 5 use cases
//! restants (combats, bets, economy, inventory, social). coude-bot est
//! maintenant 100% gRPC pour ses appels metier.
//!
//! Refactor 2026-04 : le god-object 1880 LOC a ete splitte en un module
//! directory avec 1 fichier par service + helpers partages ici (parse_uuid,
//! taunt_event_to_proto). Chaque sous-module contient sa propre impl
//! du trait tonic + ses helpers prives (en `pub(super)` quand les tests
//! de mod.rs en ont besoin).

use sentinel_proto::coude::v1 as proto;

// Re-export du parse_uuid centralise.
pub(super) use crate::adapters::inbound::grpc::parse_uuid;

/// Helper partage : convertit un `TauntEvent` domain en message proto.
/// Utilise par `CoudeCombatsService.ResolveCombatNow` (qui retourne
/// les TauntEvents emis pendant la resolution) et par
/// `CoudeSocialService.TrackStealVictim` (qui retourne un TauntEvent
/// optionnel si la streak vol de la victime franchit un palier).
pub(super) fn taunt_event_to_proto(
    e: crate::domain::entities::coude::taunt::TauntEvent,
) -> proto::TauntEvent {
    proto::TauntEvent {
        channel_id: e.channel_id,
        target_user_id: e.target_user_id,
        message: e.message,
        nickname_suffix: e.nickname_suffix,
        streak_kind: e.streak_kind.to_string(),
        streak_value: e.streak_value,
    }
}

pub mod bets;
pub mod combats;
pub mod economy;
pub mod inventory;
pub mod players;
pub mod social;

// ══════════════════════════════════════════════════════════════════════
// Tests unitaires des converters proto <-> domain (Phase 7A.opt F.1)
// ══════════════════════════════════════════════════════════════════════
//
// Ces tests verifient que la traduction entre les entites de domaine et
// les messages protobuf est complete et correcte (aucun champ oublie ou
// melange). Ce sont des fonctions pures, donc pas de DB ni de mock.

#[cfg(test)]
#[path = "tests/coude.rs"]
mod tests;
