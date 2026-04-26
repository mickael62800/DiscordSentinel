//! Bouclier malchance du jour (cf. COUPE_AMELIORATIONS section 4.1).
//!
//! La PREMIERE defaite du jour pour un joueur est adoucie : la perte
//! coins est divisee par 2. Empeche la spirale "j ai perdu une fois,
//! je quitte". Pas de reset de win streak non plus si applique.
//!
//! Le call site (combat resolution) doit verifier `was_first_defeat_today`
//! via le repo (presence/absence d une row dans une table de tracking ou
//! requete SQL `SELECT COUNT(*) FROM coude_combats WHERE loser=user
//! AND DATE(resolved_at) = CURRENT_DATE`).

/// Multiplicateur applique a la perte si c est la premiere defaite du jour.
pub const LUCKY_SHIELD_LOSS_MULTIPLIER: f64 = 0.5;

/// Calcule la perte effective apres application eventuelle du bouclier
/// avec un multiplicateur explicite (0.5 par defaut, configurable via
/// `lucky_shield_loss_percent`).
pub fn apply_lucky_shield_with_multiplier(
    nominal_loss: i64,
    is_first_defeat_today: bool,
    multiplier: f64,
) -> i64 {
    if !is_first_defeat_today {
        return nominal_loss;
    }
    if nominal_loss <= 0 {
        return nominal_loss;
    }
    ((nominal_loss as f64) * multiplier).round() as i64
}

/// Variante avec le multiplicateur par defaut (0.5). Conservee pour la
/// retrocompatibilite des tests + des appels existants ; les nouveaux
/// call sites passent par `apply_lucky_shield_with_multiplier`.
pub fn apply_lucky_shield(nominal_loss: i64, is_first_defeat_today: bool) -> i64 {
    apply_lucky_shield_with_multiplier(
        nominal_loss,
        is_first_defeat_today,
        LUCKY_SHIELD_LOSS_MULTIPLIER,
    )
}

/// Doit-on preserver le win streak du perdant si le bouclier s applique ?
/// Reponse : oui, c est la regle metier. Le call site doit appeler ca.
pub fn should_preserve_win_streak_after_shielded_defeat(is_first_defeat_today: bool) -> bool {
    is_first_defeat_today
}

#[cfg(test)]
#[path = "tests/lucky_shield.rs"]
mod tests;
