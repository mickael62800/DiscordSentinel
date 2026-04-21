use chrono::{DateTime, Utc};
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Card {
    pub rank: String,
    pub suit: String,
}

impl Card {
    /// Valeur faciale de la carte (As = 11, figures = 10).
    pub fn value(&self) -> i32 {
        match self.rank.as_str() {
            "As" => 11,
            "Jack" | "Queen" | "King" => 10,
            _ => self.rank.parse::<i32>().unwrap_or(0),
        }
    }

    /// Nom de fichier image : ex. "As_heart.jpg", "10_club.jpg"
    pub fn filename(&self) -> String {
        format!("{}_{}.jpg", self.rank, self.suit)
    }
}

/// Calcule le score d'une main en ajustant les As (11 → 1) si nécessaire.
pub fn calculate_score(hand: &[Card]) -> i32 {
    let mut score: i32 = hand.iter().map(|c| c.value()).sum();
    let mut aces = hand.iter().filter(|c| c.rank == "As").count() as i32;
    while score > 21 && aces > 0 {
        score -= 10;
        aces -= 1;
    }
    score
}

/// Crée un deck standard de 52 cartes, mélangé.
pub fn create_deck() -> Vec<Card> {
    let suits = ["hearts", "diamonds", "clubs", "spades"];
    let ranks = [
        "2", "3", "4", "5", "6", "7", "8", "9", "10",
        "Jack", "Queen", "King", "As",
    ];
    let mut deck = Vec::with_capacity(52);
    for suit in &suits {
        for rank in &ranks {
            deck.push(Card {
                rank: rank.to_string(),
                suit: suit.to_string(),
            });
        }
    }
    let mut rng = rand::thread_rng();
    deck.shuffle(&mut rng);
    deck
}

/// Parametres metier du blackjack (mise min/max, solde initial, payout
/// blackjack naturel). Valeurs par defaut et bornes definies dans le
/// domaine, alimentees par `bot_guild_config` dans les adapters.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BlackjackConfig {
    pub min_bet: i64,
    pub max_bet: i64,
    pub starting_coins: i64,
    pub blackjack_payout: f64,
}

impl Default for BlackjackConfig {
    fn default() -> Self {
        Self {
            min_bet: 10,
            max_bet: 1000,
            starting_coins: 200,
            blackjack_payout: 1.5,
        }
    }
}

impl BlackjackConfig {
    /// Construit une config a partir de paires (key, value) lues depuis
    /// `bot_guild_config`. Les valeurs manquantes ou malformees retombent
    /// sur les defauts du domaine.
    ///
    /// Invariants metier :
    /// - `min_bet > 0`, `max_bet > 0`, `starting_coins >= 0`, `blackjack_payout > 0`.
    /// - `min_bet <= max_bet` (sinon, retombe sur defauts pour cette paire).
    pub fn from_kv_pairs(pairs: &[(String, String)]) -> Self {
        let d = Self::default();
        let mut cfg = d;
        for (k, v) in pairs {
            match k.as_str() {
                "min_bet" => { if let Ok(n) = v.parse::<i64>() { if n > 0 { cfg.min_bet = n; } } }
                "max_bet" => { if let Ok(n) = v.parse::<i64>() { if n > 0 { cfg.max_bet = n; } } }
                "starting_coins" => { if let Ok(n) = v.parse::<i64>() { if n >= 0 { cfg.starting_coins = n; } } }
                "blackjack_payout" => { if let Ok(n) = v.parse::<f64>() { if n > 0.0 { cfg.blackjack_payout = n; } } }
                _ => {}
            }
        }
        if cfg.min_bet > cfg.max_bet {
            cfg.min_bet = d.min_bet;
            cfg.max_bet = d.max_bet;
        }
        cfg
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlackjackGame {
    pub id: Uuid,
    pub guild_id: String,
    pub user_id: String,
    pub username: String,
    pub bet: i64,
    pub player_hand: Vec<Card>,
    pub dealer_hand: Vec<Card>,
    pub deck: Vec<Card>,
    pub status: String,
    pub player_score: i32,
    pub dealer_score: i32,
    pub doubled: bool,
    pub payout: i64,
    pub created_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
}

#[cfg(test)]
#[path = "tests/blackjack.rs"]
mod tests;
