use serenity::all::{
    CommandDataOptionValue, CommandInteraction, CommandOptionType, Context, CreateAttachment,
    CreateCommand, CreateCommandOption, CreateInteractionResponse,
    CreateInteractionResponseMessage,
};
use tracing::{error, warn};

use super::api_client::ModerationActionResponse;
use super::ModerationApiKey;

pub fn register() -> CreateCommand {
    CreateCommand::new("export")
        .description("Exporter l'historique de moderation d'un utilisateur")
        .default_member_permissions(serenity::all::Permissions::MODERATE_MEMBERS)
        .add_option(
            CreateCommandOption::new(CommandOptionType::String, "format", "Format d'export")
                .add_string_choice("JSON", "json")
                .add_string_choice("CSV", "csv"),
        )
        .add_option(CreateCommandOption::new(
            CommandOptionType::User,
            "user",
            "Utilisateur dont exporter l'historique (ou user_id)",
        ))
        .add_option(CreateCommandOption::new(
            CommandOptionType::String,
            "user_id",
            "ID de l'utilisateur (ex. membre parti / banni)",
        ))
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    if !super::has_mod_permission(command, serenity::all::Permissions::MODERATE_MEMBERS) {
        let _ = command
            .create_response(
                &ctx.http,
                serenity::all::CreateInteractionResponse::Message(
                    serenity::all::CreateInteractionResponseMessage::new()
                        .content("❌ Permission de modération requise.")
                        .ephemeral(true),
                ),
            )
            .await;
        return;
    }
    let target_id = match super::resolve_target_user_id(command, "user") {
        Some(id) => id,
        None => {
            crate::shared::discord_helpers::reply_ephemeral(
                ctx,
                command,
                "Indique un membre (`user`) ou un identifiant (`user_id`).",
            )
            .await;
            return;
        }
    };

    let format = command
        .data
        .options
        .iter()
        .find(|o| o.name == "format")
        .and_then(|o| match &o.value {
            CommandDataOptionValue::String(s) => Some(s.as_str()),
            _ => None,
        })
        .unwrap_or("json");

    let guild_id = match command.guild_id {
        Some(id) => id,
        None => {
            crate::shared::discord_helpers::reply_ephemeral(
                ctx,
                command,
                "Commande serveur uniquement.",
            )
            .await;
            return;
        }
    };

    let data = ctx.data.read().await;
    let api = match data.get::<ModerationApiKey>() {
        Some(a) => a,
        None => {
            tracing::error!("ModerationApiKey manquant");
            return;
        }
    };

    match api
        .get_history(&guild_id.to_string(), &target_id.to_string())
        .await
    {
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

            if let Err(e) = command
                .create_response(
                    &ctx.http,
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new()
                            .content(format!(
                                "Historique de <@{}> ({} actions) — fichier en cours d'envoi...",
                                target_id,
                                history.actions.len()
                            ))
                            .ephemeral(true),
                    ),
                )
                .await
            {
                warn!(error = %e, "Failed to send export initial response");
            }

            let attachment = CreateAttachment::bytes(content.into_bytes(), filename);
            let followup = serenity::builder::CreateInteractionResponseFollowup::new()
                .content(format!("Export de {} actions :", history.actions.len()))
                .add_file(attachment)
                .ephemeral(true);
            if let Err(e) = command.create_followup(&ctx.http, followup).await {
                warn!(error = %e, "Failed to send export followup with file");
            }
        }
        Err(e) => {
            error!(error = %e, "Erreur export historique");
            crate::shared::discord_helpers::reply_ephemeral(ctx, command, &format!("Erreur : {e}"))
                .await;
        }
    }
}

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
                moderator_name: "Mod1".to_string(),
                gravity: None,
                created_at: "2026-01-01T00:00:00Z".to_string(),
                target_name: "Alice".to_string(),
                reason: "Spam".to_string(),
                escalation_action: None,
                escalation_duration: None,
                strikes_count: Some(1),
            },
            ModerationActionResponse {
                id: "action-2".to_string(),
                action_type: "mute".to_string(),
                moderator_name: "Mod1".to_string(),
                gravity: Some("medium".to_string()),
                created_at: "2026-01-02T00:00:00Z".to_string(),
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
        assert_eq!(lines.len(), 3);
        assert!(lines[1].contains("warn"));
        assert!(lines[2].contains("mute"));
    }

    #[test]
    fn csv_escapes_commas() {
        let actions = vec![ModerationActionResponse {
            id: "1".to_string(),
            action_type: "warn".to_string(),
            moderator_name: "Mod1".to_string(),
            gravity: None,
            created_at: "2026-01-01T00:00:00Z".to_string(),
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
