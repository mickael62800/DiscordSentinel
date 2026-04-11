use serde::{Deserialize, Serialize};
use std::fmt;

/// Types d'actions de moderation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModerationActionType {
    Warn,
    #[serde(rename = "mute_temp")]
    MuteTemp,
    #[serde(rename = "mute_permanent")]
    MutePermanent,
    Unmute,
    #[serde(rename = "ban_temp")]
    BanTemp,
    #[serde(rename = "ban_permanent")]
    BanPermanent,
    Unban,
    Call,
}

impl ModerationActionType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Warn => "warn",
            Self::MuteTemp => "mute_temp",
            Self::MutePermanent => "mute_permanent",
            Self::Unmute => "unmute",
            Self::BanTemp => "ban_temp",
            Self::BanPermanent => "ban_permanent",
            Self::Unban => "unban",
            Self::Call => "call",
        }
    }

    /// Parse un type d'action depuis une string. Retourne `None` si invalide.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "warn" => Some(Self::Warn),
            "mute_temp" => Some(Self::MuteTemp),
            "mute_permanent" => Some(Self::MutePermanent),
            "unmute" => Some(Self::Unmute),
            "ban_temp" => Some(Self::BanTemp),
            "ban_permanent" => Some(Self::BanPermanent),
            "unban" => Some(Self::Unban),
            "call" => Some(Self::Call),
            _ => None,
        }
    }

    /// True si c'est un type de ban (temp ou permanent).
    pub fn is_ban(&self) -> bool {
        matches!(self, Self::BanTemp | Self::BanPermanent)
    }

    /// True si c'est un type de mute (temp ou permanent).
    pub fn is_mute(&self) -> bool {
        matches!(self, Self::MuteTemp | Self::MutePermanent)
    }

    /// Liste des valeurs valides.
    pub const VALID_VALUES: &'static [&'static str] = &[
        "warn", "mute_temp", "mute_permanent", "unmute",
        "ban_temp", "ban_permanent", "unban", "call",
    ];
}

impl fmt::Display for ModerationActionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_all_variants() {
        for s in ModerationActionType::VALID_VALUES {
            let action = ModerationActionType::from_str(s).unwrap();
            assert_eq!(action.as_str(), *s);
        }
    }

    #[test]
    fn from_str_invalid() {
        assert!(ModerationActionType::from_str("kick").is_none());
        assert!(ModerationActionType::from_str("").is_none());
    }

    #[test]
    fn is_ban() {
        assert!(ModerationActionType::BanTemp.is_ban());
        assert!(ModerationActionType::BanPermanent.is_ban());
        assert!(!ModerationActionType::Warn.is_ban());
        assert!(!ModerationActionType::MuteTemp.is_ban());
    }

    #[test]
    fn is_mute() {
        assert!(ModerationActionType::MuteTemp.is_mute());
        assert!(ModerationActionType::MutePermanent.is_mute());
        assert!(!ModerationActionType::Warn.is_mute());
        assert!(!ModerationActionType::BanTemp.is_mute());
    }

    #[test]
    fn serde_roundtrip() {
        let json = serde_json::to_string(&ModerationActionType::BanPermanent).unwrap();
        assert_eq!(json, "\"ban_permanent\"");
        let back: ModerationActionType = serde_json::from_str(&json).unwrap();
        assert_eq!(back, ModerationActionType::BanPermanent);
    }

    #[test]
    fn valid_values_count() {
        assert_eq!(ModerationActionType::VALID_VALUES.len(), 8);
    }
}
