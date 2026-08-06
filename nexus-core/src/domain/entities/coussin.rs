//! Regles pures de Coussin Piégé.
//!
//! Le jeu : on planque un coussin piégé sur le canapé et on s'arrange pour
//! que le voisin s'assoie dessus. Ce qu'on perd n'est pas de la vie, c'est du
//! CONFORT — a zero on ne meurt pas, on se leve du canape.
//!
//! Aucun acces DB/Discord : ces fonctions sont reutilisables par l'API et
//! testables sans infrastructure.

use serde::{Deserialize, Serialize};

pub const MAX_LEVEL: i32 = 25;

/// Les quatre manieres d'occuper un canape.
///
/// Chacune reprend un archetype reel de la bande : celui qui se laisse tomber
/// sans regarder, celui qui bondit, celui qui prepare son coup, et celui qui
/// ne bougera plus de la soiree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PlayerClass {
    /// Se laisse tomber de tout son poids. Fait mal, encaisse mal.
    Ecraseur,
    /// Rebondit d'un accoudoir a l'autre. Vif, mais leger.
    Ressort,
    /// Planque les coussins. Frappe correctement et chipe mieux que les autres.
    Piegeur,
    /// Roule dans la couette et ne bouge plus. Encaisse tout.
    Couette,
}

impl PlayerClass {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "ecraseur" => Some(Self::Ecraseur),
            "ressort" => Some(Self::Ressort),
            "piegeur" => Some(Self::Piegeur),
            "couette" => Some(Self::Couette),
            _ => None,
        }
    }
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ecraseur => "ecraseur",
            Self::Ressort => "ressort",
            Self::Piegeur => "piegeur",
            Self::Couette => "couette",
        }
    }

    /// (Impact, Moelleux) de depart. L'equilibre est celui d'avant la
    /// refonte : seuls les noms changent, pas les chiffres — un joueur ne
    /// doit pas retrouver son personnage affaibli parce que le jeu a change
    /// de titre.
    pub fn base_stats(self) -> (i32, i32) {
        match self {
            Self::Ecraseur => (25, 8),
            Self::Ressort => (12, 18),
            Self::Piegeur => (18, 14),
            Self::Couette => (8, 25),
        }
    }

    pub fn growth(self) -> (i32, i32) {
        match self {
            Self::Ecraseur => (4, 1),
            Self::Ressort => (2, 3),
            Self::Piegeur => (3, 2),
            Self::Couette => (1, 4),
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Ecraseur => "🪑 Écraseur",
            Self::Ressort => "🤸 Ressort",
            Self::Piegeur => "🪡 Piégeur",
            Self::Couette => "🛌 Couette",
        }
    }

    /// Une phrase pour choisir sans lire un tableau de stats.
    pub fn pitch(self) -> &'static str {
        match self {
            Self::Ecraseur => "Tu t'assois sans regarder. Ça fait mal aux deux.",
            Self::Ressort => "Tu rebondis d'un accoudoir a l'autre.",
            Self::Piegeur => "Tu places les coussins. Et tu fouilles sous les autres.",
            Self::Couette => "Tu es dans la couette. Tu n'en sortiras pas.",
        }
    }
}

/// XP cumulée requise pour atteindre un niveau. La progression est plafonnée.
pub fn xp_for_level(level: i32) -> i64 {
    let level = level.clamp(1, MAX_LEVEL) as i64;
    50 * level * (level + 1)
}

pub fn level_for_xp(xp: i64) -> i32 {
    (1..=MAX_LEVEL)
        .rev()
        .find(|&level| xp >= xp_for_level(level))
        .unwrap_or(1)
}

/// Le titre raconte la place gagnee sur le canape, pas un grade militaire :
/// on commence sur l'accoudoir et on finit au milieu, telecommande en main.
pub fn title_for_level(level: i32) -> &'static str {
    match level {
        1..=4 => "Bout d'Accoudoir",
        5..=9 => "Squatteur",
        10..=14 => "Poseur de Coussins",
        15..=19 => "Gardien de la Telecommande",
        20..=24 => "Roi du Canape",
        _ => "Le Canape, c'est Lui",
    }
}

pub fn matchmaking_handicap(first_level: i32, second_level: i32) -> Option<f32> {
    match (first_level - second_level).unsigned_abs() {
        0..=2 => Some(1.0),
        3..=5 => Some(0.8),
        6..=9 => Some(0.6),
        _ => None,
    }
}

pub fn max_hp(defense: i32, class: PlayerClass) -> i32 {
    let base = 100 + defense.max(0) * 10;
    match class {
        PlayerClass::Couette => base * 13 / 10,
        _ => base,
    }
}

/// Degats deterministes hors tirage : les effets aleatoires restent injectes
/// par le cas d'usage afin de garder le domaine reproductible en tests.
pub fn damage(attack: i32, defense: i32, attacker: PlayerClass, defender: PlayerClass) -> i32 {
    let mut value = (10 + attack.max(0) * 4 - defense.max(0) * 2).max(1);
    if attacker == PlayerClass::Ecraseur {
        value = value * 125 / 100;
    }
    if defender == PlayerClass::Couette {
        value = value * 80 / 100;
    }
    value.max(1)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DuelResult {
    pub attacker_won: Option<bool>,
    pub attacker_damage: i32,
    pub defender_damage: i32,
}

/// Resultat d'un combat en plusieurs rounds. Les jets sont fournis par
/// l'appelant afin que le domaine demeure testable et reproductible.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CombatResult {
    pub attacker_won: Option<bool>,
    pub attacker_hp: i32,
    pub defender_hp: i32,
    pub attacker_damage: i32,
    pub defender_damage: i32,
    pub rounds: i32,
}

pub fn resolve_combat(
    attacker_atk: i32,
    attacker_def: i32,
    attacker_class: PlayerClass,
    attacker_level: i32,
    defender_atk: i32,
    defender_def: i32,
    defender_class: PlayerClass,
    defender_level: i32,
    rolls: &[(i32, i32)],
) -> Result<CombatResult, &'static str> {
    let handicap = matchmaking_handicap(attacker_level, defender_level).ok_or("ecart de niveau trop important")?;
    let attacker_is_higher = attacker_level > defender_level;
    let mut attacker_hp = max_hp(attacker_def, attacker_class);
    let mut defender_hp = max_hp(defender_def, defender_class);
    let mut attacker_damage_total = 0;
    let mut defender_damage_total = 0;
    let max_rounds = if attacker_hp + defender_hp < 250 { 3 } else if attacker_hp + defender_hp <= 400 { 5 } else { 7 };
    for (index, (attacker_roll, defender_roll)) in rolls.iter().take(max_rounds as usize).enumerate() {
        let attacker_atk = if attacker_is_higher { (attacker_atk as f32 * handicap) as i32 } else { attacker_atk };
        let defender_atk = if !attacker_is_higher { (defender_atk as f32 * handicap) as i32 } else { defender_atk };
        let mut to_defender = damage(attacker_atk, defender_def, attacker_class, defender_class) * (*attacker_roll).clamp(1, 6);
        let mut to_attacker = damage(defender_atk, attacker_def, defender_class, attacker_class) * (*defender_roll).clamp(1, 6);
        if attacker_class == PlayerClass::Ecraseur && attacker_hp * 100 < max_hp(attacker_def, attacker_class) * 30 { to_defender = to_defender * 125 / 100; }
        if defender_class == PlayerClass::Ecraseur && defender_hp * 100 < max_hp(defender_def, defender_class) * 30 { to_attacker = to_attacker * 125 / 100; }
        if attacker_class == PlayerClass::Couette { to_attacker = (to_attacker - 5).max(1); }
        if defender_class == PlayerClass::Couette { to_defender = (to_defender - 5).max(1); }
        attacker_hp = (attacker_hp - to_attacker).max(0);
        defender_hp = (defender_hp - to_defender).max(0);
        attacker_damage_total += to_defender;
        defender_damage_total += to_attacker;
        if attacker_hp == 0 || defender_hp == 0 || index + 1 == max_rounds as usize { break; }
    }
    let attacker_won = (attacker_hp != defender_hp).then_some(attacker_hp > defender_hp)
        .or_else(|| (attacker_damage_total != defender_damage_total).then_some(attacker_damage_total > defender_damage_total));
    Ok(CombatResult { attacker_won, attacker_hp, defender_hp, attacker_damage: attacker_damage_total, defender_damage: defender_damage_total, rounds: rolls.len().min(max_rounds as usize) as i32 })
}

/// Resolution pure : le caller fournit les jets, ce qui garde le domaine
/// deterministe et permet au cas d'usage d'injecter son alea.
pub fn resolve_duel(
    attacker_atk: i32,
    attacker_def: i32,
    attacker_class: PlayerClass,
    defender_atk: i32,
    defender_def: i32,
    defender_class: PlayerClass,
    attacker_roll: i32,
    defender_roll: i32,
) -> DuelResult {
    let attacker_damage = damage(attacker_atk, defender_def, attacker_class, defender_class)
        * attacker_roll.clamp(1, 6);
    let defender_damage = damage(defender_atk, attacker_def, defender_class, attacker_class)
        * defender_roll.clamp(1, 6);
    DuelResult {
        attacker_won: (attacker_damage != defender_damage)
            .then_some(attacker_damage > defender_damage),
        attacker_damage,
        defender_damage,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn progression_is_bounded() {
        assert_eq!(level_for_xp(xp_for_level(25) + 1), 25);
    }
    #[test]
    fn tank_is_tougher() {
        assert!(max_hp(5, PlayerClass::Couette) > max_hp(5, PlayerClass::Ressort));
    }
    #[test]
    fn damage_never_zero() {
        assert_eq!(damage(0, 999, PlayerClass::Ressort, PlayerClass::Couette), 1);
    }
    #[test]
    fn duel_declares_winner_or_draw() {
        assert_eq!(
            resolve_duel(10, 1, PlayerClass::Ecraseur, 1, 1, PlayerClass::Ressort, 6, 1).attacker_won,
            Some(true)
        );
    }
    #[test]
    fn matchmaking_blocks_gap_of_ten_levels() {
        assert_eq!(matchmaking_handicap(11, 1), None);
    }
    #[test]
    fn multi_round_combat_produces_a_result() {
        let result = resolve_combat(25, 8, PlayerClass::Ecraseur, 1, 8, 25, PlayerClass::Couette, 1, &[(6, 1), (6, 1), (6, 1)]).unwrap();
        assert!(result.attacker_damage > 0);
        assert!(result.rounds >= 1);
    }
}
