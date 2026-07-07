//! Regles PURES d'utilisation d'une potion de soin hors combat.
//!
//! Le bareme (montant de heal par potion) vit dans `coude_combat_engine::shop`.
//! La regle anti-gaspillage et le clamp au HP max vivent ici, cote serveur,
//! et non plus dans le bot.

use crate::domain::services::coude::coude_combat_engine::shop;

/// Resultat de l'evaluation d'une tentative d'usage de potion (pre-mutation).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PotionEvaluation {
    /// L'item n'est pas une potion utilisable.
    NotAPotion,
    /// Le joueur est deja a pleine sante.
    AlreadyFull,
    /// Gaspillage : la potion soigne bien plus que le manque de HP.
    Wasteful { hp_missing: i32, heal_amount: i32 },
    /// Usage autorise : `heal_amount` = montant nominal, `actually_healed` =
    /// montant reellement applique apres clamp au HP max, `new_hp` = HP final.
    Ok {
        heal_amount: i32,
        actually_healed: i32,
        new_hp: i32,
    },
}

/// Evalue une tentative d'usage de potion a partir de l'etat HP du joueur.
///
/// Reproduit a l'identique la logique historique du bot :
/// - refus si HP courant >= HP max ;
/// - refus anti-gaspillage si `heal > hp_missing * 3 && heal > 40` ;
/// - sinon `new_hp = min(hp_current + heal, hp_max)`.
pub fn evaluate(item_key: &str, hp_current: i32, hp_max: i32) -> PotionEvaluation {
    if !shop::is_potion(item_key) {
        return PotionEvaluation::NotAPotion;
    }
    let heal_amount = shop::potion_heal_amount(item_key);
    if heal_amount <= 0 {
        return PotionEvaluation::NotAPotion;
    }
    if hp_current >= hp_max {
        return PotionEvaluation::AlreadyFull;
    }
    let hp_missing = hp_max - hp_current;
    if heal_amount > hp_missing * 3 && heal_amount > 40 {
        return PotionEvaluation::Wasteful {
            hp_missing,
            heal_amount,
        };
    }
    let new_hp = (hp_current + heal_amount).min(hp_max);
    PotionEvaluation::Ok {
        heal_amount,
        actually_healed: new_hp - hp_current,
        new_hp,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_potion_rejected() {
        assert_eq!(evaluate("rage", 50, 100), PotionEvaluation::NotAPotion);
    }

    #[test]
    fn already_full_rejected() {
        assert_eq!(
            evaluate("potion_soin", 100, 100),
            PotionEvaluation::AlreadyFull
        );
    }

    #[test]
    fn wasteful_when_heal_far_exceeds_missing() {
        // potion_majeure = +80 ; il manque 10 -> 80 > 30 && 80 > 40 -> refus.
        assert_eq!(
            evaluate("potion_majeure", 90, 100),
            PotionEvaluation::Wasteful {
                hp_missing: 10,
                heal_amount: 80,
            }
        );
    }

    #[test]
    fn small_potion_never_wasteful() {
        // potion_soin = +30 ; meme si le manque est petit, 30 <= 40 -> autorise.
        assert_eq!(
            evaluate("potion_soin", 95, 100),
            PotionEvaluation::Ok {
                heal_amount: 30,
                actually_healed: 5,
                new_hp: 100,
            }
        );
    }

    #[test]
    fn ok_clamps_to_max() {
        assert_eq!(
            evaluate("potion_majeure", 50, 100),
            PotionEvaluation::Ok {
                heal_amount: 80,
                actually_healed: 50,
                new_hp: 100,
            }
        );
    }
}
