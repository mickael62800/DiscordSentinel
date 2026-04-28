use super::*;

#[test]
fn roundtrip() {
    for c in [CoudeClass::Bourrin, CoudeClass::Agile, CoudeClass::Fourbe, CoudeClass::Tank] {
        assert_eq!(CoudeClass::from_str_lossy(c.as_str()), Some(c));
    }
}

#[test]
fn unknown_returns_none() {
    assert_eq!(CoudeClass::from_str_lossy("ninja"), None);
}

#[test]
fn serde_lowercase() {
    let json = serde_json::to_string(&CoudeClass::Tank).unwrap();
    assert_eq!(json, "\"tank\"");
}
