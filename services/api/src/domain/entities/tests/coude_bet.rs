use super::*;
use uuid::Uuid;

fn bet(id: i64, bettor: &str, backed: &str, amount: i64) -> CoudeBet {
    // Mapper id i64 -> Uuid deterministe pour les tests legacy.
    let id = Uuid::from_u128(id as u128);
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
    let distributed = u1.payout + u2.payout;
    assert!(
        (849..=851).contains(&distributed),
        "distribution totale {distributed} hors tolérance [849,851]"
    );
    assert_eq!(u3.payout, 0);

    let bonus = plan.fighter_bonus.unwrap();
    assert_eq!(bonus.winner_id, "A");
    assert_eq!(bonus.loser_id, "B");
    assert_eq!(bonus.winner_bonus, 100);
    assert_eq!(bonus.loser_bonus, 50);
    assert_eq!(bonus.total_pot, 1000);
}

// ══════════════════════════════════════════════════════════
//  Stress tests — invariants avec valeurs extremes et aleatoires
// ══════════════════════════════════════════════════════════

/// Invariant : pour chaque resolution, le total distribue aux parieurs + le
/// bonus combattants doit etre ≤ total_pot (aucune creation de coins).
#[test]
fn invariant_no_coin_creation() {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    for iter in 0..500 {
        let n: usize = rng.gen_range(1..=10);
        let bets: Vec<CoudeBet> = (0..n)
            .map(|i| {
                let backed = if rng.gen_bool(0.5) { "A" } else { "B" };
                let amount: i64 = rng.gen_range(1..=100_000);
                bet(i as i64, &format!("u{i}"), backed, amount)
            })
            .collect();
        let total_pot: i64 = bets.iter().map(|b| b.amount).sum();
        let winner = if rng.gen_bool(0.5) { Some("A") } else { Some("B") };
        let plan = calculate_bet_resolution(&bets, winner, "A", "B");

        let distributed: i64 = plan.payouts.iter().map(|p| p.payout).sum();
        let fighter_total: i64 = plan
            .fighter_bonus
            .as_ref()
            .map(|f| f.winner_bonus + f.loser_bonus)
            .unwrap_or(0);

        // Invariant : distributed + fighter_total <= total_pot + tolerance d'arrondi
        // Tolerance = n_winners (1 coin de rounding par gagnant max).
        let n_winners = plan.payouts.iter().filter(|p| p.won).count() as i64;
        let tolerance = n_winners + 2; // +2 pour commission rounding
        assert!(
            distributed + fighter_total <= total_pot + tolerance,
            "[iter {iter}] coin creation: distributed={distributed}, fighter={fighter_total}, pot={total_pot}, tolerance={tolerance}"
        );
    }
}

/// Invariant : en cas d'egalite (winner=None), chaque parieur recupere exactement sa mise.
#[test]
fn draw_always_refunds_exact_amount() {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    for _ in 0..100 {
        let n: usize = rng.gen_range(1..=20);
        let bets: Vec<CoudeBet> = (0..n)
            .map(|i| bet(i as i64, &format!("u{i}"), "A", rng.gen_range(1..=1_000_000)))
            .collect();
        let plan = calculate_bet_resolution(&bets, None, "A", "B");
        for (b, p) in bets.iter().zip(plan.payouts.iter()) {
            assert_eq!(p.payout, b.amount, "egalite doit rembourser la mise exacte");
            assert!(!p.won, "egalite : won = false par convention");
        }
        assert!(plan.fighter_bonus.is_none(), "egalite : pas de bonus combattants");
    }
}

/// Si aucun parieur n'a backe le gagnant, le pool est a 0 : les parieurs
/// perdants doivent avoir payout=0, mais les combattants touchent quand meme
/// leur bonus (commission). Verifier que c'est coherent.
#[test]
fn no_winning_bettors_zero_payout_but_fighter_bonus_still_paid() {
    let bets = vec![
        bet(1, "u1", "B", 500),
        bet(2, "u2", "B", 500),
    ];
    let plan = calculate_bet_resolution(&bets, Some("A"), "A", "B");
    for p in &plan.payouts {
        assert_eq!(p.payout, 0, "pas de gagnants → payout 0");
        assert!(!p.won);
    }
    let bonus = plan.fighter_bonus.unwrap();
    // winner_bonus = 10% de 1000 = 100 ; loser_bonus = 150 - 100 = 50.
    assert_eq!(bonus.winner_bonus, 100);
    assert_eq!(bonus.loser_bonus, 50);
    // 🐛 Observation : les combattants touchent 150 coins alors que personne
    // n'a parie sur le gagnant. 850 coins (distributable) sont "perdus" — pas de
    // redistribution ni retour aux parieurs.
    let distributed: i64 = plan.payouts.iter().map(|p| p.payout).sum();
    let fighter_total = bonus.winner_bonus + bonus.loser_bonus;
    let total_pot: i64 = bets.iter().map(|b| b.amount).sum();
    assert!(distributed + fighter_total < total_pot, "coins non redistribues");
}

/// Un seul parieur qui backe le gagnant recupere 85% du pot (distributable).
#[test]
fn single_winning_bettor_gets_full_distributable() {
    let bets = vec![bet(1, "u1", "A", 1000)];
    let plan = calculate_bet_resolution(&bets, Some("A"), "A", "B");
    // commission = 1000 * 0.15 = 150 ; distributable = 850
    assert_eq!(plan.payouts[0].payout, 850);
    assert!(plan.payouts[0].won);
}

/// Rounding : 3 parieurs a parts egales — verifier que le total perdu par
/// arrondi reste ≤ n-1 coins.
#[test]
fn rounding_loss_bounded_by_winner_count() {
    // 3 parieurs a 100 coins chacun = 300 pot. Commission = 45. Distributable = 255.
    // Chacun : 100/300 * 255 = 85 pile. Total = 255. Pas de rounding loss ici.
    let bets = vec![
        bet(1, "u1", "A", 100),
        bet(2, "u2", "A", 100),
        bet(3, "u3", "A", 100),
    ];
    let plan = calculate_bet_resolution(&bets, Some("A"), "A", "B");
    let total: i64 = plan.payouts.iter().map(|p| p.payout).sum();
    assert!(total >= 253 && total <= 256, "total {total} hors tolerance");

    // Cas avec rounding : 3 parts de 100 sur pot de 100 ne divise pas exactement.
    // 100*0.85 = 85, part = 100/100 * 85 = 85. OK.
    // Cas plus interessant : 7 parieurs
    let bets: Vec<CoudeBet> = (0..7)
        .map(|i| bet(i, &format!("u{i}"), "A", 100))
        .collect();
    let plan = calculate_bet_resolution(&bets, Some("A"), "A", "B");
    // total_pot = 700, commission = round(700*0.15) = 105, distributable = 595.
    // Chaque parieur : 100/700 * 595 = 85 pile.
    let total: i64 = plan.payouts.iter().map(|p| p.payout).sum();
    assert_eq!(total, 595);
}

/// Montants negatifs : le code n'est pas garde contre ca — documente le
/// comportement actuel (qui peut creer des inconsistances si la validation
/// amont tombe).
#[test]
fn negative_amount_passthrough_without_validation() {
    // Si amount negatif ateint le calcul, le pot peut etre negatif.
    let bets = vec![
        bet(1, "u1", "A", 100),
        bet(2, "u2", "A", -50), // 🐛 devrait etre rejete amont, mais on documente le passthrough
    ];
    let plan = calculate_bet_resolution(&bets, Some("A"), "A", "B");
    // total_pot = 50. commission = round(50*0.15) = 8. distributable = 42.
    // winner_pool = 50. parts : u1 = 100/50 * 42 = 84, u2 = -50/50 * 42 = -42.
    // C'est absurde mais le code actuel laisse passer. Ce test verrouille le
    // comportement et garantit qu'une future validation amont sera detectee.
    let total: i64 = bets.iter().map(|b| b.amount).sum();
    let bonus = plan.fighter_bonus.unwrap();
    assert_eq!(bonus.total_pot, total); // propagation de la negativite
}

/// Ordre des payouts : doit refleter l'ordre des paris entrants.
#[test]
fn payouts_preserve_input_order() {
    let bets = vec![
        bet(10, "zulu", "A", 100),
        bet(20, "alpha", "B", 200),
        bet(30, "mike", "A", 150),
    ];
    let plan = calculate_bet_resolution(&bets, Some("A"), "A", "B");
    assert_eq!(plan.payouts.len(), 3);
    assert_eq!(plan.payouts[0].bet_id, Uuid::from_u128(10));
    assert_eq!(plan.payouts[1].bet_id, Uuid::from_u128(20));
    assert_eq!(plan.payouts[2].bet_id, Uuid::from_u128(30));
}

#[test]
fn winner_pool_zero_with_matching_backs_gives_zero_share() {
    // Couvre la branche defensive line 131 : si backed_id == winner_id mais
    // winner_pool == 0 (tous les montants gagnants a 0), share = 0.
    let bets = vec![
        bet(1, "u1", "A", 0),    // mise 0 sur A (winner), mais pot zero
        bet(2, "u2", "B", 100),
    ];
    let plan = calculate_bet_resolution(&bets, Some("A"), "A", "B");
    // u1 backe A (gagnant) mais winner_pool=0 → payout=0.
    assert_eq!(plan.payouts[0].payout, 0);
    assert!(plan.payouts[0].won);
}

/// Invariant loser_bonus = commission - winner_bonus (i.e. 5% du pot en
/// theorie).
#[test]
fn commission_split_is_consistent() {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    for _ in 0..100 {
        let amount: i64 = rng.gen_range(1..=10_000_000);
        let bets = vec![bet(1, "u", "A", amount)];
        let plan = calculate_bet_resolution(&bets, Some("A"), "A", "B");
        let b = plan.fighter_bonus.unwrap();
        let expected_commission = (amount as f64 * 0.15).round() as i64;
        let expected_winner = (amount as f64 * 0.10).round() as i64;
        assert_eq!(b.winner_bonus, expected_winner);
        assert_eq!(b.loser_bonus, expected_commission - expected_winner);
    }
}
