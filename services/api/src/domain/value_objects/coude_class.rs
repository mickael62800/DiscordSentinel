use serde::{Deserialize, Serialize};

/// Phase 2 A.3 — Classes de joueur Coude (mappe sur le type Postgres `coude_class`).
///
/// Le `#[sqlx(type_name = "coude_class")]` lie ce type au CREATE TYPE de la
/// migration 103. `rename_all = "lowercase"` aligne les variants Rust avec les
/// labels Postgres ('bourrin', 'agile', 'fourbe', 'tank').
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "lowercase")]
#[sqlx(type_name = "coude_class", rename_all = "lowercase")]
pub enum CoudeClass {
    Bourrin,
    Agile,
    Fourbe,
    Tank,
}

impl CoudeClass {
    pub fn as_str(&self) -> &'static str {
        match self {
            CoudeClass::Bourrin => "bourrin",
            CoudeClass::Agile => "agile",
            CoudeClass::Fourbe => "fourbe",
            CoudeClass::Tank => "tank",
        }
    }

    pub fn from_str_lossy(s: &str) -> Option<Self> {
        match s {
            "bourrin" => Some(Self::Bourrin),
            "agile" => Some(Self::Agile),
            "fourbe" => Some(Self::Fourbe),
            "tank" => Some(Self::Tank),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
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
}
