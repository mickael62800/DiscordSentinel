use serenity::builder::{CreateActionRow, CreateSelectMenu, CreateSelectMenuKind, CreateSelectMenuOption};

/// Template de reponse rapide pour le staff.
#[derive(Debug, Clone, PartialEq)]
pub struct Template {
    pub label: String,
    pub content: String,
}

pub const TEMPLATE_BUTTON_ID: &str = "sentinel_ticket_template";
pub const TEMPLATE_SELECT_ID: &str = "sentinel_ticket_template_select";

/// Parse les templates depuis le format config : "label|contenu" par ligne.
pub fn parse_templates(raw: &str) -> Vec<Template> {
    sentinel_shared::parsers::parse_pipe_lines(raw)
        .into_iter()
        .map(|(label, content)| Template { label, content })
        .collect()
}

/// Construit le menu select pour choisir un template (max 25 options Discord).
pub fn build_template_select(templates: &[Template]) -> CreateActionRow {
    let options: Vec<CreateSelectMenuOption> = templates
        .iter()
        .take(25)
        .enumerate()
        .map(|(i, t)| {
            let desc = if t.content.len() > 80 {
                format!("{}...", &t.content[..77])
            } else {
                t.content.clone()
            };
            CreateSelectMenuOption::new(&t.label, i.to_string()).description(desc)
        })
        .collect();

    let select = CreateSelectMenu::new(
        TEMPLATE_SELECT_ID,
        CreateSelectMenuKind::String { options },
    )
    .placeholder("Choisissez une reponse rapide...");

    CreateActionRow::SelectMenu(select)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple() {
        let raw = "Bienvenue|Bonjour et bienvenue !\nMerci|Merci pour votre message.";
        let templates = parse_templates(raw);
        assert_eq!(templates.len(), 2);
        assert_eq!(templates[0].label, "Bienvenue");
        assert_eq!(templates[0].content, "Bonjour et bienvenue !");
        assert_eq!(templates[1].label, "Merci");
    }

    #[test]
    fn parse_ignores_empty_lines() {
        let raw = "A|Content A\n\n\nB|Content B\n";
        let templates = parse_templates(raw);
        assert_eq!(templates.len(), 2);
    }

    #[test]
    fn parse_ignores_invalid_lines() {
        let raw = "No separator here\n|empty label\nempty content|\nOk|Valid";
        let templates = parse_templates(raw);
        assert_eq!(templates.len(), 1);
        assert_eq!(templates[0].label, "Ok");
    }

    #[test]
    fn parse_trims_whitespace() {
        let raw = "  Hello  |  World  ";
        let templates = parse_templates(raw);
        assert_eq!(templates[0].label, "Hello");
        assert_eq!(templates[0].content, "World");
    }

    #[test]
    fn parse_empty_string() {
        assert!(parse_templates("").is_empty());
        assert!(parse_templates("   \n  ").is_empty());
    }

    #[test]
    fn parse_content_with_pipe() {
        // Le contenu peut contenir des | apres le premier
        let raw = "Label|Content with | pipe";
        let templates = parse_templates(raw);
        assert_eq!(templates.len(), 1);
        assert_eq!(templates[0].content, "Content with | pipe");
    }
}
