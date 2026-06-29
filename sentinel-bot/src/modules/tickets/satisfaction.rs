use serenity::builder::{CreateActionRow, CreateButton, CreateEmbed, CreateMessage};

pub const SATISFACTION_PREFIX: &str = "sentinel_ticket_satisfaction_";

/// Construit le message de sondage satisfaction avec 5 boutons etoiles.
pub fn build_survey_message(ticket_id: &str) -> CreateMessage {
    let buttons: Vec<CreateButton> = (1..=5)
        .map(|rating| {
            let stars = "\u{2b50}".repeat(rating);
            let custom_id = format!("{}{}_{}", SATISFACTION_PREFIX, ticket_id, rating);
            CreateButton::new(custom_id)
                .label(stars)
                .style(serenity::all::ButtonStyle::Secondary)
        })
        .collect();

    let row = CreateActionRow::Buttons(buttons);

    let embed = CreateEmbed::new()
        .title("Comment evaluez-vous votre experience ?")
        .description(
            "Votre ticket a ete ferme. Merci de noter la qualite du support recu.\n\
             Cliquez sur le nombre d'etoiles correspondant a votre satisfaction.",
        )
        .color(0x3498db);

    CreateMessage::new().embed(embed).components(vec![row])
}

/// Extrait le rating (1-5) depuis le custom_id d'un bouton satisfaction.
pub fn extract_rating(custom_id: &str) -> Option<u8> {
    let suffix = custom_id.strip_prefix(SATISFACTION_PREFIX)?;
    let rating_str = suffix.rsplit('_').next()?;
    let rating: u8 = rating_str.parse().ok()?;
    if (1..=5).contains(&rating) {
        Some(rating)
    } else {
        None
    }
}

/// Extrait le ticket UUID depuis le custom_id.
pub fn extract_ticket_id(custom_id: &str) -> Option<&str> {
    let suffix = custom_id.strip_prefix(SATISFACTION_PREFIX)?;
    let (ticket_id, _rating) = suffix.rsplit_once('_')?;
    if ticket_id.is_empty() {
        None
    } else {
        Some(ticket_id)
    }
}
