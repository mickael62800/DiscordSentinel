//! Règles domain pures extraites de `resolve_combat_now_service`.
//! Logique de calcul métier autour de la résolution d'un combat :
//! application d'assurance sur les pertes + calcul des XP (Giant Killer).

use crate::domain::entities::CoudeInsurance;

/// Résultat de l'ajustement d'une perte par une assurance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InsuranceAdjustment {
    /// Perte finale après application de l'assurance.
    pub actual_loss: i64,
    /// Message à afficher dans l'embed (None si pas d'assurance).
    pub message: Option<String>,
    /// Si Some, l'assurance a été consommée et doit être expirée.
    pub consumed_insurance_id: Option<uuid::Uuid>,
}

/// Applique l'effet d'une assurance sur une perte de combat :
/// - Pas d'assurance → perte inchangée, pas de message.
/// - Assurance légitime → perte divisée par 2.
/// - Assurance scam → perte doublée (saturating).
///
/// Le `loser_id` sert uniquement à générer le message d'embed.
pub fn apply_insurance_to_loss(
    coins_lost: i64,
    insurance: Option<&CoudeInsurance>,
    loser_id: &str,
) -> InsuranceAdjustment {
    match insurance {
        None => InsuranceAdjustment {
            actual_loss: coins_lost,
            message: None,
            consumed_insurance_id: None,
        },
        Some(ins) if ins.is_scam => {
            let doubled = coins_lost.saturating_mul(2);
            InsuranceAdjustment {
                actual_loss: doubled,
                message: Some(format!(
                    "\u{1f480} L'assurance de <@{}> etait une **ARNAQUE** ! Double perte : **-{} coins** !",
                    loser_id, doubled
                )),
                consumed_insurance_id: Some(ins.id),
            }
        }
        Some(ins) => {
            let halved = coins_lost / 2;
            InsuranceAdjustment {
                actual_loss: halved,
                message: Some(format!(
                    "\u{1f6e1}\u{fe0f} L'assurance a amorti le coup pour <@{}> ! Perte reduite : **-{} coins** (au lieu de {})",
                    loser_id, halved, coins_lost
                )),
                consumed_insurance_id: Some(ins.id),
            }
        }
    }
}

/// XP attribué après un combat résolu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CombatXpAwards {
    pub winner_xp: i64,
    pub loser_xp: i64,
    /// True si le gagnant bénéficie du bonus Giant Killer (x2 XP).
    pub winner_is_underdog: bool,
}

/// XP de base pour le gagnant sans bonus.
pub const COMBAT_XP_WINNER_BASE: i64 = 15;
/// XP gagnant avec bonus Giant Killer (underdog de ≥3 niveaux).
pub const COMBAT_XP_WINNER_UNDERDOG: i64 = 30;
/// XP consolation pour le perdant.
pub const COMBAT_XP_LOSER: i64 = 5;
/// Écart de niveaux minimum pour activer le bonus underdog.
pub const UNDERDOG_LEVEL_GAP: i32 = 3;

/// Calcule les XP attribués après un combat.
/// Le bonus "Giant Killer" s'applique si le gagnant avait ≥3 niveaux d'écart
/// avec le perdant ET a effectivement déclenché le flag `is_giant_killer`
/// au sein du moteur de combat.
pub fn compute_combat_xp(
    attacker_level: i32,
    defender_level: i32,
    is_giant_killer: bool,
) -> CombatXpAwards {
    let level_gap = (attacker_level - defender_level).abs();
    let winner_is_underdog = level_gap >= UNDERDOG_LEVEL_GAP && is_giant_killer;
    let winner_xp = if winner_is_underdog {
        COMBAT_XP_WINNER_UNDERDOG
    } else {
        COMBAT_XP_WINNER_BASE
    };
    CombatXpAwards {
        winner_xp,
        loser_xp: COMBAT_XP_LOSER,
        winner_is_underdog,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use uuid::Uuid;

    fn make_insurance(is_scam: bool) -> CoudeInsurance {
        CoudeInsurance {
            id: Uuid::new_v4(),
            is_scam,
            expires_at: Utc::now() + chrono::Duration::days(1),
        }
    }

    // ── apply_insurance_to_loss ──

    #[test]
    fn insurance_none_leaves_loss_unchanged() {
        let adj = apply_insurance_to_loss(1000, None, "user1");
        assert_eq!(adj.actual_loss, 1000);
        assert!(adj.message.is_none());
        assert!(adj.consumed_insurance_id.is_none());
    }

    #[test]
    fn insurance_legitimate_halves_loss() {
        let ins = make_insurance(false);
        let adj = apply_insurance_to_loss(1000, Some(&ins), "user1");
        assert_eq!(adj.actual_loss, 500);
        assert!(adj.message.as_ref().unwrap().contains("amorti"));
        assert!(adj.message.as_ref().unwrap().contains("500"));
        assert_eq!(adj.consumed_insurance_id, Some(ins.id));
    }

    #[test]
    fn insurance_scam_doubles_loss() {
        let ins = make_insurance(true);
        let adj = apply_insurance_to_loss(1000, Some(&ins), "user1");
        assert_eq!(adj.actual_loss, 2000);
        assert!(adj.message.as_ref().unwrap().contains("ARNAQUE"));
        assert_eq!(adj.consumed_insurance_id, Some(ins.id));
    }

    #[test]
    fn insurance_scam_saturates_on_overflow() {
        let ins = make_insurance(true);
        let adj = apply_insurance_to_loss(i64::MAX, Some(&ins), "u");
        assert_eq!(adj.actual_loss, i64::MAX); // saturated
    }

    #[test]
    fn insurance_halves_odd_number_floor_division() {
        // 999 / 2 = 499 (division entiere)
        let ins = make_insurance(false);
        let adj = apply_insurance_to_loss(999, Some(&ins), "u");
        assert_eq!(adj.actual_loss, 499);
    }

    #[test]
    fn insurance_halves_zero_remains_zero() {
        let ins = make_insurance(false);
        let adj = apply_insurance_to_loss(0, Some(&ins), "u");
        assert_eq!(adj.actual_loss, 0);
    }

    #[test]
    fn insurance_scam_on_zero_remains_zero() {
        let ins = make_insurance(true);
        let adj = apply_insurance_to_loss(0, Some(&ins), "u");
        assert_eq!(adj.actual_loss, 0);
    }

    #[test]
    fn insurance_message_contains_user_mention() {
        let ins = make_insurance(false);
        let adj = apply_insurance_to_loss(100, Some(&ins), "12345");
        assert!(adj.message.as_ref().unwrap().contains("<@12345>"));
    }

    // ── compute_combat_xp ──

    #[test]
    fn xp_no_level_gap_returns_base_winner_xp() {
        let awards = compute_combat_xp(10, 10, false);
        assert_eq!(awards.winner_xp, COMBAT_XP_WINNER_BASE);
        assert_eq!(awards.loser_xp, COMBAT_XP_LOSER);
        assert!(!awards.winner_is_underdog);
    }

    #[test]
    fn xp_small_gap_does_not_trigger_underdog() {
        // Ecart de 2 niveaux = pas underdog meme si giant_killer=true
        let awards = compute_combat_xp(5, 7, true);
        assert!(!awards.winner_is_underdog);
        assert_eq!(awards.winner_xp, COMBAT_XP_WINNER_BASE);
    }

    #[test]
    fn xp_big_gap_with_giant_killer_flag_doubles_winner() {
        // Ecart de 5 levels + giant killer = underdog
        let awards = compute_combat_xp(3, 8, true);
        assert!(awards.winner_is_underdog);
        assert_eq!(awards.winner_xp, COMBAT_XP_WINNER_UNDERDOG);
    }

    #[test]
    fn xp_big_gap_without_giant_killer_flag_no_bonus() {
        // Ecart de 10 levels mais giant_killer=false → pas de bonus
        let awards = compute_combat_xp(3, 13, false);
        assert!(!awards.winner_is_underdog);
        assert_eq!(awards.winner_xp, COMBAT_XP_WINNER_BASE);
    }

    #[test]
    fn xp_gap_uses_absolute_value() {
        // Que l'attaquant soit au-dessus ou au-dessous, la valeur absolue s'applique.
        let a = compute_combat_xp(3, 10, true);
        let b = compute_combat_xp(10, 3, true);
        assert_eq!(a.winner_is_underdog, b.winner_is_underdog);
        assert_eq!(a.winner_xp, b.winner_xp);
    }

    #[test]
    fn xp_exactly_three_levels_triggers_underdog() {
        // Boundary : 3 niveaux d'ecart = seuil exact
        let awards = compute_combat_xp(5, 8, true);
        assert!(awards.winner_is_underdog);
    }

    #[test]
    fn xp_loser_always_gets_five_xp() {
        for gap in 0..20 {
            let awards = compute_combat_xp(10, 10 + gap, true);
            assert_eq!(awards.loser_xp, COMBAT_XP_LOSER);
        }
    }
}
