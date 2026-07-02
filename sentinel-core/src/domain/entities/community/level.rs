use crate::domain::entities::system::discord_ids::GuildId;
use crate::domain::entities::system::discord_ids::UserId;
use chrono::DateTime;
use chrono::Utc;
use uuid::Uuid;

/// Source d'XP : texte (messages) ou vocal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XpSource {
    Text,
    Voice,
}

impl XpSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            XpSource::Text => "text",
            XpSource::Voice => "voice",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        match s {
            "voice" => XpSource::Voice,
            _ => XpSource::Text,
        }
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct UserLevel {
    pub id: Uuid,
    pub guild_id: GuildId,
    pub user_id: UserId,
    pub username: String,
    pub xp: i64,
    pub level: i32,
    pub xp_text: i64,
    pub level_text: i32,
    pub xp_voice: i64,
    pub level_voice: i32,
    pub last_xp_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// XP requis pour atteindre un niveau donne (progression exponentielle).
/// Niveau 1 = 100 XP, Niveau 2 = 255 XP, Niveau 10 = 4750 XP, etc.
pub fn xp_for_level(level: i32) -> i64 {
    if level <= 0 {
        return 0;
    }
    let l = level as f64;
    (5.0 * l.powi(2) + 50.0 * l + 100.0) as i64
}

/// Plafond de niveau pour le systeme d'XP communautaire. Borne le calcul
/// pour eviter une boucle non bornee / un overflow i64 sur des XP absurdes.
/// Volontairement tres genereux : aucun joueur reel ne l'atteindra.
pub const MAX_LEVEL: i32 = 10_000;

/// Calcule le niveau a partir du XP total.
pub fn level_from_xp(xp: i64) -> i32 {
    let mut level = 0;
    let mut total_needed: i64 = 0;
    while level < MAX_LEVEL {
        let next = xp_for_level(level + 1);
        if total_needed + next > xp {
            break;
        }
        total_needed += next;
        level += 1;
    }
    level
}

/// XP restant dans le niveau actuel et XP requis pour le prochain.
pub fn xp_progress(xp: i64) -> (i64, i64) {
    let level = level_from_xp(xp);
    let mut consumed: i64 = 0;
    for l in 1..=level {
        consumed += xp_for_level(l);
    }
    let current_in_level = xp - consumed;
    let needed = xp_for_level(level + 1);
    (current_in_level, needed)
}

#[cfg(test)]
#[path = "tests/level.rs"]
mod tests;
