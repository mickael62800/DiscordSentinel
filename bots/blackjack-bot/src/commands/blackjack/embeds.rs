//! Construction des embeds Discord pour une partie de blackjack.

use serenity::all::{CreateEmbed, CreateEmbedFooter};

use super::messages::{pick_random, BJ_BUST, BJ_LOSE, BJ_NATURAL, BJ_WIN};
use crate::api_client::{BlackjackGameDto, CardDto};

/// `true` si la partie est dans un état final (plus d'action possible).
pub fn is_game_over(status: &str) -> bool {
    matches!(
        status,
        "player_bust" | "player_win" | "dealer_win" | "dealer_bust" | "push" | "player_blackjack"
    )
}

/// Représentation textuelle d'une carte ("As♥️", "Roi♠️", "🂠" si cachée).
fn card_to_unicode(card: &CardDto) -> String {
    if card.rank == "hidden" {
        return "🂠".to_string();
    }

    let suit_emoji = match card.suit.as_str() {
        "hearts" => "♥️",
        "diamonds" => "♦️",
        "clubs" => "♣️",
        "spades" => "♠️",
        _ => "?",
    };

    let rank_display = match card.rank.as_str() {
        "A" => "As",
        "K" => "Roi",
        "Q" => "Dame",
        "J" => "Valet",
        r => r,
    };

    format!("{}{}", rank_display, suit_emoji)
}

fn hand_to_string(hand: &[CardDto]) -> String {
    hand.iter()
        .map(card_to_unicode)
        .collect::<Vec<_>>()
        .join("  ")
}

/// Embed principal d'une partie — en cours ou terminée (victoire / bust / push / ...).
pub fn build_game_embed(game: &BlackjackGameDto) -> CreateEmbed {
    let over = is_game_over(&game.status);

    let player_hand_str = hand_to_string(&game.player_hand);
    let dealer_hand_str = hand_to_string(&game.dealer_hand);

    let dealer_score_str = if over {
        format!("{}", game.dealer_score)
    } else {
        format!("{}+?", game.dealer_score)
    };

    let (title, description, color) = if !over {
        (
            "🃏 BLACKJACK".to_string(),
            format!("**Mise :** {} coins\n\nA toi de jouer !", game.bet),
            0xF1C40F, // or
        )
    } else {
        match game.status.as_str() {
            "player_blackjack" => {
                let msg = pick_random(BJ_NATURAL).replace("{joueur}", &game.username);
                (
                    "🌟 BLACKJACK NATUREL !".to_string(),
                    format!("{}\n\n**+{} coins !**", msg, game.payout),
                    0x57F287, // vert
                )
            }
            "player_win" | "dealer_bust" => {
                let msg = pick_random(BJ_WIN)
                    .replace("{joueur}", &game.username)
                    .replace("{total}", &game.player_score.to_string())
                    .replace("{croupier}", &game.dealer_score.to_string())
                    .replace("{gain}", &game.payout.to_string());
                (
                    "🎉 VICTOIRE !".to_string(),
                    format!("{}\n\n**+{} coins !**", msg, game.payout),
                    0x57F287,
                )
            }
            "player_bust" => {
                let msg = pick_random(BJ_BUST)
                    .replace("{joueur}", &game.username)
                    .replace("{total}", &game.player_score.to_string());
                let lost = if game.doubled { game.bet * 2 } else { game.bet };
                (
                    "💥 BUST !".to_string(),
                    format!("{}\n\n**-{} coins**", msg, lost),
                    0xED4245, // rouge
                )
            }
            "dealer_win" => {
                let msg = pick_random(BJ_LOSE)
                    .replace("{joueur}", &game.username)
                    .replace("{total}", &game.player_score.to_string())
                    .replace("{croupier}", &game.dealer_score.to_string())
                    .replace("{mise}", &game.bet.to_string());
                let lost = if game.doubled { game.bet * 2 } else { game.bet };
                (
                    "😤 DEFAITE".to_string(),
                    format!("{}\n\n**-{} coins**", msg, lost),
                    0xED4245,
                )
            }
            "push" => (
                "🤝 EGALITE".to_string(),
                format!(
                    "{} et le croupier font tous les deux **{}**.\nMise remboursee !",
                    game.username, game.player_score
                ),
                0x95A5A6, // gris
            ),
            _ => (
                "🃏 BLACKJACK".to_string(),
                "Partie terminee.".to_string(),
                0x95A5A6,
            ),
        }
    };

    let mut embed = CreateEmbed::new()
        .title(&title)
        .description(&description)
        .field(
            "🎴 Tes cartes",
            format!("{}\n**Score : {}**", player_hand_str, game.player_score),
            true,
        )
        .field(
            "🏦 Croupier",
            format!("{}\n**Score : {}**", dealer_hand_str, dealer_score_str),
            true,
        )
        .color(color)
        .footer(CreateEmbedFooter::new("Blackjack | Sentinel"))
        .timestamp(serenity::model::Timestamp::now());

    if game.doubled {
        embed = embed.field(
            "💰 Mise doublee",
            format!("{} coins", game.bet * 2),
            false,
        );
    }

    embed
}
