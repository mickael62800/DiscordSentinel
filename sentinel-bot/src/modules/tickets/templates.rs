use serenity::builder::{
    CreateActionRow, CreateSelectMenu, CreateSelectMenuKind, CreateSelectMenuOption,
};

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
    crate::shared::parsers::parse_pipe_lines(raw)
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

    let select =
        CreateSelectMenu::new(TEMPLATE_SELECT_ID, CreateSelectMenuKind::String { options })
            .placeholder("Choisissez une reponse rapide...");

    CreateActionRow::SelectMenu(select)
}
