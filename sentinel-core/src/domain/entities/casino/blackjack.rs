use crate::domain::entities::system::discord_ids::GuildId;
use crate::domain::entities::system::discord_ids::UserId;
use chrono::DateTime;
use chrono::Utc;
use rand::seq::SliceRandom;
use serde::Deserialize;
use serde::Serialize;
use uuid::Uuid;

/// Plafond dur d'une mise blackjack (config). Empeche un max_bet abusif de faire
/// deborder le calcul de payout (f64 + saturating_mul). 1e12 << i64::MAX / 4.
pub const MAX_BLACKJACK_BET: i64 = 1_000_000_000_000;

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
        "2", "3", "4", "5", "6", "7", "8", "9", "10", "Jack", "Queen", "King", "As",
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
///
/// Nombre max de joueurs par defaut sur une table blackjack multijoueur.
/// Override possible via bot_config key `max_players_per_table`.
pub const DEFAULT_BLACKJACK_MAX_PLAYERS: i64 = 7;

/// Statuts finaux d'une partie de blackjack (plus d'action possible).
/// Regle metier : cette liste definit la fin de partie (affichage dealer
/// complet, payout final, broadcast blackjack_result).
pub const BLACKJACK_FINAL_STATUSES: &[&str] = &[
    "player_bust",
    "player_win",
    "dealer_win",
    "dealer_bust",
    "push",
    "player_blackjack",
];

/// `true` si le statut correspond a une partie terminee.
pub fn is_blackjack_game_over(status: &str) -> bool {
    BLACKJACK_FINAL_STATUSES.contains(&status)
}

/// Resultat possible d'une main a la distribution initiale (avant toute
/// action du joueur), en fonction des scores naturels.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NaturalDealOutcome {
    /// Aucun blackjack naturel cote joueur : la partie continue.
    KeepPlaying,
    /// Joueur ET croupier ont un 21 naturel : egalite (push). La mise est
    /// remboursee a l'identique (gain net nul).
    Push,
    /// Blackjack naturel du joueur (le croupier n'a pas 21) : paiement majore.
    PlayerBlackjack,
}

/// Determine l'issue d'une main des la distribution.
///
/// Regle metier : un 21 naturel du joueur paie le blackjack, SAUF si le
/// croupier a egalement un 21 naturel, auquel cas c'est une egalite (push).
pub fn natural_deal_outcome(player_score: i32, dealer_score: i32) -> NaturalDealOutcome {
    if player_score == 21 && dealer_score == 21 {
        NaturalDealOutcome::Push
    } else if player_score == 21 {
        NaturalDealOutcome::PlayerBlackjack
    } else {
        NaturalDealOutcome::KeepPlaying
    }
}

/// Nombre de decks dans un shoe de blackjack multiplayer. Regle standard
/// casino (6 decks = 312 cartes) : ratisse le cardcount pour diluer les
/// avantages statistiques et eviter les sessions trop longues.
pub const BLACKJACK_SHOE_DECKS: usize = 6;

/// Nombre total de cartes dans un shoe neuf : 6 decks * 52 cartes.
pub const BLACKJACK_SHOE_TOTAL_CARDS: usize = BLACKJACK_SHOE_DECKS * 52;

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
                "min_bet" => {
                    if let Ok(n) = v.parse::<i64>() {
                        if n > 0 {
                            cfg.min_bet = n.min(MAX_BLACKJACK_BET);
                        }
                    }
                }
                "max_bet" => {
                    if let Ok(n) = v.parse::<i64>() {
                        if n > 0 {
                            // Plafond dur : sans lui, un max_bet abusif (proche de
                            // i64::MAX) faisait exploser le payout (overflow f64 +
                            // saturating_mul -> credit ~ i64::MAX pour un debit de
                            // 2*bet seulement).
                            cfg.max_bet = n.min(MAX_BLACKJACK_BET);
                        }
                    }
                }
                "starting_coins" => {
                    if let Ok(n) = v.parse::<i64>() {
                        if n >= 0 {
                            cfg.starting_coins = n;
                        }
                    }
                }
                "blackjack_payout" => {
                    if let Ok(n) = v.parse::<f64>() {
                        if n > 0.0 {
                            cfg.blackjack_payout = n;
                        }
                    }
                }
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
    pub guild_id: GuildId,
    pub user_id: UserId,
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
