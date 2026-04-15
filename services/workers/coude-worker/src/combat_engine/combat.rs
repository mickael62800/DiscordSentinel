use rand::Rng;

use super::chaos::{self, ChaosEvent};
use super::classes;
use super::progression;
use super::{PlayerLite as Player, ServerEventLite as ServerEvent};

// ══════════════════════════════════════════════════════════════════════
// ── Flavor text ──
// ══════════════════════════════════════════════════════════════════════

const COMBAT_START: &[&str] = &[
    "\u{2694}\u{fe0f} {attaquant} craque ses doigts et regarde {defenseur} droit dans les yeux...",
    "\u{1f514} DING DING ! Le match {attaquant} vs {defenseur} commence !",
    "\u{1f3ac} Les lumieres s'eteignent... le spot s'allume sur {attaquant} et {defenseur} !",
    "\u{1f32a}\u{fe0f} L'arene tremble ! {attaquant} et {defenseur} entrent en scene !",
    "\u{2620}\u{fe0f} Ca va saigner ! {attaquant} defie {defenseur} ! Prenez vos popcorns !",
];

const ROUND_ATTACK: &[&str] = &[
    "\u{1f4a5} {attaquant} envoie un coup de coude VIOLENT ! {degats} degats !",
    "\u{1f44a} {attaquant} frappe avec precision ! {degats} degats !",
    "\u{1f9b5} {attaquant} enchaine avec un coup vicieux ! {degats} degats !",
    "\u{1f4ab} {attaquant} met toute sa force dans ce coup ! {degats} degats !",
    "\u{1f94a} BOUM ! {attaquant} connecte un coup solide ! {degats} degats !",
];

const ROUND_WEAK: &[&str] = &[
    "\u{1f6e1}\u{fe0f} {defenseur} encaisse sans broncher ! {degats} degats seulement.",
    "\u{1f634} {attaquant} tape comme un chatonnet... {degats} degats.",
    "\u{1f9f1} {defenseur} est un MUR. {degats} petits degats.",
    "\u{1f41c} {attaquant} chatouille {defenseur}. {degats} degats.",
];

const COMBAT_KO: &[&str] = &[
    "\u{2620}\u{fe0f} {perdant} s'ecroule ! K.O. ! {gagnant} remporte le combat !",
    "\u{1f480} C'est TERMINE ! {perdant} est a terre ! {gagnant} leve le poing !",
    "\u{1f3c6} {gagnant} acheve {perdant} avec un dernier coup ! VICTOIRE !",
    "\u{1faa6} Repose en paix la dignite de {perdant}. {gagnant} domine !",
];

const COMBAT_TIMEOUT: &[&str] = &[
    "\u{23f0} TEMPS ECOULE ! {gagnant} gagne aux points ({hp_g}% HP vs {hp_p}% HP) !",
    "\u{1f514} Fin du match ! {gagnant} l'emporte avec {hp_g}% de vie restante !",
    "\u{1f4ca} Les juges tranchent : {gagnant} gagne avec {hp_g}% HP contre {hp_p}% !",
];

const COMBAT_DRAW: &[&str] = &[
    "\u{1f91d} Les deux combattants sont a bout de souffle ! Match nul !",
    "\u{2696}\u{fe0f} Impossible de les departager ! Egalite parfaite !",
    "\u{1fae0} Personne ne gagne... personne ne perd... c'est frustrant.",
];

// ══════════════════════════════════════════════════════════════════════
// ── Structs ──
// ══════════════════════════════════════════════════════════════════════

/// Result of a single round.
#[allow(dead_code)]
pub struct RoundResult {
    pub round_number: i32,
    pub attacker_roll: i32,
    pub defender_roll: i32,
    pub attacker_damage: i32,
    pub defender_damage: i32,
    pub attacker_hp_after: i32,
    pub defender_hp_after: i32,
    pub chaos_event: Option<ChaosEvent>,
    pub attacker_passif: Option<String>,
    pub defender_passif: Option<String>,
    pub message: String,
}

/// Full combat result.
#[allow(dead_code)]
pub struct CombatResult {
    pub winner_id: Option<String>,
    pub loser_id: Option<String>,
    pub rounds: Vec<RoundResult>,
    pub total_rounds: i32,
    pub attacker_hp_final: i32,
    pub defender_hp_final: i32,
    pub attacker_hp_max: i32,
    pub defender_hp_max: i32,
    pub chaos_events_count: i32,
    pub coins_won: i64,
    pub coins_lost_by_loser: i64,
    pub stolen_bonus: i64,
    pub vol_coins: i64,
    pub message: String,
    pub is_giant_killer: bool,
    pub attacker_class_revealed: Option<String>,
    pub defender_class_revealed: Option<String>,
}

// ══════════════════════════════════════════════════════════════════════
// ── Helpers ──
// ══════════════════════════════════════════════════════════════════════

fn pick_random<'a>(templates: &'a [&'a str]) -> &'a str {
    let mut rng = rand::thread_rng();
    let idx = rng.gen_range(0..templates.len());
    templates[idx]
}

fn fmt_template(template: &str, replacements: &[(&str, &str)]) -> String {
    let mut s = template.to_string();
    for (key, val) in replacements {
        s = s.replace(key, val);
    }
    s
}

/// Calcule les stats effectives d'un joueur (ATK, DEF).
pub fn effective_stats(player: &Player) -> (i32, i32) {
    let class = classes::get_class(player.class.as_deref().unwrap_or("bourrin"));
    let atk = class.base_atk + (player.level - 1) * class.atk_growth + player.atk;
    let def = class.base_def + (player.level - 1) * class.def_growth + player.def;
    (atk, def)
}

/// Calculate maximum HP for a player.
pub fn calculate_hp_max(player: &Player) -> i32 {
    let (_, def) = effective_stats(player);
    100 + def * 2
}

/// Calculate damage for one hit.
fn calc_damage(roll: i32, atk: i32, enemy_def: i32) -> i32 {
    let degats_bruts = (roll as f64 * atk as f64) / 10.0;
    let reduction = enemy_def as f64 / (enemy_def as f64 + 50.0);
    let degats = degats_bruts * (1.0 - reduction);
    3i32.max(degats as i32)
}

/// Determine max rounds from combined HP.
fn max_rounds(combined_hp: i32) -> i32 {
    if combined_hp < 250 {
        3
    } else if combined_hp <= 400 {
        5
    } else {
        7
    }
}

// ══════════════════════════════════════════════════════════════════════
// ── Main combat function ──
// ══════════════════════════════════════════════════════════════════════

pub fn resolve_combat(
    attacker: &Player,
    defender: &Player,
    attacker_current_hp: i32,
    defender_current_hp: i32,
    mise: i64,
    special: Option<&str>,
    defender_special: Option<&str>,
    active_events: &[ServerEvent],
) -> CombatResult {
    let mut rng = rand::thread_rng();

    let atk_class = classes::get_class(attacker.class.as_deref().unwrap_or("bourrin"));
    let def_class = classes::get_class(defender.class.as_deref().unwrap_or("bourrin"));

    // Base effective stats
    let (mut atk_atk, mut atk_def) = effective_stats(attacker);
    let (mut def_atk, mut def_def) = effective_stats(defender);

    // Matchmaking handicap
    let (handicap, _blocked) = progression::matchmaking_handicap(attacker.level, defender.level);
    let level_gap = (attacker.level - defender.level).abs();
    let stronger_is_attacker = attacker.level > defender.level;
    let stronger_is_defender = defender.level > attacker.level;

    if stronger_is_attacker && level_gap >= 3 {
        atk_atk = (atk_atk as f64 * handicap) as i32;
    }
    if stronger_is_defender && level_gap >= 3 {
        def_atk = (def_atk as f64 * handicap) as i32;
    }

    // ── Item effects (global, applied once) ──

    // Rage: +50% ATK, -30% DEF
    if special == Some("rage") {
        atk_atk = (atk_atk as f64 * 1.5) as i32;
        atk_def = (atk_def as f64 * 0.7) as i32;
    }
    if defender_special == Some("rage") {
        def_atk = (def_atk as f64 * 1.5) as i32;
        def_def = (def_def as f64 * 0.7) as i32;
    }

    // Coup traitre: reduce enemy DEF by 50%
    if special == Some("coup_traitre") {
        def_def = (def_def as f64 * 0.5) as i32;
    }
    if defender_special == Some("coup_traitre") {
        atk_def = (atk_def as f64 * 0.5) as i32;
    }

    // Bouclier: +20% DEF
    if special == Some("bouclier") {
        atk_def = (atk_def as f64 * 1.2) as i32;
    }
    if defender_special == Some("bouclier") {
        def_def = (def_def as f64 * 1.2) as i32;
    }

    // Recalculate HP max with modified DEF
    let atk_hp_max = 100 + atk_def * 2;
    let def_hp_max = 100 + def_def * 2;

    let mut atk_hp = attacker_current_hp.min(atk_hp_max);
    let mut def_hp = defender_current_hp.min(def_hp_max);

    // Has double_coup?
    let atk_double = special == Some("double_coup");
    let def_double = defender_special == Some("double_coup");

    // Has poison?
    let atk_poison = special == Some("poison");
    let def_poison = defender_special == Some("poison");

    // Happy hour
    let happy_hour = active_events.iter().any(|e| e.event_type == "happy_hour");
    let multiplier = if happy_hour { 2 } else { 1 };

    // Cowardice penalty
    let coward_penalty_atk = if attacker.cowardice_count >= 5 { 0.80 } else { 1.0 };
    let coward_penalty_def = if defender.cowardice_count >= 5 { 0.80 } else { 1.0 };

    let atk_name = format!("<@{}>", attacker.user_id);
    let def_name = format!("<@{}>", defender.user_id);

    // ── Explosion: early exit, both lose 50% of mise ──
    if defender_special == Some("explosion") {
        let lost = (mise as f64 * 0.5) as i64;
        return CombatResult {
            winner_id: None,
            loser_id: None,
            rounds: vec![],
            total_rounds: 0,
            attacker_hp_final: atk_hp,
            defender_hp_final: def_hp,
            attacker_hp_max: atk_hp_max,
            defender_hp_max: def_hp_max,
            chaos_events_count: 0,
            coins_won: 0,
            coins_lost_by_loser: lost,
            stolen_bonus: 0,
            vol_coins: 0,
            message: format!(
                "\u{1f4a3} **EXPLOSION !** {} active une bombe ! Les deux perdent **{} coins** !",
                def_name, lost
            ),
            is_giant_killer: false,
            attacker_class_revealed: None,
            defender_class_revealed: None,
        };
    }

    // ── Combat start message ──
    let start_msg = fmt_template(
        pick_random(COMBAT_START),
        &[("{attaquant}", &atk_name), ("{defenseur}", &def_name)],
    );

    let rounds_max = max_rounds(atk_hp_max + def_hp_max);
    let mut rounds: Vec<RoundResult> = Vec::new();
    let mut chaos_count = 0;
    let mut vol_coins_total: i64 = 0;
    let mut attacker_class_revealed: Option<String> = None;
    let mut defender_class_revealed: Option<String> = None;

    // ══════════════════════════════════════════════════════════════════
    // ── Combat loop ──
    // ══════════════════════════════════════════════════════════════════

    for round_num in 1..=rounds_max {
        let mut round_msg = format!("**--- Round {} ---**\n", round_num);
        let mut atk_passif: Option<String> = None;
        let mut def_passif: Option<String> = None;

        // ── Rolls ──
        let mut atk_roll: i32 = rng.gen_range(1..=20);
        let mut def_roll: i32 = rng.gen_range(1..=20);

        if atk_double {
            let second: i32 = rng.gen_range(1..=20);
            atk_roll = atk_roll.max(second);
        }
        if def_double {
            let second: i32 = rng.gen_range(1..=20);
            def_roll = def_roll.max(second);
        }

        // ── Effective ATK this round (class passives) ──
        let mut atk_atk_round = atk_atk;
        let mut def_atk_round = def_atk;

        // Bourrin: Berserker — ATK +25% when HP <= 30%
        // (inclusif pour eviter l'off-by-one : a exactement 30% le passif
        // s'active, coherent avec le 50% / 25% inclusifs des autres seuils).
        let atk_berserker_threshold = (atk_hp_max as f64 * 0.3).ceil() as i32;
        let def_berserker_threshold = (def_hp_max as f64 * 0.3).ceil() as i32;
        if atk_class.name == "bourrin" && atk_hp <= atk_berserker_threshold {
            atk_atk_round = (atk_atk_round as f64 * 1.25) as i32;
            atk_passif = Some("berserker".to_string());
            attacker_class_revealed = Some("bourrin".to_string());
        }
        if def_class.name == "bourrin" && def_hp <= def_berserker_threshold {
            def_atk_round = (def_atk_round as f64 * 1.25) as i32;
            def_passif = Some("berserker".to_string());
            defender_class_revealed = Some("bourrin".to_string());
        }

        // ── Base damage calc ──
        let mut atk_dmg = calc_damage(atk_roll, atk_atk_round, def_def);
        let mut def_dmg = calc_damage(def_roll, def_atk_round, atk_def);

        // ── Tank: Blindage — reduce damage taken by 5 flat (after formula) ──
        // Exception : Tank vs Tank → les deux blindages s'annulent sinon on se
        // retrouve avec 1 dmg/round chacun et un timeout garanti (draw/accident).
        let tank_mirror = atk_class.name == "tank" && def_class.name == "tank";
        if !tank_mirror {
            if atk_class.name == "tank" {
                def_dmg = (def_dmg - 5).max(1);
                if atk_passif.is_none() {
                    atk_passif = Some("blindage".to_string());
                }
                attacker_class_revealed = Some("tank".to_string());
            }
            if def_class.name == "tank" {
                atk_dmg = (atk_dmg - 5).max(1);
                if def_passif.is_none() {
                    def_passif = Some("blindage".to_string());
                }
                defender_class_revealed = Some("tank".to_string());
            }
        } else {
            // Mirror match : on revele quand meme les classes pour la tension
            // mais aucun passif ne s'applique.
            attacker_class_revealed = Some("tank".to_string());
            defender_class_revealed = Some("tank".to_string());
            if atk_passif.is_none() {
                atk_passif = Some("tank_mirror".to_string());
            }
            if def_passif.is_none() {
                def_passif = Some("tank_mirror".to_string());
            }
        }

        // ── Agile: Esquive — dodge chance per round ──
        let atk_dodged = if atk_class.dodge_chance > 0.0 {
            rng.gen_bool(atk_class.dodge_chance.min(1.0))
        } else {
            false
        };
        let def_dodged = if def_class.dodge_chance > 0.0 {
            rng.gen_bool(def_class.dodge_chance.min(1.0))
        } else {
            false
        };

        if atk_dodged {
            def_dmg = 0;
            atk_passif = Some("esquive".to_string());
            attacker_class_revealed = Some("agile".to_string());
            round_msg.push_str(&format!(
                "\u{1f3c3} {} esquive le coup !\n", atk_name
            ));
        }
        if def_dodged {
            atk_dmg = 0;
            def_passif = Some("esquive".to_string());
            defender_class_revealed = Some("agile".to_string());
            round_msg.push_str(&format!(
                "\u{1f3c3} {} esquive le coup !\n", def_name
            ));
        }

        // ── Chaos event (8% per round) ──
        let chaos_event = chaos::roll_chaos();
        // We use roll_chaos which has 18% total; for now we treat it as-is
        // (will be adjusted to 8% per-round in chaos.rs separately)

        if let Some(ref ce) = chaos_event {
            chaos_count += 1;
            match ce {
                ChaosEvent::CritiqueSauvage => {
                    // x2 damage for whoever deals more this round
                    if atk_dmg >= def_dmg {
                        atk_dmg *= 2;
                        round_msg.push_str(&format!(
                            "{} **{}** — {} inflige x2 degats ce round !\n",
                            ce.emoji(), ce.label(), atk_name
                        ));
                    } else {
                        def_dmg *= 2;
                        round_msg.push_str(&format!(
                            "{} **{}** — {} inflige x2 degats ce round !\n",
                            ce.emoji(), ce.label(), def_name
                        ));
                    }
                }
                ChaosEvent::EsquiveDivine => {
                    // Defender dodges and counter-attacks with +50% damage
                    atk_dmg = 0;
                    def_dmg = (def_dmg as f64 * 1.5) as i32;
                    round_msg.push_str(&format!(
                        "{} **{}** — {} esquive et contre-attaque a +50% !\n",
                        ce.emoji(), ce.label(), def_name
                    ));
                }
                ChaosEvent::AccidentDebile => {
                    // Both take 10% of their max HP
                    let atk_self_dmg = (atk_hp_max as f64 * 0.1) as i32;
                    let def_self_dmg = (def_hp_max as f64 * 0.1) as i32;
                    atk_hp -= atk_self_dmg;
                    def_hp -= def_self_dmg;
                    round_msg.push_str(&format!(
                        "{} **{}** — Les deux prennent des degats ! ({} et {} HP perdus)\n",
                        ce.emoji(), ce.label(), atk_self_dmg, def_self_dmg
                    ));
                }
                ChaosEvent::Glissade => {
                    // Attacker hits himself
                    atk_hp -= atk_dmg;
                    atk_dmg = 0;
                    round_msg.push_str(&format!(
                        "{} **{}** — {} se frappe lui-meme !\n",
                        ce.emoji(), ce.label(), atk_name
                    ));
                }
                ChaosEvent::Vol => {
                    // Winner of this round steals 5% of opponent's coins
                    let steal_amount = (mise as f64 * 0.05) as i64;
                    vol_coins_total += steal_amount;
                    round_msg.push_str(&format!(
                        "{} **{}** — {} coins voles en bonus !\n",
                        ce.emoji(), ce.label(), steal_amount
                    ));
                }
            }
        }

        // ── Apply poison ──
        if atk_poison {
            def_hp -= 5;
            round_msg.push_str(&format!(
                "\u{2620}\u{fe0f} {} subit 5 degats de poison !\n", def_name
            ));
        }
        if def_poison {
            atk_hp -= 5;
            round_msg.push_str(&format!(
                "\u{2620}\u{fe0f} {} subit 5 degats de poison !\n", atk_name
            ));
        }

        // ── Apply damage simultaneously ──
        def_hp -= atk_dmg;
        atk_hp -= def_dmg;

        // ── Fourbe: Vampirisme — heal 10% of damage dealt ──
        if atk_class.name == "fourbe" && atk_dmg > 0 {
            let heal = (atk_dmg as f64 * 0.1) as i32;
            atk_hp = (atk_hp + heal).min(atk_hp_max);
            if atk_passif.is_none() {
                atk_passif = Some("vampirisme".to_string());
            }
            attacker_class_revealed = Some("fourbe".to_string());
            if heal > 0 {
                round_msg.push_str(&format!(
                    "\u{1fa78} {} se soigne de {} HP (vampirisme) !\n", atk_name, heal
                ));
            }
        }
        if def_class.name == "fourbe" && def_dmg > 0 {
            let heal = (def_dmg as f64 * 0.1) as i32;
            def_hp = (def_hp + heal).min(def_hp_max);
            if def_passif.is_none() {
                def_passif = Some("vampirisme".to_string());
            }
            defender_class_revealed = Some("fourbe".to_string());
            if heal > 0 {
                round_msg.push_str(&format!(
                    "\u{1fa78} {} se soigne de {} HP (vampirisme) !\n", def_name, heal
                ));
            }
        }

        // Clamp HP to 0 minimum
        atk_hp = atk_hp.max(0);
        def_hp = def_hp.max(0);

        // ── Round flavor text ──
        if atk_dmg > 0 {
            let templates = if atk_dmg < 5 { ROUND_WEAK } else { ROUND_ATTACK };
            let txt = fmt_template(
                pick_random(templates),
                &[
                    ("{attaquant}", &atk_name),
                    ("{defenseur}", &def_name),
                    ("{degats}", &atk_dmg.to_string()),
                ],
            );
            round_msg.push_str(&txt);
            round_msg.push('\n');
        }
        if def_dmg > 0 {
            let templates = if def_dmg < 5 { ROUND_WEAK } else { ROUND_ATTACK };
            let txt = fmt_template(
                pick_random(templates),
                &[
                    ("{attaquant}", &def_name),
                    ("{defenseur}", &atk_name),
                    ("{degats}", &def_dmg.to_string()),
                ],
            );
            round_msg.push_str(&txt);
            round_msg.push('\n');
        }

        round_msg.push_str(&format!(
            "\u{2764}\u{fe0f} {} : {}/{} HP | {} : {}/{} HP",
            atk_name, atk_hp, atk_hp_max, def_name, def_hp, def_hp_max
        ));

        rounds.push(RoundResult {
            round_number: round_num,
            attacker_roll: atk_roll,
            defender_roll: def_roll,
            attacker_damage: atk_dmg,
            defender_damage: def_dmg,
            attacker_hp_after: atk_hp,
            defender_hp_after: def_hp,
            chaos_event,
            attacker_passif: atk_passif,
            defender_passif: def_passif,
            message: round_msg,
        });

        // ── Check KO ──
        if atk_hp <= 0 || def_hp <= 0 {
            break;
        }
    }

    // ══════════════════════════════════════════════════════════════════
    // ── Determine winner ──
    // ══════════════════════════════════════════════════════════════════

    let total_rounds = rounds.len() as i32;
    let atk_hp_pct = if atk_hp_max > 0 {
        (atk_hp as f64 / atk_hp_max as f64 * 100.0) as i32
    } else {
        0
    };
    let def_hp_pct = if def_hp_max > 0 {
        (def_hp as f64 / def_hp_max as f64 * 100.0) as i32
    } else {
        0
    };

    let ko = atk_hp <= 0 || def_hp <= 0;

    // Determine winner/loser
    let (winner_id, loser_id, winner_pct, loser_pct, winner_coward) = if atk_hp <= 0 && def_hp <= 0 {
        // Both KO at same time -> compare who had more HP% before last round
        // Treat as draw
        (None, None, atk_hp_pct, def_hp_pct, 1.0)
    } else if def_hp <= 0 {
        (
            Some(attacker.user_id.clone()),
            Some(defender.user_id.clone()),
            atk_hp_pct,
            def_hp_pct,
            coward_penalty_atk,
        )
    } else if atk_hp <= 0 {
        (
            Some(defender.user_id.clone()),
            Some(attacker.user_id.clone()),
            def_hp_pct,
            atk_hp_pct,
            coward_penalty_def,
        )
    } else if atk_hp_pct > def_hp_pct {
        // Timeout: highest HP% wins
        (
            Some(attacker.user_id.clone()),
            Some(defender.user_id.clone()),
            atk_hp_pct,
            def_hp_pct,
            coward_penalty_atk,
        )
    } else if def_hp_pct > atk_hp_pct {
        (
            Some(defender.user_id.clone()),
            Some(attacker.user_id.clone()),
            def_hp_pct,
            atk_hp_pct,
            coward_penalty_def,
        )
    } else {
        // Equal HP% -> draw
        (None, None, atk_hp_pct, def_hp_pct, 1.0)
    };

    let is_draw = winner_id.is_none();

    // ── Gains calculation based on HP% margin ──
    let hp_diff = (atk_hp_pct - def_hp_pct).abs();
    let (win_pct, lose_pct) = if hp_diff < 15 {
        (0.70, 0.60)
    } else if hp_diff <= 40 {
        (0.85, 0.80)
    } else {
        (1.00, 1.00)
    };

    if is_draw {
        // ── Draw path ──
        let mut final_msg = format!("{}\n\n", start_msg);
        for r in &rounds {
            final_msg.push_str(&r.message);
            final_msg.push_str("\n\n");
        }
        final_msg.push_str(pick_random(COMBAT_DRAW));

        return CombatResult {
            winner_id: None,
            loser_id: None,
            rounds,
            total_rounds,
            attacker_hp_final: atk_hp,
            defender_hp_final: def_hp,
            attacker_hp_max: atk_hp_max,
            defender_hp_max: def_hp_max,
            chaos_events_count: chaos_count,
            coins_won: 0,
            coins_lost_by_loser: 0,
            stolen_bonus: 0,
            vol_coins: vol_coins_total,
            message: final_msg,
            is_giant_killer: false,
            attacker_class_revealed,
            defender_class_revealed,
        };
    }

    // ── Winner path ──
    // Tous les calculs utilisent saturating_* pour eviter overflow/wrap sur
    // des mises proches de i64::MAX. Les coins sont clamp a [1, i64::MAX].
    let mise_f = mise as f64;
    let mut coins_won: i64 = ((mise_f * win_pct).clamp(0.0, i64::MAX as f64)) as i64;
    let coins_lost: i64 = ((mise_f * lose_pct).clamp(0.0, i64::MAX as f64)) as i64;

    if coins_won < 1 {
        coins_won = 1;
    }

    // Giant killer: 3+ level gap underdog winning
    let is_giant = if let (Some(ref wid), Some(ref lid)) = (&winner_id, &loser_id) {
        let winner_lvl = if *wid == attacker.user_id { attacker.level } else { defender.level };
        let loser_lvl = if *lid == attacker.user_id { attacker.level } else { defender.level };
        level_gap >= 3 && winner_lvl < loser_lvl
    } else {
        false
    };

    // Fourbe steal bonus
    let winner_class_name = if winner_id.as_deref() == Some(&attacker.user_id) {
        atk_class.name
    } else {
        def_class.name
    };
    let w_class = classes::get_class(winner_class_name);
    let stolen_bonus_val: i64 = if w_class.steal_bonus > 0.0 {
        ((mise_f * w_class.steal_bonus).clamp(0.0, i64::MAX as f64)) as i64
    } else {
        0
    };
    coins_won = coins_won.saturating_add(stolen_bonus_val);

    // Cowardice penalty
    coins_won = ((coins_won as f64 * winner_coward).clamp(0.0, i64::MAX as f64)) as i64;

    // Happy hour (multiplier est un i64 entier, typiquement 1 ou 2).
    coins_won = coins_won.saturating_mul(multiplier);

    // ── Build final message ──
    let winner_name = if winner_id.as_deref() == Some(&attacker.user_id) {
        &atk_name
    } else {
        &def_name
    };
    let loser_name = if winner_id.as_deref() == Some(&attacker.user_id) {
        &def_name
    } else {
        &atk_name
    };

    let mut final_msg = format!("{}\n\n", start_msg);

    // Append round summaries
    for r in &rounds {
        final_msg.push_str(&r.message);
        final_msg.push_str("\n\n");
    }

    // Ending
    if ko {
        let ko_txt = fmt_template(
            pick_random(COMBAT_KO),
            &[("{perdant}", loser_name), ("{gagnant}", winner_name)],
        );
        final_msg.push_str(&ko_txt);
    } else {
        let timeout_txt = fmt_template(
            pick_random(COMBAT_TIMEOUT),
            &[
                ("{gagnant}", winner_name),
                ("{hp_g}", &winner_pct.to_string()),
                ("{hp_p}", &loser_pct.to_string()),
            ],
        );
        final_msg.push_str(&timeout_txt);
    }

    final_msg.push_str(&format!(
        "\n\u{1f4b0} {} empoche **{} coins** ! {} perd **{} coins** !",
        winner_name, coins_won, loser_name, coins_lost
    ));

    if is_giant {
        final_msg.push_str(&format!(
            "\n\u{1f525} **GIANT KILLER !** {} terrasse un adversaire de {} niveaux au-dessus ! +15 XP bonus !",
            winner_name, level_gap
        ));
    }

    if vol_coins_total > 0 {
        final_msg.push_str(&format!(
            "\n\u{1f4b0} Vol a la Tire total : +{} coins voles !",
            vol_coins_total
        ));
    }

    if stolen_bonus_val > 0 {
        final_msg.push_str(&format!(
            "\n\u{1f5e1}\u{fe0f} Bonus fourbe : +{} coins voles !",
            stolen_bonus_val
        ));
    }

    if happy_hour {
        final_msg.push_str("\n\u{1f389} **HAPPY HOUR** — Gains doubles !");
    }

    if winner_coward < 1.0 {
        final_msg.push_str("\n\u{1f414} Le gagnant est un lache notoire... -20% sur les gains !");
    }

    if level_gap >= 3 {
        let handicap_pct = ((1.0 - handicap) * 100.0) as i32;
        if stronger_is_attacker {
            final_msg.push_str(&format!(
                "\n\u{2696}\u{fe0f} Handicap matchmaking : {} a -{}% ATK",
                atk_name, handicap_pct
            ));
        } else if stronger_is_defender {
            final_msg.push_str(&format!(
                "\n\u{2696}\u{fe0f} Handicap matchmaking : {} a -{}% ATK",
                def_name, handicap_pct
            ));
        }
    }

    CombatResult {
        winner_id,
        loser_id,
        rounds,
        total_rounds,
        attacker_hp_final: atk_hp,
        defender_hp_final: def_hp,
        attacker_hp_max: atk_hp_max,
        defender_hp_max: def_hp_max,
        chaos_events_count: chaos_count,
        coins_won,
        coins_lost_by_loser: coins_lost,
        stolen_bonus: 0,
        vol_coins: vol_coins_total,
        message: final_msg,
        is_giant_killer: is_giant,
        attacker_class_revealed,
        defender_class_revealed,
    }
}
