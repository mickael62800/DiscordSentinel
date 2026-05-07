//! Construction des embeds et boutons pour le flow de défi `/coude`.
//!
//! Trois variantes d'embed :
//! - `build_challenge_embed` : le défi normal avec boutons Accepter/Refuser/Annuler
//! - `build_surprise_embed`  : attaque surprise auto-résolue (pas de boutons)
//! - `build_bloodbath_embed` : auto-accept forcé par l'event Bloodbath
//!
//! Plus le `build_notification_embed` pour le ping dans le salon notifications.

use serenity::all::{
    ButtonStyle, CreateActionRow, CreateButton, CreateEmbed, CreateEmbedFooter,
};
use serenity::model::id::UserId;

const FOOTER: &str = crate::shared::branding::COUDE_TAGLINE_SHORT;

/// Embed du défi normal avec tous les boutons d'action.
pub fn build_challenge_embed(
    attacker_id: UserId,
    defender_id: UserId,
    mise: i64,
    special_label: &str,
    handicap_warning: &str,
) -> CreateEmbed {
    CreateEmbed::new()
        .title("\u{1f44a} Coup de Coude !")
        .description(format!(
            "<@{}> defie <@{}> pour **{} coins** !{}{}\n\n<@{}>, tu acceptes ?",
            attacker_id, defender_id, mise, special_label, handicap_warning, defender_id
        ))
        .color(0xFFA500)
        .field("Attaquant", format!("<@{}>", attacker_id), true)
        .field("Defenseur", format!("<@{}>", defender_id), true)
        .field("Mise", format!("{} coins", mise), true)
        .footer(CreateEmbedFooter::new(
            "Coup de Coude | Sentinel — Expire dans 24h",
        ))
        .timestamp(serenity::model::Timestamp::now())
}

/// Boutons du défi : Accepter / Objet / Refuser / Annuler.
pub fn build_challenge_buttons(combat_id: &str) -> CreateActionRow {
    let accept_btn = CreateButton::new(format!("coude_accept:{}", combat_id))
        .label("Accepter")
        .emoji(serenity::model::channel::ReactionType::Unicode(
            "\u{270a}".to_string(),
        ))
        .style(ButtonStyle::Success);

    let item_btn = CreateButton::new(format!("coude_defend:{}", combat_id))
        .label("Objet")
        .emoji(serenity::model::channel::ReactionType::Unicode(
            "\u{1f6e1}\u{fe0f}".to_string(),
        ))
        .style(ButtonStyle::Primary);

    let refuse_btn = CreateButton::new(format!("coude_refuse:{}", combat_id))
        .label("Refuser")
        .emoji(serenity::model::channel::ReactionType::Unicode(
            "\u{1f414}".to_string(),
        ))
        .style(ButtonStyle::Danger);

    let cancel_btn = CreateButton::new(format!("coude_cancel:{}", combat_id))
        .label("Annuler")
        .emoji(serenity::model::channel::ReactionType::Unicode(
            "\u{274c}".to_string(),
        ))
        .style(ButtonStyle::Secondary);

    CreateActionRow::Buttons(vec![accept_btn, item_btn, refuse_btn, cancel_btn])
}

/// Embed pour une attaque surprise (auto-résolue, pas de boutons).
pub fn build_surprise_embed(attacker_id: UserId, defender_id: UserId) -> CreateEmbed {
    CreateEmbed::new()
        .title("\u{1f4a8} ATTAQUE SURPRISE !")
        .description(format!(
            "<@{}> lance une attaque surprise sur <@{}> !\nImpossible de refuser...",
            attacker_id, defender_id
        ))
        .color(0xFF4500)
        .footer(CreateEmbedFooter::new(FOOTER))
        .timestamp(serenity::model::Timestamp::now())
}

/// Embed Bloodbath : le défenseur est forcé d'accepter.
pub fn build_bloodbath_embed(attacker_id: UserId, defender_id: UserId) -> CreateEmbed {
    CreateEmbed::new()
        .title("\u{1fa78} BLOODBATH EN COURS !")
        .description(format!(
            "Pas le choix ! <@{}> est force d'accepter le defi de <@{}> !",
            defender_id, attacker_id
        ))
        .color(0xED4245)
        .footer(CreateEmbedFooter::new(FOOTER))
        .timestamp(serenity::model::Timestamp::now())
}

/// Embed pour le salon de notifications (ping du défenseur).
pub fn build_notification_embed(
    defender_id: UserId,
    attacker_name: &str,
    mise: i64,
    combat_channel: &str,
) -> CreateEmbed {
    CreateEmbed::new()
        .title("\u{2694}\u{fe0f} Nouveau defi !")
        .description(format!(
            "<@{}> ! **{}** te defie en Coup de Coude pour **{} coins** !\n\n\
            Rends-toi dans <#{}> pour accepter ou refuser.\n\
            Les autres : venez parier avec `/pari` !",
            defender_id, attacker_name, mise, combat_channel
        ))
        .color(0xFFA500)
        .footer(CreateEmbedFooter::new(FOOTER))
        .timestamp(serenity::model::Timestamp::now())
}

/// Construit le message de handicap matchmaking à afficher dans le défi.
///
/// Vide si l'écart de niveau est < 3.
pub fn build_handicap_warning(
    attacker_id: UserId,
    attacker_level: i32,
    defender_id: UserId,
    defender_level: i32,
    handicap: f64,
) -> String {
    let level_gap = (attacker_level - defender_level).abs();
    if level_gap < 3 {
        return String::new();
    }
    let handicap_pct = ((1.0 - handicap) * 100.0) as i32;
    let stronger_name = if attacker_level > defender_level {
        format!("<@{}>", attacker_id)
    } else {
        format!("<@{}>", defender_id)
    };
    format!(
        "\n\u{2696}\u{fe0f} **Handicap matchmaking** : {} a -{}% ATK (ecart {} niveaux). Si l'underdog gagne : mise doublee + XP x2 !",
        stronger_name, handicap_pct, level_gap
    )
}
