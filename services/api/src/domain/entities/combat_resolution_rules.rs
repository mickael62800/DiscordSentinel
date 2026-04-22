//! Règles domain pures extraites de `resolve_combat_now_service`.
//! Logique de calcul métier autour de la résolution d'un combat :
//! application d'assurance sur les pertes + calcul des XP (Giant Killer)
//! + formatage des résultats de paris.

use crate::domain::entities::{BetResolutionPlan, CoudeInsurance};

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

/// Applique l'effet d'une assurance sur une perte de combat, en tenant
/// compte du solde réel du perdant.
///
/// **Ordre des opérations** (sémantique "Flow B", cf. commit unification) :
/// 1. Clamp `coins_lost` au solde réel du perdant (pas de découvert).
/// 2. Applique l'effet d'assurance sur le montant clampé :
///    - Pas d'assurance → perte = montant clampé.
///    - Assurance légitime → perte = clampé / 2 (protection effective même
///      si le joueur est fauché).
///    - Assurance scam → perte = (clampé × 2).min(balance) (double, re-clamp).
///
/// Rationale : l'assurance doit réduire ce que le joueur paie *réellement*,
/// pas un nombre théorique. Si on halve avant le clamp, un joueur fauché
/// voit son assurance annulée par le clamp final — anti-intuitif et punitif
/// envers ceux qui en ont le plus besoin.
///
/// Historique : avant unification, `resolve_combat_now` halvait avant clamp
/// (Flow A) tandis que `resolve_betting` clampait avant halving (Flow B).
/// Flow B retenu comme sémantique officielle.
pub fn apply_insurance_to_loss(
    coins_lost: i64,
    loser_balance: i64,
    insurance: Option<&CoudeInsurance>,
    loser_id: &str,
) -> InsuranceAdjustment {
    let clamped = coins_lost.min(loser_balance).max(0);
    match insurance {
        None => InsuranceAdjustment {
            actual_loss: clamped,
            message: None,
            consumed_insurance_id: None,
        },
        Some(ins) if ins.is_scam => {
            let doubled = clamped.saturating_mul(2).min(loser_balance);
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
            let halved = clamped / 2;
            InsuranceAdjustment {
                actual_loss: halved,
                message: Some(format!(
                    "\u{1f6e1}\u{fe0f} L'assurance a amorti le coup pour <@{}> ! Perte reduite : **-{} coins** (au lieu de {})",
                    loser_id, halved, clamped
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

/// Formate les lignes de résultats de paris pour un combat résolu.
/// Retourne None si aucun payout (rien à afficher).
///
/// - `plan` : résultat de `bets_uc.resolve` (payouts + bonus éventuel).
/// - `winner_id` / `loser_id` : None si draw (tous les parieurs perdent).
///
/// Utilisé par resolve_combat_now pour l'embed Discord.
pub fn format_bet_payout_lines(
    plan: &BetResolutionPlan,
    winner_id: Option<&str>,
    loser_id: Option<&str>,
) -> Option<String> {
    if plan.payouts.is_empty() {
        return None;
    }
    let mut lines = vec!["\u{1f3b2} **Resultats des paris :**".to_string()];
    for p in &plan.payouts {
        if p.won && winner_id.is_some() {
            lines.push(format!(
                "\u{2705} **{}** gagne **{} coins** !",
                p.bettor_name, p.payout
            ));
        } else {
            lines.push(format!(
                "\u{274c} **{}** perd sa mise de **{} coins**",
                p.bettor_name, p.amount_bet
            ));
        }
    }
    if let (Some(bonus), Some(winner), Some(loser)) =
        (plan.fighter_bonus.as_ref(), winner_id, loser_id)
    {
        lines.push(String::new());
        lines.push(format!(
            "\u{1f4b0} **Pot des paris : {} coins**",
            bonus.total_pot
        ));
        lines.push(format!(
            "\u{1f451} <@{}> recoit **+{} coins** (10% du pot)",
            winner, bonus.winner_bonus
        ));
        lines.push(format!(
            "\u{1f3c5} <@{}> recoit **+{} coins** (5% du pot, merci d'avoir participe)",
            loser, bonus.loser_bonus
        ));
    }
    Some(lines.join("\n"))
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
    // Sémantique Flow B : clamp(coins_lost, balance) d'abord, puis apply.

    #[test]
    fn insurance_none_clamps_to_balance() {
        // balance < coins_lost → loss = balance
        let adj = apply_insurance_to_loss(1000, 600, None, "user1");
        assert_eq!(adj.actual_loss, 600);
        assert!(adj.message.is_none());
    }

    #[test]
    fn insurance_none_solvent_player_pays_full() {
        let adj = apply_insurance_to_loss(500, 10_000, None, "user1");
        assert_eq!(adj.actual_loss, 500);
    }

    #[test]
    fn insurance_legitimate_halves_after_clamp() {
        // Le point critique de la correction : balance 100, coins_lost 1000
        // → clamp à 100, halve → 50 (pas 500 clampé à 100).
        let ins = make_insurance(false);
        let adj = apply_insurance_to_loss(1000, 100, Some(&ins), "user1");
        assert_eq!(adj.actual_loss, 50);
        assert!(adj.message.as_ref().unwrap().contains("amorti"));
        assert_eq!(adj.consumed_insurance_id, Some(ins.id));
    }

    #[test]
    fn insurance_legitimate_halves_when_solvent() {
        // Joueur solvable : clamp ne s'applique pas, halving du raw.
        let ins = make_insurance(false);
        let adj = apply_insurance_to_loss(1000, 10_000, Some(&ins), "user1");
        assert_eq!(adj.actual_loss, 500);
    }

    #[test]
    fn insurance_scam_doubles_then_caps_at_balance() {
        let ins = make_insurance(true);
        // coins 1000, balance 100 : clamp 100, x2 = 200, cap 100.
        let adj = apply_insurance_to_loss(1000, 100, Some(&ins), "u");
        assert_eq!(adj.actual_loss, 100);
        assert!(adj.message.as_ref().unwrap().contains("ARNAQUE"));
    }

    #[test]
    fn insurance_scam_solvent_player_loses_double_raw() {
        let ins = make_insurance(true);
        // coins 500, balance 10000 : clamp 500, x2 = 1000 (<balance).
        let adj = apply_insurance_to_loss(500, 10_000, Some(&ins), "u");
        assert_eq!(adj.actual_loss, 1000);
    }

    #[test]
    fn insurance_scam_saturates_on_overflow() {
        let ins = make_insurance(true);
        // Balance très élevée, coins_lost énorme : la saturation mul ne doit
        // pas panic, et le clamp final réaligne sur balance.
        let adj = apply_insurance_to_loss(i64::MAX, i64::MAX, Some(&ins), "u");
        assert_eq!(adj.actual_loss, i64::MAX);
    }

    #[test]
    fn insurance_halves_odd_number_floor_division() {
        let ins = make_insurance(false);
        // 999 clampé à 10_000 = 999, /2 = 499
        let adj = apply_insurance_to_loss(999, 10_000, Some(&ins), "u");
        assert_eq!(adj.actual_loss, 499);
    }

    #[test]
    fn insurance_on_zero_balance_is_zero() {
        let ins = make_insurance(false);
        let adj = apply_insurance_to_loss(1000, 0, Some(&ins), "u");
        assert_eq!(adj.actual_loss, 0);
    }

    #[test]
    fn insurance_scam_on_zero_balance_is_zero() {
        let ins = make_insurance(true);
        let adj = apply_insurance_to_loss(1000, 0, Some(&ins), "u");
        assert_eq!(adj.actual_loss, 0);
    }

    #[test]
    fn insurance_negative_balance_treated_as_zero() {
        // Défensif : si un repo retourne un solde négatif corrompu, on clamp à 0.
        let ins = make_insurance(false);
        let adj = apply_insurance_to_loss(100, -50, Some(&ins), "u");
        assert_eq!(adj.actual_loss, 0);
    }

    #[test]
    fn insurance_message_contains_user_mention() {
        let ins = make_insurance(false);
        let adj = apply_insurance_to_loss(100, 10_000, Some(&ins), "12345");
        assert!(adj.message.as_ref().unwrap().contains("<@12345>"));
    }

    #[test]
    fn insurance_broke_player_with_legit_insurance_gets_protected() {
        // Regression test pour le bug Flow A : auparavant, un joueur à 100
        // coins avec assurance légitime et perte théorique 1000 payait 100
        // (assurance inefficace). Maintenant il paie 50.
        let ins = make_insurance(false);
        let adj = apply_insurance_to_loss(1000, 100, Some(&ins), "broke_user");
        assert_eq!(adj.actual_loss, 50, "l'assurance doit proteger meme les joueurs fauches");
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

    // ── format_bet_payout_lines ──

    use crate::domain::entities::{BetPayout, CoudeFighterBetBonus};

    fn payout(name: &str, amount: i64, payout_amt: i64, won: bool) -> BetPayout {
        BetPayout {
            bet_id: Uuid::new_v4(),
            bettor_id: "u".into(),
            bettor_name: name.into(),
            backed_id: "a".into(),
            amount_bet: amount,
            payout: payout_amt,
            won,
        }
    }

    #[test]
    fn format_bet_payouts_empty_returns_none() {
        let plan = BetResolutionPlan { payouts: vec![], fighter_bonus: None };
        assert!(format_bet_payout_lines(&plan, Some("w"), Some("l")).is_none());
    }

    #[test]
    fn format_bet_payouts_win_lines() {
        let plan = BetResolutionPlan {
            payouts: vec![
                payout("Alice", 100, 250, true),
                payout("Bob", 50, 0, false),
            ],
            fighter_bonus: None,
        };
        let out = format_bet_payout_lines(&plan, Some("w"), Some("l")).unwrap();
        assert!(out.contains("Alice"));
        assert!(out.contains("gagne"));
        assert!(out.contains("250"));
        assert!(out.contains("Bob"));
        assert!(out.contains("perd"));
        assert!(out.contains("50"));
        assert!(out.starts_with("\u{1f3b2} **Resultats des paris"));
    }

    #[test]
    fn format_bet_payouts_draw_all_bettors_lose() {
        // En cas de draw, winner_id/loser_id = None → toutes les lignes
        // affichent "perd sa mise", même pour p.won=true (défensif).
        let plan = BetResolutionPlan {
            payouts: vec![payout("Carol", 200, 500, true)],
            fighter_bonus: None,
        };
        let out = format_bet_payout_lines(&plan, None, None).unwrap();
        assert!(out.contains("Carol"));
        assert!(out.contains("perd"));
        assert!(!out.contains("gagne"));
    }

    #[test]
    fn format_bet_payouts_with_fighter_bonus() {
        let plan = BetResolutionPlan {
            payouts: vec![payout("A", 10, 20, true)],
            fighter_bonus: Some(CoudeFighterBetBonus {
                winner_id: "w".into(), winner_bonus: 1000,
                loser_id: "l".into(), loser_bonus: 500,
                total_pot: 5000,
            }),
        };
        let out = format_bet_payout_lines(&plan, Some("w"), Some("l")).unwrap();
        assert!(out.contains("Pot des paris"));
        assert!(out.contains("5000"));
        assert!(out.contains("1000")); // winner bonus
        assert!(out.contains("500"));  // loser bonus
        assert!(out.contains("<@w>"));
        assert!(out.contains("<@l>"));
    }

    #[test]
    fn format_bet_payouts_bonus_skipped_on_draw() {
        // Draw (winner_id=None) → bonus ignoré même si présent dans le plan.
        let plan = BetResolutionPlan {
            payouts: vec![payout("A", 10, 0, false)],
            fighter_bonus: Some(CoudeFighterBetBonus {
                winner_id: "w".into(), winner_bonus: 1000,
                loser_id: "l".into(), loser_bonus: 500,
                total_pot: 5000,
            }),
        };
        let out = format_bet_payout_lines(&plan, None, None).unwrap();
        assert!(!out.contains("Pot des paris"));
        assert!(!out.contains("1000"));
    }
}
