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
    let suits = ["heart", "diamond", "club", "spade"];
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
