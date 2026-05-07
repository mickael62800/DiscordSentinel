//! Construction des boutons d'action (Tirer / Rester / Doubler).

use serenity::all::{ButtonStyle, CreateActionRow, CreateButton};

use super::api_client::BlackjackGameDto;

/// Construit les boutons actifs pour une partie en cours.
/// Le bouton "Doubler" n'apparait que sur la main initiale (2 cartes).
pub fn build_buttons(game: &BlackjackGameDto) -> Vec<CreateActionRow> {
    let game_id = &game.id;

    let hit_btn = CreateButton::new(format!("bj_hit:{game_id}"))
        .label("Tirer")
        .emoji(serenity::model::channel::ReactionType::Unicode(
            "\u{1f0cf}".to_string(),
        ))
        .style(ButtonStyle::Primary);

    let stand_btn = CreateButton::new(format!("bj_stand:{game_id}"))
        .label("Rester")
        .emoji(serenity::model::channel::ReactionType::Unicode(
            "\u{270b}".to_string(),
        ))
        .style(ButtonStyle::Secondary);

    let mut buttons = vec![hit_btn, stand_btn];

    // Doubler seulement possible au premier tour (2 cartes en main)
    if game.player_hand.len() == 2 && !game.doubled {
        let double_btn = CreateButton::new(format!("bj_double:{game_id}"))
            .label("Doubler")
            .emoji(serenity::model::channel::ReactionType::Unicode(
                "\u{1f4b0}".to_string(),
            ))
            .style(ButtonStyle::Danger);
        buttons.push(double_btn);
    }

    vec![CreateActionRow::Buttons(buttons)]
}
