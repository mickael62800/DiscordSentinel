use serenity::builder::{CreateActionRow, CreateButton, CreateEmbed};

/// Entree FAQ.
#[derive(Debug, Clone, PartialEq)]
pub struct FaqEntry {
    pub question: String,
    pub answer: String,
}

pub const FAQ_CONTINUE_ID: &str = "sentinel_ticket_faq_continue";

/// Parse les FAQ depuis le format config : "question|reponse" par ligne.
pub fn parse_faq(raw: &str) -> Vec<FaqEntry> {
    sentinel_shared::parsers::parse_pipe_lines(raw)
        .into_iter()
        .map(|(question, answer)| FaqEntry { question, answer })
        .collect()
}

/// Construit un embed affichant les FAQ.
pub fn build_faq_embed(entries: &[FaqEntry]) -> CreateEmbed {
    let mut embed = CreateEmbed::new()
        .title("Questions frequentes")
        .description("Votre reponse se trouve peut-etre ici ! Consultez les FAQ ci-dessous avant de creer un ticket.")
        .color(0x3498db);

    for (i, entry) in entries.iter().take(10).enumerate() {
        embed = embed.field(
            format!("{}. {}", i + 1, entry.question),
            &entry.answer,
            false,
        );
    }

    embed
}

/// Construit le bouton "Creer un ticket quand meme".
pub fn build_faq_continue_button() -> CreateActionRow {
    let button = CreateButton::new(FAQ_CONTINUE_ID)
        .label("Ma question n'est pas dans la FAQ — Creer un ticket")
        .style(serenity::all::ButtonStyle::Primary);

    CreateActionRow::Buttons(vec![button])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple() {
        let raw = "Comment ca marche ?|Il suffit de cliquer\nOu sont les regles ?|Dans #regles";
        let faq = parse_faq(raw);
        assert_eq!(faq.len(), 2);
        assert_eq!(faq[0].question, "Comment ca marche ?");
        assert_eq!(faq[0].answer, "Il suffit de cliquer");
    }

    #[test]
    fn parse_ignores_empty() {
        let raw = "\n\nQ1|A1\n\n";
        let faq = parse_faq(raw);
        assert_eq!(faq.len(), 1);
    }

    #[test]
    fn parse_empty_string() {
        assert!(parse_faq("").is_empty());
    }
}
