use serde::{Deserialize, Serialize};

/// Phase 2 A.3 — Type de salon vocal temporaire.
///
/// Lie au type Postgres `voice_channel_kind` (migration 103).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "lowercase")]
#[sqlx(type_name = "voice_channel_kind", rename_all = "lowercase")]
#[derive(Default)]
pub enum VoiceChannelKind {
    #[default]
    Public,
    Private,
}

impl VoiceChannelKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Public => "public",
            Self::Private => "private",
        }
    }

    pub fn from_str_lossy(s: &str) -> Self {
        match s {
            "private" => Self::Private,
            _ => Self::Public,
        }
    }
}


#[cfg(test)]
#[path = "tests/voice_channel_kind.rs"]
mod tests;
