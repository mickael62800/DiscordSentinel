//! Fonctions pures du domaine economique Coup de Coude (vol, penalites).
//!
//! Extrait depuis `manage_coude_economy_service.rs` (P4 #1 audit) pour
//! decouper la logique de calcul (clamp + validation) des effets de bord
//! (lecture solde, mutation wallet). Ces fonctions ne touchent ni la DB
//! ni le wallet ni l'horloge — elles ne dependent que du `DomainError`.

use crate::domain::errors::DomainError;

/// Resolution d'une demande de vol vs. solde reel de la victime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClampedSteal {
    /// Montant effectivement transferable (`<= amount`, `<= victim_coins`).
    pub stolen: i64,
}

/// Borne le montant demande par le solde de la victime et valide qu'il
/// reste quelque chose a voler.
///
/// Errors :
/// - `ValidationError("Le montant doit etre positif")` si `amount <= 0`
/// - `ValidationError("Impossible de se voler soi-meme")` si `thief_id == victim_id`
/// - `ValidationError("La victime n'a pas de coins a voler")` si solde <= 0
///
/// Comportement : `min(amount, victim_coins)`. Si `victim_coins == 0`, on
/// renvoie l'erreur (pas de creation de coins, pas de vol vide).
pub fn clamp_steal_amount(
    thief_id: &str,
    victim_id: &str,
    amount: i64,
    victim_coins: i64,
) -> Result<ClampedSteal, DomainError> {
    if amount <= 0 {
        return Err(DomainError::ValidationError(
            "Le montant doit etre positif".into(),
        ));
    }
    if thief_id == victim_id {
        return Err(DomainError::ValidationError(
            "Impossible de se voler soi-meme".into(),
        ));
    }
    let stolen = amount.min(victim_coins);
    if stolen <= 0 {
        return Err(DomainError::ValidationError(
            "La victime n'a pas de coins a voler".into(),
        ));
    }
    Ok(ClampedSteal { stolen })
}

/// Borne une penalite de vol echoue par le solde reel du voleur.
/// Comportement legacy : pas d'erreur si penalite > solde, on debit ce
/// qu'on peut (clamp a 0). `amount <= 0` -> 0.
pub fn clamp_steal_fail_penalty(amount: i64, thief_coins: i64) -> i64 {
    if amount <= 0 {
        return 0;
    }
    amount.min(thief_coins).max(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamp_steal_rejects_zero_amount() {
        let err = clamp_steal_amount("a", "b", 0, 100).unwrap_err();
        assert!(matches!(err, DomainError::ValidationError(_)));
    }

    #[test]
    fn clamp_steal_rejects_negative_amount() {
        let err = clamp_steal_amount("a", "b", -10, 100).unwrap_err();
        assert!(matches!(err, DomainError::ValidationError(_)));
    }

    #[test]
    fn clamp_steal_rejects_self_steal() {
        let err = clamp_steal_amount("a", "a", 50, 100).unwrap_err();
        match err {
            DomainError::ValidationError(msg) => assert!(msg.contains("soi-meme")),
            _ => panic!("expected ValidationError"),
        }
    }

    #[test]
    fn clamp_steal_rejects_empty_victim() {
        let err = clamp_steal_amount("a", "b", 50, 0).unwrap_err();
        assert!(matches!(err, DomainError::ValidationError(_)));
        let err = clamp_steal_amount("a", "b", 50, -5).unwrap_err();
        assert!(matches!(err, DomainError::ValidationError(_)));
    }

    #[test]
    fn clamp_steal_caps_at_victim_coins() {
        let r = clamp_steal_amount("a", "b", 500, 100).unwrap();
        assert_eq!(r.stolen, 100);
    }

    #[test]
    fn clamp_steal_takes_amount_when_victim_richer() {
        let r = clamp_steal_amount("a", "b", 50, 1000).unwrap();
        assert_eq!(r.stolen, 50);
    }

    #[test]
    fn fail_penalty_zero_or_negative_amount() {
        assert_eq!(clamp_steal_fail_penalty(0, 100), 0);
        assert_eq!(clamp_steal_fail_penalty(-50, 100), 0);
    }

    #[test]
    fn fail_penalty_caps_at_thief_coins() {
        assert_eq!(clamp_steal_fail_penalty(500, 100), 100);
        assert_eq!(clamp_steal_fail_penalty(500, 0), 0);
    }

    #[test]
    fn fail_penalty_below_balance_takes_amount() {
        assert_eq!(clamp_steal_fail_penalty(50, 1000), 50);
    }
}
