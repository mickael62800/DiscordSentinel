//! Rendu de la carte d'une idee et de ses boutons de decision.

use std::collections::HashMap;

use serenity::all::{ButtonStyle, CreateActionRow, CreateButton};
use serenity::builder::{CreateEmbed, CreateEmbedFooter};
use serenity::model::user::User;

use crate::shared::api_client::BaseApiClient;

use super::constants::*;

/// Couleur de l'embed selon le statut, reglable par serveur.
fn status_color(status: &str, cfg: &HashMap<String, String>) -> serenity::all::Color {
    let (key, default) = status_color_config(status);
    let hex = BaseApiClient::config_or(cfg, key, default);
    let raw = u32::from_str_radix(hex.trim_start_matches('#'), 16).unwrap_or(0x3498db);
    serenity::all::Color::new(raw)
}

/// Carte d'une idee. `author` est l'auteur de la proposition (sa vignette est
/// affichee), `decision` le couple (qui, motif) quand le staff a tranche.
#[allow(clippy::too_many_arguments)]
pub fn build_idea_embed_full(
    idea_id: &str,
    title: &str,
    description: &str,
    category: &str,
    status: &str,
    author_label: &str,
    author_avatar: Option<String>,
    decision: Option<(&str, Option<&str>)>,
    cfg: &HashMap<String, String>,
) -> CreateEmbed {
    let mut embed = CreateEmbed::new()
        .title(format!("Idee — {title}"))
        .description(description)
        .color(status_color(status, cfg))
        .field("Categorie", category_label(category), true)
        .field("Statut", status_label(status), true)
        .field("Propose par", author_label, true);

    if let Some(avatar) = author_avatar {
        embed = embed.thumbnail(avatar);
    }
    if let Some((by, reason)) = decision {
        embed = embed.field("Decision de", by, true);
        if let Some(r) = reason.filter(|r| !r.trim().is_empty()) {
            embed = embed.field("Motif", r, false);
        }
    }

    // L'id sert au staff et au support pour retrouver l'idee cote web.
    embed.footer(CreateEmbedFooter::new(format!("Idee {idea_id}")))
}

/// Variante courte pour la creation, quand l'auteur est l'utilisateur Discord
/// qui vient de soumettre la modale.
pub fn build_idea_embed(
    idea_id: &str,
    title: &str,
    description: &str,
    category: &str,
    status: &str,
    author: &User,
    cfg: &HashMap<String, String>,
) -> CreateEmbed {
    build_idea_embed_full(
        idea_id,
        title,
        description,
        category,
        status,
        &author.name,
        Some(author.face()),
        None,
        cfg,
    )
}

/// Boutons de decision. Ils restent visibles pour tous mais le handler refuse
/// les clics hors staff : Discord ne sait pas restreindre un bouton par role.
pub fn build_staff_buttons() -> CreateActionRow {
    CreateActionRow::Buttons(vec![
        CreateButton::new(DISCUSS_BUTTON_ID)
            .label("En discussion")
            .style(ButtonStyle::Primary),
        CreateButton::new(ACCEPT_BUTTON_ID)
            .label("Accepter")
            .style(ButtonStyle::Success),
        CreateButton::new(REFUSE_BUTTON_ID)
            .label("Refuser")
            .style(ButtonStyle::Danger),
        CreateButton::new(DONE_BUTTON_ID)
            .label("Realisee")
            .style(ButtonStyle::Secondary),
    ])
}
