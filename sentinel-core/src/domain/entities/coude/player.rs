use chrono::DateTime;
use chrono::Utc;
use crate::domain::enums::coude::coude_class::PlayerClass;
use crate::domain::entities::system::discord_ids::UserId;
use crate::domain::entities::system::discord_ids::GuildId;

/// Niveau maximum atteignable par un joueur.
pub const COUDE_MAX_LEVEL: i32 = 25;

/// XP cumul nécessaire pour atteindre le niveau `n`.
pub fn xp_for_level(n: i32) -> i64 {
    let n = n as i64;
    50 * n * n + 50 * n
}

/// Titre attribué pour un niveau donné.
pub fn title_for_level(level: i32) -> &'static str {
    match level {
        1..=4 => "Debutant",
        5..=9 => "Bagarreur",
        10..=14 => "Guerrier",
        15..=19 => "Veteran",
        20..=24 => "Champion",
        25 => "Inarretable",
        _ => "Debutant",
    }
}

/// Entité riche représentant un joueur de Coup de Coude.
///
/// Mappe directement la table `coude_players` (avec toutes les colonnes
/// utilisées par les handlers : économie, combats, casino, progression, HP, saison).
#[derive(Debug, Clone)]
pub struct Player {
    pub guild_id: GuildId,
    pub user_id: UserId,
    pub username: String,
    pub coins: i64,
    // ── Combats ──
    pub total_wins: i32,
    pub total_losses: i32,
    pub total_draws: i32,
    pub total_earned: i64,
    pub total_lost: i64,
    pub total_stolen: i64,
    pub cowardice_count: i32,
    pub chaos_events: i32,
    // ── Casino ──
    pub casino_wins: i32,
    pub casino_losses: i32,
    // ── Progression ──
    pub level: i32,
    pub xp: i64,
    pub stat_points: i32,
    pub atk: i32,
    pub def: i32,
    pub class: Option<PlayerClass>,
    pub title: Option<String>,
    pub class_changed_at: Option<DateTime<Utc>>,
    // ── HP ──
    pub hp_current: i32,
    pub hp_max: i32,
    pub hp_last_regen: Option<DateTime<Utc>>,
    pub repos_last_used: Option<DateTime<Utc>>,
    // ── Saison ──
    pub season: i32,
    // ── Méta ──
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Résultat d'un ajout d'XP : niveau atteint et points de stats gagnés.
#[derive(Debug, Clone)]
pub struct XpProgress {
    pub new_xp: i64,
    pub new_level: i32,
    pub leveled_up: bool,
    pub stat_points_gained: i32,
}

/// Stat de combat allouable par le joueur.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CombatStat {
    Atk,
    Def,
}

impl CombatStat {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "atk" => Some(Self::Atk),
            "def" => Some(Self::Def),
            _ => None,
        }
    }

    pub fn column(self) -> &'static str {
        match self {
            Self::Atk => "atk",
            Self::Def => "def",
        }
    }
}

#[cfg(test)]
#[path = "tests/player.rs"]
mod tests;
