//! Validations pures pour la creation d'un combat Coup de Coude.
//!
//! Extrait depuis `manage_coude_combats_service::create` (P4 #2 audit).
//! Ces fonctions ne touchent ni la DB ni les ports — elles encapsulent
//! les regles metier ("mise > 0", "pas de self-duel", "HP min %") pour
//! permettre une couverture unitaire 100% sans I/O.

use crate::domain::entities::NewCoudeCombat;
use crate::domain::errors::DomainError;

/// Valide la coherence basique d'une demande de combat (avant tout I/O).
///
/// Errors :
/// - `ValidationError("La mise doit etre strictement positive")` si `mise <= 0`
/// - `ValidationError("Un joueur ne peut pas se defier lui-meme")` si attacker == defender
pub fn validate_new_combat(new: &NewCoudeCombat) -> Result<(), DomainError> {
    if new.mise <= 0 {
        return Err(DomainError::ValidationError(
            "La mise doit etre strictement positive".into(),
        ));
    }
    if new.attacker_id == new.defender_id {
        return Err(DomainError::ValidationError(
            "Un joueur ne peut pas se defier lui-meme".into(),
        ));
    }
    Ok(())
}

/// Verifie que `hp_current / hp_max >= min_pct%`. Si `min_pct == 0`, OK.
/// `who` est utilise dans le message d'erreur (ex. "L'attaquant").
pub fn check_min_hp_pct(
    who: &str,
    hp_current: i32,
    hp_max: i32,
    min_pct: u64,
) -> Result<(), DomainError> {
    if min_pct == 0 {
        return Ok(());
    }
    let hp_max_u = (hp_max.max(1)) as u64;
    let hp_cur_u = (hp_current.max(0)) as u64;
    let cur_pct = hp_cur_u.saturating_mul(100) / hp_max_u;
    if cur_pct < min_pct {
        return Err(DomainError::ValidationError(format!(
            "{who} n'a pas assez de PV pour combattre : {hp_cur_u}/{hp_max_u} ({cur_pct}%), minimum requis {min_pct}%. Utilise /repos pour te soigner."
        )));
    }
    Ok(())
}

/// Verifie le seuil HP specifique a l'attaque surprise (message different).
pub fn check_surprise_hp_pct(
    hp_current: i32,
    hp_max: i32,
    min_pct: u64,
) -> Result<(), DomainError> {
    if min_pct == 0 {
        return Ok(());
    }
    let hp_max_u = (hp_max.max(1)) as u64;
    let hp_cur_u = (hp_current.max(0)) as u64;
    let cur_pct = hp_cur_u.saturating_mul(100) / hp_max_u;
    if cur_pct < min_pct {
        return Err(DomainError::ValidationError(format!(
            "HP insuffisants pour une attaque surprise : {hp_cur_u}/{hp_max_u} ({cur_pct}%), minimum requis {min_pct}%."
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::NewCoudeCombat;

    fn nc(mise: i64, att: &str, def: &str) -> NewCoudeCombat {
        NewCoudeCombat {
            guild_id: "g".into(),
            channel_id: None,
            attacker_id: att.into(),
            attacker_name: att.into(),
            defender_id: def.into(),
            defender_name: def.into(),
            mise,
            special_attack: None,
        }
    }

    #[test]
    fn rejects_zero_mise() {
        let err = validate_new_combat(&nc(0, "a", "b")).unwrap_err();
        assert!(matches!(err, DomainError::ValidationError(_)));
    }

    #[test]
    fn rejects_negative_mise() {
        let err = validate_new_combat(&nc(-10, "a", "b")).unwrap_err();
        assert!(matches!(err, DomainError::ValidationError(_)));
    }

    #[test]
    fn rejects_self_duel() {
        let err = validate_new_combat(&nc(100, "a", "a")).unwrap_err();
        match err {
            DomainError::ValidationError(msg) => assert!(msg.contains("lui-meme")),
            _ => panic!("expected ValidationError"),
        }
    }

    #[test]
    fn accepts_valid_combat() {
        assert!(validate_new_combat(&nc(100, "a", "b")).is_ok());
    }

    #[test]
    fn check_min_hp_zero_pct_always_ok() {
        assert!(check_min_hp_pct("X", 0, 100, 0).is_ok());
        assert!(check_min_hp_pct("X", 100, 100, 0).is_ok());
    }

    #[test]
    fn check_min_hp_below_threshold_fails() {
        let err = check_min_hp_pct("L'attaquant", 10, 100, 30).unwrap_err();
        match err {
            DomainError::ValidationError(msg) => {
                assert!(msg.contains("L'attaquant"));
                assert!(msg.contains("10/100"));
            }
            _ => panic!("expected ValidationError"),
        }
    }

    #[test]
    fn check_min_hp_at_threshold_ok() {
        assert!(check_min_hp_pct("X", 30, 100, 30).is_ok());
    }

    #[test]
    fn check_min_hp_handles_negative_hp() {
        // hp_current negatif clamp a 0 -> 0% < 30% -> error
        assert!(check_min_hp_pct("X", -5, 100, 30).is_err());
    }

    #[test]
    fn check_min_hp_handles_zero_hp_max() {
        // hp_max=0 clamp a 1 pour eviter div par zero
        assert!(check_min_hp_pct("X", 1, 0, 50).is_ok()); // 1/1=100%
    }

    #[test]
    fn surprise_message_differs() {
        let err = check_surprise_hp_pct(20, 100, 50).unwrap_err();
        match err {
            DomainError::ValidationError(msg) => {
                assert!(msg.contains("attaque surprise"));
                assert!(!msg.contains("/repos"));
            }
            _ => panic!("expected ValidationError"),
        }
    }

    #[test]
    fn surprise_zero_pct_ok() {
        assert!(check_surprise_hp_pct(0, 100, 0).is_ok());
    }
}
