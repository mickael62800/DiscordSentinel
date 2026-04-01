use serenity::builder::{CreateActionRow, CreateButton, CreateEmbed, CreateMessage};

pub const SATISFACTION_PREFIX: &str = "sentinel_ticket_satisfaction_";

/// Construit le message de sondage satisfaction avec 5 boutons etoiles.
#[allow(dead_code)]
pub fn build_survey_message(ticket_short_id: &str) -> CreateMessage {
    let buttons: Vec<CreateButton> = (1..=5)
        .map(|rating| {
            let stars = "\u{2b50}".repeat(rating);
            let custom_id = format!("{}{}_{}", SATISFACTION_PREFIX, ticket_short_id, rating);
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
/// Format: "sentinel_ticket_satisfaction_{ticket_short}_{rating}"
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_valid_rating() {
        let id = "sentinel_ticket_satisfaction_abcdef12_3";
        assert_eq!(extract_rating(id), Some(3));
    }

    #[test]
    fn extract_rating_1() {
        let id = "sentinel_ticket_satisfaction_abcdef12_1";
        assert_eq!(extract_rating(id), Some(1));
    }

    #[test]
    fn extract_rating_5() {
        let id = "sentinel_ticket_satisfaction_abcdef12_5";
        assert_eq!(extract_rating(id), Some(5));
    }

    #[test]
    fn extract_invalid_rating_0() {
        let id = "sentinel_ticket_satisfaction_abcdef12_0";
        assert_eq!(extract_rating(id), None);
    }

    #[test]
    fn extract_invalid_rating_6() {
        let id = "sentinel_ticket_satisfaction_abcdef12_6";
        assert_eq!(extract_rating(id), None);
    }

    #[test]
    fn extract_no_prefix() {
        assert_eq!(extract_rating("random_id"), None);
    }

    #[test]
    fn extract_no_rating() {
        let id = "sentinel_ticket_satisfaction_";
        assert_eq!(extract_rating(id), None);
    }
}
