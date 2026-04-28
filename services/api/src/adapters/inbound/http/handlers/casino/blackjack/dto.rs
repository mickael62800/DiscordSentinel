//! DTOs HTTP pour le blackjack (solo + multiplayer tables).

use serde::{Deserialize, Serialize};

use crate::domain::entities::{BlackjackGame, Card};

// ══════════════════════════════════════════════════════════════════════
// ── Solo game DTOs ──
// ══════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize)]
pub struct StartGameDto {
    pub guild_id: String,
    pub user_id: String,
    pub username: String,
    pub bet: i64,
}

#[derive(Debug, Serialize)]
pub struct CardDto {
    pub rank: String,
    pub suit: String,
    pub filename: String,
}

impl From<&Card> for CardDto {
    fn from(c: &Card) -> Self {
        Self {
            rank: c.rank.clone(),
            suit: c.suit.clone(),
            filename: c.filename(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct BlackjackGameDto {
    pub id: String,
    pub guild_id: String,
    pub user_id: String,
    pub username: String,
    pub bet: i64,
    pub player_hand: Vec<CardDto>,
    pub dealer_hand: Vec<CardDto>,
    pub status: String,
    pub player_score: i32,
    pub dealer_score: i32,
    pub doubled: bool,
    pub payout: i64,
    pub created_at: String,
    pub finished_at: Option<String>,
}

/// `true` si la partie est dans un état final (plus d'action possible).
/// Delegue a la regle metier dans `domain::entities::is_blackjack_game_over`.
pub fn game_is_over(status: &str) -> bool {
    crate::domain::entities::is_blackjack_game_over(status)
}

/// Convertit un `BlackjackGame` domaine vers un DTO HTTP.
///
/// Règle métier côté présentation : tant que la partie n'est pas terminée,
/// la seconde carte du croupier est masquée (rank="hidden") et son score
/// affiché ne compte que la carte visible.
pub fn to_dto(game: &BlackjackGame) -> BlackjackGameDto {
    let over = game_is_over(&game.status);

    let dealer_hand: Vec<CardDto> = if over {
        // Partie terminée : révéler toutes les cartes du dealer
        game.dealer_hand.iter().map(CardDto::from).collect()
    } else {
        // Partie en cours : cacher la 2e carte du dealer
        let mut cards: Vec<CardDto> = Vec::new();
        if let Some(first) = game.dealer_hand.first() {
            cards.push(CardDto::from(first));
        }
        if game.dealer_hand.len() > 1 {
            cards.push(CardDto {
                rank: "hidden".to_string(),
                suit: "hidden".to_string(),
                filename: "card_back.jpg".to_string(),
            });
        }
        cards
    };

    let dealer_score = if over {
        game.dealer_score
    } else {
        // Score visible = seulement la première carte
        game.dealer_hand.first().map(|c| c.value()).unwrap_or(0)
    };

    BlackjackGameDto {
        id: game.id.to_string(),
        guild_id: game.guild_id.clone(),
        user_id: game.user_id.clone(),
        username: game.username.clone(),
        bet: game.bet,
        player_hand: game.player_hand.iter().map(CardDto::from).collect(),
        dealer_hand,
        status: game.status.clone(),
        player_score: game.player_score,
        dealer_score,
        doubled: game.doubled,
        payout: game.payout,
        created_at: game.created_at.to_rfc3339(),
        finished_at: game.finished_at.map(|d| d.to_rfc3339()),
    }
}

// ══════════════════════════════════════════════════════════════════════
// ── Multiplayer table DTOs ──
// ══════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize)]
pub struct CreateTableDto {
    pub guild_id: String,
    pub channel_id: String,
    pub owner_id: String,
    pub owner_name: String,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct TableDto {
    pub id: String,
    pub guild_id: String,
    pub channel_id: String,
    pub owner_id: String,
    pub owner_name: String,
    pub status: String,
    pub created_at: String,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct TablePlayerDto {
    pub user_id: String,
    pub user_name: String,
    pub joined_at: String,
}

#[derive(Debug, Deserialize)]
pub struct JoinTableDto {
    pub user_id: String,
    pub user_name: String,
}

#[cfg(test)]
#[path = "tests/dto.rs"]
mod tests;
