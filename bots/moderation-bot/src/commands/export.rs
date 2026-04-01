use serenity::all::{
    CommandDataOptionValue, CommandInteraction, CommandOptionType, Context, CreateAttachment,
    CreateCommand, CreateCommandOption, CreateInteractionResponse,
    CreateInteractionResponseMessage,
};
use tracing::error;

use crate::api_client::ModerationActionResponse;
use crate::handler::ModerationApiKey;

pub fn register() -> CreateCommand {
    CreateCommand::new("export")
        .description("Exporter l'historique de moderation d'un utilisateur")
        .add_option(
            CreateCommandOption::new(CommandOptionType::User, "user", "Utilisateur dont exporter l'historique")
                .required(true),
        )
        .add_option(
            CreateCommandOption::new(CommandOptionType::String, "format", "Format d'export")
                .add_string_choice("JSON", "json")
                .add_string_choice("CSV", "csv"),
        )
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    let target_id = command.data.options.iter().find(|o| o.name == "user")
        .and_then(|o| match &o.value { CommandDataOptionValue::User(id) => Some(*id), _ => None })
        .unwrap();

    let format = command.data.options.iter().find(|o| o.name == "format")
        .and_then(|o| match &o.value { CommandDataOptionValue::String(s) => Some(s.as_str()), _ => None })
        .unwrap_or("json");

    let guild_id = match command.guild_id {
        Some(id) => id,
        None => { sentinel_shared::discord_helpers::reply_ephemeral(ctx, command, "Commande serveur uniquement.").await; return; }
    };

    let data = ctx.data.read().await;
    let api = data.get::<ModerationApiKey>().unwrap();

    match api.get_history(&guild_id.to_string(), &target_id.to_string()).await {
        Ok(history) => {
            let (content, filename) = match format {
                "csv" => (
                    generate_csv(&history.actions),
                    format!("history_{}_{}.csv", target_id, guild_id),
                ),
                _ => (
                    generate_json(&history.actions),
                    format!("history_{}_{}.json", target_id, guild_id),
                ),
            };

            // Repondre d'abord (ephemere)
            command.create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content(format!("Historique de <@{}> ({} actions) — fichier en cours d'envoi...", target_id, history.actions.len()))
                        .ephemeral(true),
                ),
            ).await.ok();

            // Envoyer le fichier en followup
            let attachment = CreateAttachment::bytes(content.into_bytes(), filename);
            let followup = serenity::builder::CreateInteractionResponseFollowup::new()
                .content(format!("Export de {} actions :", history.actions.len()))
                .add_file(attachment)
                .ephemeral(true);
            command.create_followup(&ctx.http, followup).await.ok();
        }
        Err(e) => {
            error!(error = %e, "Erreur export historique");
            sentinel_shared::discord_helpers::reply_ephemeral(ctx, command, &format!("Erreur : {e}")).await;
        }
    }
}

/// Genere un CSV a partir des actions de moderation.
pub fn generate_csv(actions: &[ModerationActionResponse]) -> String {
    let mut csv = String::from("id,type,raison,escalation\n");
    for action in actions {
        csv.push_str(&format!(
            "{},{},{},{}\n",
            csv_escape(&action.id),
            csv_escape(&action.action_type),
            csv_escape(&action.reason),
            action.escalation_action.as_deref().unwrap_or(""),
        ));
    }
    csv
}

/// Genere un JSON a partir des actions de moderation.
pub fn generate_json(actions: &[ModerationActionResponse]) -> String {
    let entries: Vec<serde_json::Value> = actions
        .iter()
        .map(|a| {
            serde_json::json!({
                "id": a.id,
                "type": a.action_type,
                "reason": a.reason,
                "escalation": a.escalation_action,
                "strikes": a.strikes_count,
            })
        })
        .collect();

    serde_json::to_string_pretty(&entries).unwrap_or_else(|_| "[]".to_string())
}

fn csv_escape(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    fn make_actions() -> Vec<ModerationActionResponse> {
        vec![
            ModerationActionResponse {
                id: "action-1".to_string(),
                action_type: "warn".to_string(),
                target_name: "Alice".to_string(),
                reason: "Spam".to_string(),
                escalation_action: None,
                escalation_duration: None,
                strikes_count: Some(1),
            },
            ModerationActionResponse {
                id: "action-2".to_string(),
                action_type: "mute".to_string(),
                target_name: "Alice".to_string(),
                reason: "Insulte, comportement toxique".to_string(),
                escalation_action: Some("mute".to_string()),
                escalation_duration: Some(600),
                strikes_count: Some(3),
            },
        ]
    }

    #[test]
    fn csv_has_header() {
        let csv = generate_csv(&make_actions());
        assert!(csv.starts_with("id,type,raison,escalation\n"));
    }

    #[test]
    fn csv_has_rows() {
        let csv = generate_csv(&make_actions());
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines.len(), 3); // header + 2 rows
        assert!(lines[1].contains("warn"));
        assert!(lines[2].contains("mute"));
    }

    #[test]
    fn csv_escapes_commas() {
        let actions = vec![ModerationActionResponse {
            id: "1".to_string(),
            action_type: "warn".to_string(),
            target_name: "Bob".to_string(),
            reason: "Raison avec, virgule".to_string(),
            escalation_action: None,
            escalation_duration: None,
            strikes_count: None,
        }];
        let csv = generate_csv(&actions);
        assert!(csv.contains("\"Raison avec, virgule\""));
    }

    #[test]
    fn csv_empty() {
        let csv = generate_csv(&[]);
        assert_eq!(csv, "id,type,raison,escalation\n");
    }

    #[test]
    fn json_valid() {
        let json = generate_json(&make_actions());
        let parsed: Vec<serde_json::Value> = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0]["type"], "warn");
    }

    #[test]
    fn json_empty() {
        let json = generate_json(&[]);
        let parsed: Vec<serde_json::Value> = serde_json::from_str(&json).unwrap();
        assert!(parsed.is_empty());
    }

    #[test]
    fn json_includes_escalation() {
        let json = generate_json(&make_actions());
        let parsed: Vec<serde_json::Value> = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed[1]["escalation"], "mute");
        assert_eq!(parsed[1]["strikes"], 3);
    }
}
