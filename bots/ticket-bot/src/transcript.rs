#![allow(dead_code)]
use crate::api_client::{Ticket, TicketMessage};

/// Genere un transcript au format Markdown.
pub fn generate_markdown(ticket: &Ticket, messages: &[TicketMessage], sla_info: Option<&str>) -> String {
    let mut md = format!(
        "# Transcript du ticket #{short_id}\n\n\
         | Champ | Valeur |\n\
         |-------|--------|\n\
         | **Sujet** | {title} |\n\
         | **Type** | {category} |\n\
         | **Priorite** | {priority} |\n\
         | **Auteur** | {author} |\n\
         | **Statut** | Ferme |\n\
         | **Cree le** | {created} |\n",
        short_id = &ticket.id[..8.min(ticket.id.len())],
        title = ticket.title,
        category = ticket.category,
        priority = ticket.priority,
        author = ticket.author_name,
        created = ticket.created_at,
    );

    if let Some(assigned) = &ticket.assigned_to {
        md.push_str(&format!("| **Assigne a** | {} |\n", assigned));
    }

    if let Some(sla) = sla_info {
        md.push_str(&format!("\n## SLA\n\n{}\n", sla));
    }

    md.push_str("\n---\n\n## Messages\n\n");

    if messages.is_empty() {
        md.push_str("_Aucun message dans ce ticket._\n");
    } else {
        for msg in messages {
            let role_badge = match msg.author_role.as_str() {
                "moderator" => "[Staff]",
                _ => "[User]",
            };
            md.push_str(&format!(
                "### {} {} — {}\n\n{}\n\n",
                role_badge, msg.author_name, msg.created_at, msg.content
            ));
        }
    }

    md
}

/// Genere un transcript au format HTML avec CSS inline.
pub fn generate_html(ticket: &Ticket, messages: &[TicketMessage], sla_info: Option<&str>) -> String {
    let mut html = format!(
        r#"<!DOCTYPE html>
<html lang="fr">
<head>
<meta charset="utf-8">
<title>Transcript — #{short_id}</title>
<style>
body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; max-width: 800px; margin: 40px auto; padding: 0 20px; background: #1a1a2e; color: #e0e0e0; }}
h1 {{ color: #7289da; border-bottom: 2px solid #7289da; padding-bottom: 10px; }}
table {{ border-collapse: collapse; width: 100%; margin: 15px 0; }}
th, td {{ text-align: left; padding: 8px 12px; border: 1px solid #333; }}
th {{ background: #16213e; color: #7289da; }}
td {{ background: #0f3460; }}
.message {{ margin: 15px 0; padding: 12px 16px; border-radius: 8px; border-left: 4px solid #555; }}
.message.staff {{ border-left-color: #f39c12; background: #1a1a2e; }}
.message.user {{ border-left-color: #3498db; background: #16213e; }}
.author {{ font-weight: bold; margin-bottom: 4px; }}
.role {{ font-size: 0.8em; padding: 2px 6px; border-radius: 3px; }}
.role.staff {{ background: #f39c12; color: #000; }}
.role.user {{ background: #3498db; color: #fff; }}
.time {{ color: #888; font-size: 0.85em; }}
.empty {{ color: #888; font-style: italic; }}
.sla {{ background: #16213e; padding: 12px; border-radius: 6px; margin: 15px 0; }}
</style>
</head>
<body>
<h1>Transcript du ticket #{short_id}</h1>
<table>
<tr><th>Champ</th><th>Valeur</th></tr>
<tr><td>Sujet</td><td>{title}</td></tr>
<tr><td>Type</td><td>{category}</td></tr>
<tr><td>Priorite</td><td>{priority}</td></tr>
<tr><td>Auteur</td><td>{author}</td></tr>
<tr><td>Statut</td><td>Ferme</td></tr>
<tr><td>Cree le</td><td>{created}</td></tr>"#,
        short_id = html_escape(&ticket.id[..8.min(ticket.id.len())]),
        title = html_escape(&ticket.title),
        category = html_escape(&ticket.category),
        priority = html_escape(&ticket.priority),
        author = html_escape(&ticket.author_name),
        created = html_escape(&ticket.created_at),
    );

    if let Some(assigned) = &ticket.assigned_to {
        html.push_str(&format!(
            "<tr><td>Assigne a</td><td>{}</td></tr>",
            html_escape(assigned)
        ));
    }

    html.push_str("</table>\n");

    if let Some(sla) = sla_info {
        html.push_str(&format!("<div class=\"sla\"><strong>SLA :</strong> {}</div>\n", html_escape(sla)));
    }

    html.push_str("<h2>Messages</h2>\n");

    if messages.is_empty() {
        html.push_str("<p class=\"empty\">Aucun message dans ce ticket.</p>\n");
    } else {
        for msg in messages {
            let (role_class, role_label) = match msg.author_role.as_str() {
                "moderator" => ("staff", "Staff"),
                _ => ("user", "User"),
            };
            html.push_str(&format!(
                "<div class=\"message {role_class}\">\
                <div class=\"author\">\
                <span class=\"role {role_class}\">{role_label}</span> {author} \
                <span class=\"time\">{time}</span>\
                </div>\
                <div>{content}</div>\
                </div>\n",
                role_class = role_class,
                role_label = role_label,
                author = html_escape(&msg.author_name),
                time = html_escape(&msg.created_at),
                content = html_escape(&msg.content),
            ));
        }
    }

    html.push_str("</body>\n</html>");
    html
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Formate une duree en minutes en texte lisible.
pub fn format_duration_minutes(minutes: u64) -> String {
    if minutes < 60 {
        format!("{}min", minutes)
    } else {
        let h = minutes / 60;
        let m = minutes % 60;
        if m == 0 {
            format!("{}h", h)
        } else {
            format!("{}h{}min", h, m)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_ticket() -> Ticket {
        Ticket {
            id: "abcdef12-3456-7890-abcd-ef1234567890".to_string(),
            title: "Mon probleme".to_string(),
            status: "closed".to_string(),
            priority: "medium".to_string(),
            author_id: "123456".to_string(),
            author_name: "Alice".to_string(),
            assigned_to: Some("Bob".to_string()),
            server: "guild1".to_string(),
            category: "question".to_string(),
            ticket_type: Some("question".to_string()),
            channel_id: None,
            voice_channel_id: None,
            invited_user_id: None,
            created_at: "2025-01-15T10:00:00Z".to_string(),
            updated_at: "2025-01-15T12:00:00Z".to_string(),
            messages_count: 2,
            first_response_at: None,
            resolved_at: None,
            satisfaction_rating: None,
        }
    }

    fn make_messages() -> Vec<TicketMessage> {
        vec![
            TicketMessage {
                id: "msg1".to_string(),
                ticket_id: "abcdef12".to_string(),
                author_name: "Alice".to_string(),
                author_role: "user".to_string(),
                content: "Bonjour, j'ai un souci".to_string(),
                created_at: "2025-01-15T10:05:00Z".to_string(),
            },
            TicketMessage {
                id: "msg2".to_string(),
                ticket_id: "abcdef12".to_string(),
                author_name: "Bob".to_string(),
                author_role: "moderator".to_string(),
                content: "Je regarde ca".to_string(),
                created_at: "2025-01-15T10:10:00Z".to_string(),
            },
        ]
    }

    #[test]
    fn markdown_contains_header() {
        let md = generate_markdown(&make_ticket(), &make_messages(), None);
        assert!(md.contains("# Transcript du ticket #abcdef12"));
        assert!(md.contains("Mon probleme"));
        assert!(md.contains("Alice"));
    }

    #[test]
    fn markdown_contains_messages() {
        let md = generate_markdown(&make_ticket(), &make_messages(), None);
        assert!(md.contains("[Staff] Bob"));
        assert!(md.contains("[User] Alice"));
        assert!(md.contains("Bonjour, j'ai un souci"));
        assert!(md.contains("Je regarde ca"));
    }

    #[test]
    fn markdown_empty_messages() {
        let md = generate_markdown(&make_ticket(), &[], None);
        assert!(md.contains("Aucun message"));
    }

    #[test]
    fn markdown_with_sla() {
        let md = generate_markdown(&make_ticket(), &[], Some("Premiere reponse: 5min"));
        assert!(md.contains("## SLA"));
        assert!(md.contains("Premiere reponse: 5min"));
    }

    #[test]
    fn html_contains_structure() {
        let html = generate_html(&make_ticket(), &make_messages(), None);
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("Transcript du ticket #abcdef12"));
        assert!(html.contains("Mon probleme"));
    }

    #[test]
    fn html_contains_messages() {
        let html = generate_html(&make_ticket(), &make_messages(), None);
        assert!(html.contains("Staff"));
        assert!(html.contains("Bob"));
        assert!(html.contains("Je regarde ca"));
    }

    #[test]
    fn html_escapes_xss() {
        let mut ticket = make_ticket();
        ticket.title = "<script>alert('xss')</script>".to_string();
        let html = generate_html(&ticket, &[], None);
        assert!(!html.contains("<script>"));
        assert!(html.contains("&lt;script&gt;"));
    }

    #[test]
    fn html_empty_messages() {
        let html = generate_html(&make_ticket(), &[], None);
        assert!(html.contains("Aucun message"));
    }

    #[test]
    fn format_duration_minutes_only() {
        assert_eq!(format_duration_minutes(5), "5min");
        assert_eq!(format_duration_minutes(45), "45min");
    }

    #[test]
    fn format_duration_hours() {
        assert_eq!(format_duration_minutes(60), "1h");
        assert_eq!(format_duration_minutes(120), "2h");
    }

    #[test]
    fn format_duration_mixed() {
        assert_eq!(format_duration_minutes(90), "1h30min");
        assert_eq!(format_duration_minutes(125), "2h5min");
    }
}
