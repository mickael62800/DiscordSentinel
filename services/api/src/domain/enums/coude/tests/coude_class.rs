use super::*;

#[test]
fn roundtrip() {
    for c in [PlayerClass::Bourrin, PlayerClass::Agile, PlayerClass::Fourbe, PlayerClass::Tank] {
        assert_eq!(PlayerClass::from_str_lossy(c.as_str()), Some(c));
    }
}

#[test]
fn unknown_returns_none() {
    assert_eq!(PlayerClass::from_str_lossy("ninja"), None);
}

#[test]
fn serde_lowercase() {
    let json = serde_json::to_string(&PlayerClass::Tank).unwrap();
    assert_eq!(json, "\"tank\"");
}
