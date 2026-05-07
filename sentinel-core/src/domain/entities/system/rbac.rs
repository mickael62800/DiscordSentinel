//! Regles metier du RBAC applicatif (Phase 7 B).
//!
//! L'enum `Role` + les checks d'authorisation vivent cote adapter
//! (`middleware/rbac.rs`) car ils dependent du contexte HTTP/middleware.
//! Ici on capture les regles **business** pures (garde-fous metier) que
//! le handler `handlers/rbac.rs` applique avant de toucher la DB :
//!
//! - Anti-lockout : un owner ne peut pas se retrograder (sinon plus
//!   personne ne peut gerer le RBAC de la guild).
//! - Dernier owner : on ne peut pas revoquer le seul owner restant
//!   (meme raison, et en plus les bots/endpoints `check_role_for_guild`
//!   s'appuient sur l'existence d'au moins un owner pour pouvoir
//!   operer sans superadmin).
//! - Display name : tronque a `RBAC_DISPLAY_NAME_MAX` chars pour
//!   respecter la colonne `api_users.display_name`.

/// Longueur max du `display_name` stocke dans `api_users`. Le handler
/// tronque en `chars()` (pas en bytes) pour etre unicode-safe.
pub const RBAC_DISPLAY_NAME_MAX: usize = 100;

/// `true` si un update de role correspondrait a une auto-retrogradation
/// d'un owner (caller modifie son propre role vers autre chose qu'owner).
///
/// Usage : refuser l'update avec une ValidationError explicite ("lockout risk").
pub fn is_owner_self_demotion(
    caller_id: &str,
    target_user_id: &str,
    new_role: &str,
) -> bool {
    caller_id == target_user_id && new_role != "owner"
}

/// `true` si revoquer le role ciblerait le dernier owner de la guild
/// (= lockout definitif). `total_owners_for_guild` est le nombre courant
/// de lignes `api_user_guilds` avec `role = 'owner'` pour la guild.
///
/// Usage : refuser le DELETE avec une ValidationError.
pub fn would_revoke_last_owner(
    is_target_owner: bool,
    total_owners_for_guild: i64,
) -> bool {
    is_target_owner && total_owners_for_guild <= 1
}

/// Tronque un display_name a la limite DB (`RBAC_DISPLAY_NAME_MAX` chars).
/// Unicode-safe : decoupe par `char`, pas par byte.
pub fn truncate_display_name(raw: &str) -> String {
    raw.chars().take(RBAC_DISPLAY_NAME_MAX).collect()
}

#[cfg(test)]
#[path = "tests/rbac.rs"]
mod tests;
