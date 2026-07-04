use super::*;

fn rates() -> ConversionRates {
    ConversionRates {
        money_to_reputation: 10,
        reputation_to_influence: 5,
        money_to_information: 20,
    }
}

#[test]
fn convert_arrondit_et_ne_debite_que_le_convertible() {
    // 95 argent / 10 = 9 reputation, on ne debite que 90 (5 restent).
    let r = convert(ConversionKind::MoneyToReputation, 95, 1000, &rates()).unwrap();
    assert_eq!(r.gained, 9);
    assert_eq!(r.spent, 90);
}

#[test]
fn convert_montant_exact() {
    let r = convert(ConversionKind::ReputationToInfluence, 50, 50, &rates()).unwrap();
    assert_eq!(r.gained, 10);
    assert_eq!(r.spent, 50);
}

#[test]
fn convert_sous_le_minimum() {
    let e = convert(ConversionKind::MoneyToInformation, 15, 1000, &rates()).unwrap_err();
    assert_eq!(e, ConversionError::BelowMinimum { cost: 20 });
}

#[test]
fn convert_solde_insuffisant() {
    // budget 100 -> voudrait debiter 100, mais dispo 40.
    let e = convert(ConversionKind::MoneyToReputation, 100, 40, &rates()).unwrap_err();
    assert_eq!(
        e,
        ConversionError::Insufficient {
            available: 40,
            needed: 100
        }
    );
}

#[test]
fn convert_taux_invalide() {
    let mut r = rates();
    r.money_to_reputation = 0;
    let e = convert(ConversionKind::MoneyToReputation, 100, 1000, &r).unwrap_err();
    assert_eq!(e, ConversionError::InvalidRate);
}
