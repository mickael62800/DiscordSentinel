//! Construction des embeds Discord pour une partie de blackjack.

use serenity::all::{CreateAttachment, CreateEmbed, CreateEmbedFooter};

use super::api_client::{BlackjackGameDto, CardDto};
use super::card_image;

/// Cle de template flavor a utiliser selon le statut final d'une partie.
/// Retourne `None` si la partie n'est pas dans un etat final qui necessite
/// une raillerie (ex: en cours, push).
pub fn flavor_key_for_status(status: &str) -> Option<&'static str> {
    match status {
        "player_blackjack" => Some("bj_natural"),
        "player_win" | "dealer_bust" => Some("bj_win"),
        "player_bust" => Some("bj_bust"),
        "dealer_win" => Some("bj_lose"),
        _ => None,
    }
}

/// Nom du fichier attachment pour l'image composee player+dealer.
pub const TABLE_IMAGE_NAME: &str = "table.png";

/// `true` si la partie est dans un etat final (plus d'action possible).
pub fn is_game_over(status: &str) -> bool {
    matches!(
        status,
        "player_bust" | "player_win" | "dealer_win" | "dealer_bust" | "push" | "player_blackjack"
    )
}

/// Representation textuelle d'une carte ("As\u{2665}\u{fe0f}", "Roi\u{2660}\u{fe0f}", "\u{1f0a0}" si cachee).
fn card_to_unicode(card: &CardDto) -> String {
    if card.rank == "hidden" {
        return "\u{1f0a0}".to_string();
    }

    let suit_emoji = match card.suit.as_str() {
        "hearts" => "\u{2665}\u{fe0f}",
        "diamonds" => "\u{2666}\u{fe0f}",
        "clubs" => "\u{2663}\u{fe0f}",
        "spades" => "\u{2660}\u{fe0f}",
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

/// Construit l'embed complet + l'attachment image de la table pour une
/// partie en cours ou terminee. Retourne `(embed, Some(attachment))` si
/// l'image a pu etre composee, sinon `(embed, None)` et l'embed retombe
/// sur une representation texte.
pub fn build_game_message(
    game: &BlackjackGameDto,
    wallet_balance: i64,
    flavor: Option<&str>,
) -> (CreateEmbed, Option<CreateAttachment>) {
    let embed = build_game_embed(game, wallet_balance, flavor);
    match card_image::render_table(&game.player_hand, &game.dealer_hand) {
        Some(bytes) => {
            let embed_with_image = embed.image(format!("attachment://{}", TABLE_IMAGE_NAME));
            let attachment = CreateAttachment::bytes(bytes, TABLE_IMAGE_NAME);
            (embed_with_image, Some(attachment))
        }
        None => (embed, None),
    }
}

/// Embed principal d'une partie — en cours ou terminee (victoire / bust / push / ...).
///
/// `flavor` : template tire cote API (`api.random_flavor`) selon le statut.
/// `None` si la partie n'est pas en etat final, OU si l'API n'a pas pu
/// repondre — dans ce cas on affiche un texte neutre.
pub fn build_game_embed(
    game: &BlackjackGameDto,
    wallet_balance: i64,
    flavor: Option<&str>,
) -> CreateEmbed {
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
            "\u{1f0cf} BLACKJACK".to_string(),
            format!("**Mise :** {} coins\n\nA toi de jouer !", game.bet),
            0xF1C40F, // or
        )
    } else {
        // `flavor` est pre-fetch via `api.random_flavor` selon `flavor_key_for_status`.
        // S'il est None (API down ou cle non seedee), on affiche un texte neutre
        // — pas de fallback local (templates migres en DB, cf. migration 174).
        let format_flavor = |tmpl: Option<&str>| -> Option<String> {
            tmpl.map(|t| {
                t.replace("{joueur}", &game.username)
                    .replace("{total}", &game.player_score.to_string())
                    .replace("{croupier}", &game.dealer_score.to_string())
                    .replace("{gain}", &game.payout.to_string())
                    .replace("{mise}", &game.bet.to_string())
            })
        };
        match game.status.as_str() {
            "player_blackjack" => {
                let msg = format_flavor(flavor)
                    .unwrap_or_else(|| format!("{} sort un blackjack !", game.username));
                (
                    "\u{1f31f} BLACKJACK NATUREL !".to_string(),
                    format!("{}\n\n**+{} coins !**", msg, game.payout),
                    0x57F287, // vert
                )
            }
            "player_win" | "dealer_bust" => {
                let msg = format_flavor(flavor)
                    .unwrap_or_else(|| format!("{} gagne {} contre {} !", game.username, game.player_score, game.dealer_score));
                (
                    "\u{1f389} VICTOIRE !".to_string(),
                    format!("{}\n\n**+{} coins !**", msg, game.payout),
                    0x57F287,
                )
            }
            "player_bust" => {
                let msg = format_flavor(flavor)
                    .unwrap_or_else(|| format!("{} bust a {} !", game.username, game.player_score));
                let lost = if game.doubled { game.bet * 2 } else { game.bet };
                (
                    "\u{1f4a5} BUST !".to_string(),
                    format!("{}\n\n**-{} coins**", msg, lost),
                    0xED4245, // rouge
                )
            }
            "dealer_win" => {
                let msg = format_flavor(flavor)
                    .unwrap_or_else(|| format!("Le croupier gagne {} contre {} de {}.", game.dealer_score, game.player_score, game.username));
                let lost = if game.doubled { game.bet * 2 } else { game.bet };
                (
                    "\u{1f624} DEFAITE".to_string(),
                    format!("{}\n\n**-{} coins**", msg, lost),
                    0xED4245,
                )
            }
            "push" => (
                "\u{1f91d} EGALITE".to_string(),
                format!(
                    "{} et le croupier font tous les deux **{}**.\nMise remboursee !",
                    game.username, game.player_score
                ),
                0x95A5A6, // gris
            ),
            _ => (
                "\u{1f0cf} BLACKJACK".to_string(),
                "Partie terminee.".to_string(),
                0x95A5A6,
            ),
        }
    };

    let mut embed = CreateEmbed::new()
        .title(&title)
        .description(&description)
        .field(
            "\u{1f3b4} Tes cartes",
            format!("{}\n**Score : {}**", player_hand_str, game.player_score),
            true,
        )
        .field(
            "\u{1f3e6} Croupier",
            format!("{}\n**Score : {}**", dealer_hand_str, dealer_score_str),
            true,
        )
        .field(
            "\u{1f4b0} Porte-monnaie",
            format!("{} coins", wallet_balance),
            false,
        )
        .color(color)
        .footer(CreateEmbedFooter::new(crate::shared::branding::BLACKJACK_TAGLINE))
        .timestamp(serenity::model::Timestamp::now());

    if game.doubled {
        embed = embed.field(
            "\u{1f4b0} Mise doublee",
            format!("{} coins", game.bet * 2),
            false,
        );
    }

    embed
}
