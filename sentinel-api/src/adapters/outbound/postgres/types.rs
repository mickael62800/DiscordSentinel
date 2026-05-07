//! Wrappers Postgres pour les enums du domaine.
//!
//! Le `sentinel-core` ne connait pas sqlx : les derives `sqlx::Type` qui
//! lient les enums aux types Postgres custom (`moderation_gravity`,
//! `coude_class`, `voice_channel_kind`) vivent ici, dans l'adapter. Les
//! repos `query_as!` decodent vers `Pg*` puis convertissent via `.into()`.

use sentinel_core::domain::enums::community::voice_channel_kind::VoiceChannelKind;
use sentinel_core::domain::enums::coude::coude_class::PlayerClass;
use sentinel_core::domain::enums::moderation::moderation_gravity::ModerationGravity;

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "moderation_gravity", rename_all = "lowercase")]
pub enum PgModerationGravity {
    Low,
    Medium,
    High,
    Critical,
}

impl From<PgModerationGravity> for ModerationGravity {
    fn from(g: PgModerationGravity) -> Self {
        match g {
            PgModerationGravity::Low => Self::Low,
            PgModerationGravity::Medium => Self::Medium,
            PgModerationGravity::High => Self::High,
            PgModerationGravity::Critical => Self::Critical,
        }
    }
}

impl From<ModerationGravity> for PgModerationGravity {
    fn from(g: ModerationGravity) -> Self {
        match g {
            ModerationGravity::Low => Self::Low,
            ModerationGravity::Medium => Self::Medium,
            ModerationGravity::High => Self::High,
            ModerationGravity::Critical => Self::Critical,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "coude_class", rename_all = "lowercase")]
pub enum PgPlayerClass {
    Bourrin,
    Agile,
    Fourbe,
    Tank,
}

impl From<PgPlayerClass> for PlayerClass {
    fn from(c: PgPlayerClass) -> Self {
        match c {
            PgPlayerClass::Bourrin => Self::Bourrin,
            PgPlayerClass::Agile => Self::Agile,
            PgPlayerClass::Fourbe => Self::Fourbe,
            PgPlayerClass::Tank => Self::Tank,
        }
    }
}

impl From<PlayerClass> for PgPlayerClass {
    fn from(c: PlayerClass) -> Self {
        match c {
            PlayerClass::Bourrin => Self::Bourrin,
            PlayerClass::Agile => Self::Agile,
            PlayerClass::Fourbe => Self::Fourbe,
            PlayerClass::Tank => Self::Tank,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, sqlx::Type)]
#[sqlx(type_name = "voice_channel_kind", rename_all = "lowercase")]
pub enum PgVoiceChannelKind {
    #[default]
    Public,
    Private,
}

impl From<PgVoiceChannelKind> for VoiceChannelKind {
    fn from(k: PgVoiceChannelKind) -> Self {
        match k {
            PgVoiceChannelKind::Public => Self::Public,
            PgVoiceChannelKind::Private => Self::Private,
        }
    }
}

impl From<VoiceChannelKind> for PgVoiceChannelKind {
    fn from(k: VoiceChannelKind) -> Self {
        match k {
            VoiceChannelKind::Public => Self::Public,
            VoiceChannelKind::Private => Self::Private,
        }
    }
}
