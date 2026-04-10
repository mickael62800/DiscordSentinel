use uuid::Uuid;

/// Pari posé sur un combat Coup de Coude.
#[derive(Debug, Clone)]
pub struct CoudeBet {
    pub id: i64,
    pub guild_id: String,
    pub combat_id: Uuid,
    pub bettor_id: String,
    pub bettor_name: String,
    pub backed_id: String,
    pub amount: i64,
    /// `None` tant que le combat n'est pas résolu, `Some(true)` = gagné, `Some(false)` = perdu.
    pub won: Option<bool>,
    pub payout: Option<i64>,
}

/// Données nécessaires pour placer un nouveau pari.
#[derive(Debug, Clone)]
pub struct NewCoudeBet {
    pub guild_id: String,
    pub combat_id: Uuid,
    pub bettor_id: String,
    pub bettor_name: String,
    pub backed_id: String,
    pub amount: i64,
}

/// Résultat de résolution d'un pari : paiement individuel + flag gagné/perdu.
#[derive(Debug, Clone)]
pub struct BetPayout {
    pub bet_id: i64,
    pub bettor_id: String,
    pub bettor_name: String,
    pub backed_id: String,
    pub amount_bet: i64,
    pub payout: i64,
    pub won: bool,
}

/// Bonus versé aux deux combattants sur la commission pari-mutuel.
///
/// Commission totale = 15% du pot :
/// - 10% pour le vainqueur
/// - 5% pour le perdant (lot de consolation)
/// Les 85% restants sont redistribués proportionnellement aux parieurs gagnants.
#[derive(Debug, Clone)]
pub struct FighterBetBonus {
    pub winner_id: String,
    pub winner_bonus: i64,
    pub loser_id: String,
    pub loser_bonus: i64,
    pub total_pot: i64,
}

/// Plan de résolution calculé par `calculate_bet_resolution` et appliqué par le repo.
///
/// `fighter_bonus` est `None` en cas d'égalité (remboursement intégral).
#[derive(Debug, Clone)]
pub struct BetResolutionPlan {
    pub payouts: Vec<BetPayout>,
    pub fighter_bonus: Option<FighterBetBonus>,
}

/// Calcul pari-mutuel pur (pas d'I/O).
///
/// - `winner_id: Some(id)` : commission 15% (10% gagnant combattant, 5% perdant combattant),
///   85% distribués proportionnellement aux parieurs qui ont backé le gagnant.
/// - `winner_id: None` (égalité / pas de vainqueur) : tous les parieurs sont remboursés
///   de leur mise initiale, aucun bonus combattant.
pub fn calculate_bet_resolution(
    bets: &[CoudeBet],
    winner_id: Option<&str>,
    attacker_id: &str,
    defender_id: &str,
) -> BetResolutionPlan {
    if bets.is_empty() {
        return BetResolutionPlan {
            payouts: Vec::new(),
            fighter_bonus: None,
        };
    }

    let total_pot: i64 = bets.iter().map(|b| b.amount).sum();

    let Some(winner_id) = winner_id else {
        // Égalité : remboursement intégral. Les paris sont marqués `won = false`
        // mais payout = mise (convention du code legacy).
        let payouts = bets
            .iter()
            .map(|b| BetPayout {
                bet_id: b.id,
                bettor_id: b.bettor_id.clone(),
                bettor_name: b.bettor_name.clone(),
                backed_id: b.backed_id.clone(),
                amount_bet: b.amount,
                payout: b.amount,
                won: false,
            })
            .collect();
        return BetResolutionPlan {
            payouts,
            fighter_bonus: None,
        };
    };

    let commission = (total_pot as f64 * 0.15).round() as i64;
    let winner_bonus = (total_pot as f64 * 0.10).round() as i64;
    let loser_bonus = commission - winner_bonus;
    let distributable = total_pot - commission;

    let loser_id = if winner_id == attacker_id {
        defender_id.to_string()
    } else {
        attacker_id.to_string()
    };

    let winner_pool: i64 = bets
        .iter()
        .filter(|b| b.backed_id == winner_id)
        .map(|b| b.amount)
        .sum();

    let payouts = bets
        .iter()
        .map(|b| {
            if b.backed_id == winner_id {
                let share = if winner_pool > 0 {
                    ((b.amount as f64 / winner_pool as f64) * distributable as f64).round() as i64
                } else {
                    0
                };
                BetPayout {
                    bet_id: b.id,
                    bettor_id: b.bettor_id.clone(),
                    bettor_name: b.bettor_name.clone(),
                    backed_id: b.backed_id.clone(),
                    amount_bet: b.amount,
                    payout: share,
                    won: true,
                }
            } else {
                BetPayout {
                    bet_id: b.id,
                    bettor_id: b.bettor_id.clone(),
                    bettor_name: b.bettor_name.clone(),
                    backed_id: b.backed_id.clone(),
                    amount_bet: b.amount,
                    payout: 0,
                    won: false,
                }
            }
        })
        .collect();

    BetResolutionPlan {
        payouts,
        fighter_bonus: Some(FighterBetBonus {
            winner_id: winner_id.to_string(),
            winner_bonus,
            loser_id,
            loser_bonus,
            total_pot,
        }),
    }
}

/// Résumé d'un remboursement global (cancel de combat).
#[derive(Debug, Clone)]
pub struct RefundSummary {
    pub refunded_count: usize,
    pub refunded_total: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn bet(id: i64, bettor: &str, backed: &str, amount: i64) -> CoudeBet {
        CoudeBet {
            id,
            guild_id: "g".into(),
            combat_id: Uuid::nil(),
            bettor_id: bettor.into(),
            bettor_name: bettor.into(),
            backed_id: backed.into(),
            amount,
            won: None,
            payout: None,
        }
    }

    #[test]
    fn empty_bets_yield_empty_plan() {
        let plan = calculate_bet_resolution(&[], Some("A"), "A", "B");
        assert!(plan.payouts.is_empty());
        assert!(plan.fighter_bonus.is_none());
    }

    #[test]
    fn draw_refunds_everyone() {
        let bets = vec![bet(1, "u1", "A", 100), bet(2, "u2", "B", 50)];
        let plan = calculate_bet_resolution(&bets, None, "A", "B");
        assert_eq!(plan.payouts.len(), 2);
        assert_eq!(plan.payouts[0].payout, 100);
        assert_eq!(plan.payouts[1].payout, 50);
        assert!(!plan.payouts[0].won);
        assert!(plan.fighter_bonus.is_none());
    }

    #[test]
    fn winner_gets_proportional_share_minus_commission() {
        // Pot total = 1000. Commission = 150 (15%). Distribuable = 850.
        // Pool gagnant = 400 → u1 (300/400 * 850) + u2 (100/400 * 850) = 637 + 213 = 850.
        let bets = vec![
            bet(1, "u1", "A", 300),
            bet(2, "u2", "A", 100),
            bet(3, "u3", "B", 600),
        ];
        let plan = calculate_bet_resolution(&bets, Some("A"), "A", "B");
        let u1 = &plan.payouts[0];
        let u2 = &plan.payouts[1];
        let u3 = &plan.payouts[2];
        assert!(u1.won && u2.won && !u3.won);
        // Rounding peut sur-distribuer de 1 coin (637.5+212.5 → 638+213 = 851).
        // C'est le comportement du code legacy ; on tolère ±1 coin.
        let distributed = u1.payout + u2.payout;
        assert!(
            (849..=851).contains(&distributed),
            "distribution totale {distributed} hors tolérance [849,851]"
        );
        assert_eq!(u3.payout, 0);

        let bonus = plan.fighter_bonus.unwrap();
        assert_eq!(bonus.winner_id, "A");
        assert_eq!(bonus.loser_id, "B");
        assert_eq!(bonus.winner_bonus, 100); // 10%
        assert_eq!(bonus.loser_bonus, 50); // 5%
        assert_eq!(bonus.total_pot, 1000);
    }
}
