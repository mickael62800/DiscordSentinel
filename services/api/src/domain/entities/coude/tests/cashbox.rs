use super::*;

#[test]
fn cashbox_source_as_str_all_variants() {
    assert_eq!(CashboxSource::ShopPurchase.as_str(), "shop_purchase");
    assert_eq!(CashboxSource::InsurancePurchase.as_str(), "insurance_purchase");
    assert_eq!(CashboxSource::ProtectionPurchase.as_str(), "protection_purchase");
    assert_eq!(CashboxSource::BoostPurchase.as_str(), "boost_purchase");
    assert_eq!(CashboxSource::ClassChangeCost.as_str(), "class_change");
    assert_eq!(CashboxSource::ResetStatsCost.as_str(), "reset_stats");
    assert_eq!(CashboxSource::DonationTax.as_str(), "donation_tax");
    assert_eq!(CashboxSource::CowardicePenalty.as_str(), "cowardice_penalty");
    assert_eq!(CashboxSource::BetCommission.as_str(), "bet_commission");
}

#[test]
fn cashbox_source_labels_are_snake_case() {
    let all = [
        CashboxSource::ShopPurchase,
        CashboxSource::InsurancePurchase,
        CashboxSource::ProtectionPurchase,
        CashboxSource::BoostPurchase,
        CashboxSource::ClassChangeCost,
        CashboxSource::ResetStatsCost,
        CashboxSource::DonationTax,
        CashboxSource::CowardicePenalty,
        CashboxSource::BetCommission,
    ];
    for s in all {
        let label = s.as_str();
        assert!(!label.is_empty(), "{:?} produces empty label", s);
        assert!(
            label.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
            "{:?} not snake_case: {}", s, label
        );
    }
}

#[test]
fn cashbox_source_labels_unique() {
    let labels = [
        CashboxSource::ShopPurchase.as_str(),
        CashboxSource::InsurancePurchase.as_str(),
        CashboxSource::ProtectionPurchase.as_str(),
        CashboxSource::BoostPurchase.as_str(),
        CashboxSource::ClassChangeCost.as_str(),
        CashboxSource::ResetStatsCost.as_str(),
        CashboxSource::DonationTax.as_str(),
        CashboxSource::CowardicePenalty.as_str(),
        CashboxSource::BetCommission.as_str(),
    ];
    let set: std::collections::HashSet<_> = labels.iter().collect();
    assert_eq!(set.len(), labels.len(), "duplicate cashbox source labels");
}
