use rand::Rng;

use crate::db::{Player, ServerEvent};
use crate::game::chaos::{self, ChaosEvent};
use crate::game::classes;
use crate::game::progression;

/// Resultat d'un combat resolu.
pub struct CombatResult {
    /// None si match nul (accident_debile)
    pub winner_id: Option<String>,
    pub loser_id: Option<String>,
    pub attacker_roll: i32,
    pub defender_roll: i32,
    pub attacker_damage: i32,
    pub defender_damage: i32,
    pub chaos_event: Option<ChaosEvent>,
    pub coins_won: i64,
    pub coins_lost_by_loser: i64,
    pub stolen_bonus: i64,
    pub message: String,
    /// true si le gagnant est l'underdog (3+ niveaux en dessous)
    pub is_giant_killer: bool,
}

/// Calcule les stats effectives d'un joueur.
fn effective_stats(player: &Player) -> (i32, i32) {
    let class = classes::get_class(&player.class);
    let atk = class.base_atk + (player.level - 1) * class.atk_growth + player.atk;
    let def = class.base_def + (player.level - 1) * class.def_growth + player.def;
    (atk, def)
}

/// Resoud un combat entre deux joueurs.
pub fn resolve_combat(
    attacker: &Player,
    defender: &Player,
    mise: i64,
    special: Option<&str>,
    defender_special: Option<&str>,
    active_events: &[ServerEvent],
) -> CombatResult {
    let mut rng = rand::thread_rng();

    let _atk_class = classes::get_class(&attacker.class);
    let def_class = classes::get_class(&defender.class);

    let (mut atk_effective_atk, _atk_effective_def) = effective_stats(attacker);
    let (_def_effective_atk_stat, def_effective_def) = effective_stats(defender);
    let (def_effective_atk, _) = effective_stats(defender);
    let (_, atk_effective_def) = effective_stats(attacker);

    // Matchmaking handicap
    let (handicap, _blocked) = progression::matchmaking_handicap(attacker.level, defender.level);
    let level_gap = (attacker.level - defender.level).abs();
    let stronger_is_attacker = attacker.level > defender.level;
    let stronger_is_defender = defender.level > attacker.level;

    // Appliquer le handicap au plus fort
    if stronger_is_attacker && level_gap >= 3 {
        atk_effective_atk = (atk_effective_atk as f64 * handicap) as i32;
    }
    let mut def_effective_atk_adj = def_effective_atk;
    if stronger_is_defender && level_gap >= 3 {
        def_effective_atk_adj = (def_effective_atk as f64 * handicap) as i32;
    }

    // Rolls de base (1-100)
    let mut atk_roll: i32 = rng.gen_range(1..=100);
    let mut def_roll: i32 = rng.gen_range(1..=100);

    // Double coup attaquant : lance deux fois et garde le meilleur
    if special == Some("double_coup") {
        let second_roll: i32 = rng.gen_range(1..=100);
        atk_roll = atk_roll.max(second_roll);
    }

    // Double coup defenseur : lance deux fois et garde le meilleur
    if defender_special == Some("double_coup") {
        let second_roll: i32 = rng.gen_range(1..=100);
        def_roll = def_roll.max(second_roll);
    }

    // Rage attaquant : +50% ATK effective
    let mut atk_bonus_flat = 0i32;
    let mut def_bonus_flat = 0i32;

    if special == Some("rage") {
        atk_bonus_flat += 50;
    }
    if defender_special == Some("rage") {
        def_bonus_flat += 50;
    }

    // Coup traitre attaquant : ignore la defense adverse
    let ignore_def_def = special == Some("coup_traitre");
    let ignore_def_atk = defender_special == Some("coup_traitre");

    // Calcul des degats : damage = max(5, (roll * ATK / 50) - enemy_DEF)
    let attacker_atk_total = atk_effective_atk + atk_bonus_flat;
    let defender_atk_total = def_effective_atk_adj + def_bonus_flat;

    let defender_def_for_calc = if ignore_def_def { 0 } else { def_effective_def };
    let attacker_def_for_calc = if ignore_def_atk { 0 } else { atk_effective_def };

    let attacker_damage = 5i32.max((atk_roll * attacker_atk_total / 50) - defender_def_for_calc);
    let defender_damage = 5i32.max((def_roll * defender_atk_total / 50) - attacker_def_for_calc);

    // Esquive de l'agile
    let dodged = if def_class.dodge_chance > 0.0 {
        rng.gen_bool(def_class.dodge_chance)
    } else {
        false
    };

    // Chaos
    let chaos_event = chaos::roll_chaos();

    // Happy hour : gains x2
    let happy_hour = active_events.iter().any(|e| e.event_type == "happy_hour");
    let multiplier = if happy_hour { 2 } else { 1 };

    // Cowardice penalty : laches gagnent 20% de moins
    let coward_penalty_atk = if attacker.cowardice_count >= 5 { 0.80 } else { 1.0 };
    let coward_penalty_def = if defender.cowardice_count >= 5 { 0.80 } else { 1.0 };

    // Explosion defenseur : les deux perdent
    if defender_special == Some("explosion") {
        return CombatResult {
            winner_id: None,
            loser_id: None,
            attacker_roll: atk_roll,
            defender_roll: def_roll,
            attacker_damage,
            defender_damage,
            chaos_event: None,
            coins_won: 0,
            coins_lost_by_loser: mise,
            stolen_bonus: 0,
            message: format!(
                "\u{1f4a3} **EXPLOSION !** <@{}> active une bombe ! Les deux perdent **{} coins** !",
                defender.user_id, mise
            ),
            is_giant_killer: false,
        };
    }

    // Gestion chaos prioritaire
    if let Some(ChaosEvent::AccidentDebile) = chaos_event {
        return CombatResult {
            winner_id: None,
            loser_id: None,
            attacker_roll: atk_roll,
            defender_roll: def_roll,
            attacker_damage,
            defender_damage,
            chaos_event,
            coins_won: 0,
            coins_lost_by_loser: mise,
            stolen_bonus: 0,
            message: format!(
                "\u{1f4a9} **ACCIDENT DEBILE !** Les deux joueurs glissent et perdent {} coins chacun !",
                mise
            ),
            is_giant_killer: false,
        };
    }

    if let Some(ChaosEvent::Glissade) = chaos_event {
        // L'attaquant se frappe => le defenseur gagne
        let gain = (mise as f64 * coward_penalty_def) as i64 * multiplier;
        return CombatResult {
            winner_id: Some(defender.user_id.clone()),
            loser_id: Some(attacker.user_id.clone()),
            attacker_roll: atk_roll,
            defender_roll: def_roll,
            attacker_damage,
            defender_damage,
            chaos_event,
            coins_won: gain,
            coins_lost_by_loser: mise,
            stolen_bonus: 0,
            message: format!(
                "\u{1faa4} **GLISSADE !** <@{}> se frappe lui-meme ! <@{}> empoche {} coins !",
                attacker.user_id, defender.user_id, gain
            ),
            is_giant_killer: false,
        };
    }

    // Determine le gagnant par les degats
    let (winner, loser, winner_damage, loser_damage, winner_coward, _winner_is_attacker) = if dodged {
        (&defender, &attacker, defender_damage, attacker_damage, coward_penalty_def, false)
    } else if let Some(ChaosEvent::EsquiveDivine) = chaos_event {
        (&defender, &attacker, defender_damage, attacker_damage, coward_penalty_def, false)
    } else if attacker_damage > defender_damage {
        (&attacker, &defender, attacker_damage, defender_damage, coward_penalty_atk, true)
    } else if defender_damage > attacker_damage {
        (&defender, &attacker, defender_damage, attacker_damage, coward_penalty_def, false)
    } else {
        // Egalite : match nul, pas de transfert
        return CombatResult {
            winner_id: None,
            loser_id: None,
            attacker_roll: atk_roll,
            defender_roll: def_roll,
            attacker_damage,
            defender_damage,
            chaos_event: None,
            coins_won: 0,
            coins_lost_by_loser: 0,
            stolen_bonus: 0,
            message: format!(
                "\u{1f91d} **EGALITE !** Les deux joueurs ont fait {} degats ! Personne ne perd de coins.",
                attacker_damage
            ),
            is_giant_killer: false,
        };
    };

    // Marge de victoire pour le pourcentage de mise
    let margin = (winner_damage - loser_damage).abs();
    let (mise_pct, margin_label) = if margin < 10 {
        (0.60, "serree")
    } else if margin <= 20 {
        (0.80, "correcte")
    } else {
        (1.00, "nette")
    };

    // Calcul des gains de base avec marge
    let mut gain = (mise as f64 * mise_pct) as i64;
    // Minimum 1 coin
    if gain < 1 {
        gain = 1;
    }

    // Giant killer : underdog wins against someone 3+ levels above
    let is_giant_killer = level_gap >= 3 && winner.level < loser.level;

    // Si underdog gagne contre 3+ niveaux au-dessus → double mise
    if is_giant_killer {
        gain *= 2;
    }

    // Chaos bonus
    if let Some(ChaosEvent::CritiqueSauvage) = chaos_event {
        gain *= 3;
    }
    if let Some(ChaosEvent::Vol) = chaos_event {
        gain = (gain as f64 * 1.20) as i64;
    }

    // Fourbe steal bonus
    let winner_class = classes::get_class(&winner.class);
    let stolen_bonus = if winner_class.steal_bonus > 0.0 {
        (mise as f64 * winner_class.steal_bonus) as i64
    } else {
        0
    };

    gain += stolen_bonus;

    // Cowardice penalty
    gain = (gain as f64 * winner_coward) as i64;

    // Happy hour
    gain *= multiplier;

    // Message construction
    let mut msg = String::new();

    if dodged {
        msg.push_str(&format!(
            "\u{1f3c3} <@{}> esquive avec grace ! ",
            defender.user_id
        ));
    }

    if let Some(ref chaos) = chaos_event {
        msg.push_str(&format!("\n{} **{}** — {} ", chaos.emoji(), chaos.label(), chaos.description()));
    }

    msg.push_str(&format!(
        "\n\u{1f3c6} <@{}> gagne et empoche **{} coins** ! (Degats: {} vs {}) — Victoire **{}**",
        winner.user_id, gain, winner_damage, loser_damage, margin_label
    ));

    if is_giant_killer {
        msg.push_str(&format!(
            "\n\u{1f525} **GIANT KILLER !** <@{}> terrasse un adversaire de {} niveaux au-dessus ! Mise doublee !",
            winner.user_id, level_gap
        ));
    }

    if stolen_bonus > 0 {
        msg.push_str(&format!("\n\u{1f5e1}\u{fe0f} Bonus fourbe : +{} coins voles !", stolen_bonus));
    }

    if happy_hour {
        msg.push_str("\n\u{1f389} **HAPPY HOUR** — Gains doubles !");
    }

    if winner_coward < 1.0 {
        msg.push_str(&format!(
            "\n\u{1f414} Le gagnant est un lache notoire... -20% sur les gains !"
        ));
    }

    if level_gap >= 3 {
        let handicap_pct = ((1.0 - handicap) * 100.0) as i32;
        if stronger_is_attacker {
            msg.push_str(&format!(
                "\n\u{2696}\u{fe0f} Handicap matchmaking : <@{}> a -{}% ATK",
                attacker.user_id, handicap_pct
            ));
        } else if stronger_is_defender {
            msg.push_str(&format!(
                "\n\u{2696}\u{fe0f} Handicap matchmaking : <@{}> a -{}% ATK",
                defender.user_id, handicap_pct
            ));
        }
    }

    CombatResult {
        winner_id: Some(winner.user_id.clone()),
        loser_id: Some(loser.user_id.clone()),
        attacker_roll: atk_roll,
        defender_roll: def_roll,
        attacker_damage,
        defender_damage,
        chaos_event,
        coins_won: gain,
        coins_lost_by_loser: mise,
        stolen_bonus,
        message: msg,
        is_giant_killer,
    }
}
