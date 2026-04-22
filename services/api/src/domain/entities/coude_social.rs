use chrono::{DateTime, Utc};
use uuid::Uuid;

// ══════════════════════════════════════════════════════════════════════
// ── Daily chaos : regles metier ──
// ══════════════════════════════════════════════════════════════════════

/// Cap journalier d'evenements `daily chaos` par guild (5/jour).
pub const DAILY_CHAOS_MAX: i64 = 5;

/// Pourcentage des coins de la victime transferes par defaut (20%).
pub const DEFAULT_CHAOS_PERCENT: f64 = 0.20;

/// Solde minimum pour qu'un joueur soit eligible au tirage chaos.
pub const MIN_COINS_ELIGIBLE: i64 = 10;

/// Limites du parametre `limit` du leaderboard (clamp [1, 100]).
pub const LEADERBOARD_MIN_LIMIT: i64 = 1;
pub const LEADERBOARD_MAX_LIMIT: i64 = 100;

/// Calcule le montant a transferer pour un daily chaos. Retourne None
/// si le montant calcule est < 1 (chaos invisible -> skip).
pub fn daily_chaos_amount(victim_coins: i64, chaos_percent: f64) -> Option<i64> {
    let amount = ((victim_coins as f64) * chaos_percent).floor() as i64;
    if amount >= 1 { Some(amount) } else { None }
}

/// Clamp d'un parametre `limit` de leaderboard dans [1, 100].
pub fn clamp_leaderboard_limit(limit: i64) -> i64 {
    limit.clamp(LEADERBOARD_MIN_LIMIT, LEADERBOARD_MAX_LIMIT)
}

// ══════════════════════════════════════════════════════════════════════
// ── Leaderboard ──
// ══════════════════════════════════════════════════════════════════════

/// Catégorie de classement supportée par `/api/coude/{guild}/leaderboard/{category}`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeaderboardCategory {
    Richest,
    Thieves,
    Cowards,
    Chaos,
    Level,
}

impl LeaderboardCategory {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "richest" => Some(Self::Richest),
            "thieves" => Some(Self::Thieves),
            "cowards" => Some(Self::Cowards),
            "chaos" => Some(Self::Chaos),
            "level" => Some(Self::Level),
            _ => None,
        }
    }
}

/// Entrée d'un classement. `value` = critère du classement (coins, level, etc.)
#[derive(Debug, Clone)]
pub struct CoudeLeaderboardEntry {
    pub user_id: String,
    pub username: String,
    pub value: i64,
}

// ══════════════════════════════════════════════════════════════════════
// ── Événements serveur ──
// ══════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone)]
pub struct CoudeEvent {
    pub id: Uuid,
    pub guild_id: String,
    pub event_type: String,
    pub active: bool,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

// ══════════════════════════════════════════════════════════════════════
// ── Daily chaos ──
// ══════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone)]
pub struct NewDailyChaos {
    pub guild_id: String,
    pub loser_id: String,
    pub loser_name: String,
    pub winner_id: String,
    pub winner_name: String,
    pub amount: i64,
}

/// Resultat d'un trigger de chaos journalier reussi, pret a etre affiche.
///
/// `taunt_events` contient les taunts declenches par la mutation wallet
/// (faillite cote victime, jackpot cote winner) — propages via Redis
/// pub/sub par le worker pour que le bot les dispatche.
#[derive(Debug, Clone)]
pub struct DailyChaosOutcome {
    pub loser_id: String,
    pub loser_name: String,
    pub winner_id: String,
    pub winner_name: String,
    pub amount: i64,
    pub channel_id: String,
    pub taunt_events: Vec<crate::domain::entities::TauntEvent>,
}

// ══════════════════════════════════════════════════════════════════════
// ── Saison ──
// ══════════════════════════════════════════════════════════════════════

/// État d'une saison telle qu'exposée au bot (numéro, fenêtre temporelle,
/// jours restants). Durée standard : 90 jours depuis `started_at`.
#[derive(Debug, Clone)]
pub struct CoudeCurrentSeason {
    pub season_number: i32,
    pub started_at: DateTime<Utc>,
    pub ends_at: DateTime<Utc>,
    pub days_remaining: i64,
}

#[cfg(test)]
#[path = "tests/coude_social.rs"]
mod tests;
