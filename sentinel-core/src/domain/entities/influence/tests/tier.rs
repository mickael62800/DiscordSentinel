use super::*;

#[test]
fn to_tier_couvre_tous_les_paliers() {
    let t = TierThresholds::default(); // [100, 500, 2000, 10000, 50000]
    assert_eq!(to_tier(0, &t), NarrativeTier::Negligeable);
    assert_eq!(to_tier(99, &t), NarrativeTier::Negligeable);
    assert_eq!(to_tier(100, &t), NarrativeTier::Faible);
    assert_eq!(to_tier(499, &t), NarrativeTier::Faible);
    assert_eq!(to_tier(500, &t), NarrativeTier::Moyenne);
    assert_eq!(to_tier(1_999, &t), NarrativeTier::Moyenne);
    assert_eq!(to_tier(2_000, &t), NarrativeTier::Elevee);
    assert_eq!(to_tier(10_000, &t), NarrativeTier::TresElevee);
    assert_eq!(to_tier(50_000, &t), NarrativeTier::Legendaire);
    assert_eq!(to_tier(i64::MAX, &t), NarrativeTier::Legendaire);
}

#[test]
fn to_tier_valeur_negative_est_negligeable() {
    let t = TierThresholds::default();
    assert_eq!(to_tier(-100, &t), NarrativeTier::Negligeable);
}

#[test]
fn reputation_tier_centree_sur_zero() {
    assert_eq!(to_reputation_tier(-1_000), ReputationTier::Desastreuse);
    assert_eq!(to_reputation_tier(-500), ReputationTier::Desastreuse);
    assert_eq!(to_reputation_tier(-100), ReputationTier::Mauvaise);
    assert_eq!(to_reputation_tier(0), ReputationTier::Neutre);
    assert_eq!(to_reputation_tier(50), ReputationTier::Neutre);
    assert_eq!(to_reputation_tier(100), ReputationTier::Bonne);
    assert_eq!(to_reputation_tier(500), ReputationTier::Excellente);
    assert_eq!(to_reputation_tier(10_000), ReputationTier::Excellente);
}

#[test]
fn stars_toujours_cinq_glyphes() {
    for tier in [
        NarrativeTier::Negligeable,
        NarrativeTier::Faible,
        NarrativeTier::Moyenne,
        NarrativeTier::Elevee,
        NarrativeTier::TresElevee,
        NarrativeTier::Legendaire,
    ] {
        assert_eq!(tier.stars().chars().count(), 5);
    }
}
