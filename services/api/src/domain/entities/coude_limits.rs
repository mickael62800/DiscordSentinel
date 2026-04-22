//! Limites et defaults pour les endpoints Coup de Coude (combats, players,
//! social). Ces constantes vivent ici pour que la regle metier "combien
//! de combats/joueurs/lignes renvoyer par defaut" soit centralisee et
//! testee.

/// Limite par defaut pour la liste des combats d'une guild (admin panel).
pub const DEFAULT_COUDE_COMBATS_LIMIT: i64 = 50;

/// Nombre de joueurs suggeres par defaut pour un match-making.
/// Regle metier : 2 = duel (1v1), le mode par defaut.
pub const DEFAULT_COUDE_OPPONENT_COUNT: i64 = 2;

/// Taille par defaut du leaderboard social d'une guild.
pub const DEFAULT_COUDE_SOCIAL_LEADERBOARD_LIMIT: i64 = 10;

#[cfg(test)]
#[path = "tests/coude_limits.rs"]
mod tests;
