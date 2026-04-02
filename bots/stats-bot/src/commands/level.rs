use serenity::all::{
    CommandDataOptionValue, CommandInteraction, CommandOptionType, Context, CreateCommand,
    CreateCommandOption, CreateEmbed, CreateInteractionResponse,
    CreateInteractionResponseMessage,
};
use tracing::error;

use crate::handler::StatsApiKey;

pub fn register() -> CreateCommand {
    CreateCommand::new("level")
        .description("Consulter les niveaux et l'XP")
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "user",
                "Niveau d'un utilisateur",
            )
            .add_sub_option(
                CreateCommandOption::new(
                    CommandOptionType::User,
                    "target",
                    "Utilisateur cible (par defaut : vous)",
                ),
            ),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "top",
                "Classement des niveaux",
            )
            .add_sub_option(
                CreateCommandOption::new(
                    CommandOptionType::Integer,
                    "limit",
                    "Nombre de membres (1-25, defaut 10)",
                )
                .min_int_value(1)
                .max_int_value(25),
            ),
        )
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    let guild_id = match command.guild_id {
        Some(id) => id.to_string(),
        None => {
            respond(ctx, command, "Cette commande doit etre utilisee dans un serveur.").await;
            return;
        }
    };

    let sub = match command.data.options.first() {
        Some(s) => s,
        None => {
            respond(ctx, command, "Erreur : sous-commande manquante.").await;
            return;
        }
    };

    match sub.name.as_str() {
        "user" => handle_user(ctx, command, &guild_id).await,
        "top" => handle_top(ctx, command, &guild_id).await,
        _ => {}
    }
}

/// Extrait les sous-options d'une sous-commande de maniere safe.
fn get_sub_options(command: &CommandInteraction) -> Option<&Vec<serenity::all::CommandDataOption>> {
    command.data.options.first().and_then(|opt| match &opt.value {
        CommandDataOptionValue::SubCommand(opts) => Some(opts),
        _ => None,
    })
}

async fn handle_user(ctx: &Context, command: &CommandInteraction, guild_id: &str) {
    let sub_options = match get_sub_options(command) {
        Some(opts) => opts,
        None => return,
    };

    let target_id = sub_options
        .iter()
        .find(|o| o.name == "target")
        .and_then(|o| match &o.value {
            CommandDataOptionValue::User(id) => Some(id.to_string()),
            _ => None,
        })
        .unwrap_or_else(|| command.user.id.to_string());

    let data = ctx.data.read().await;
    let api = match data.get::<StatsApiKey>() {
        Some(a) => a,
        None => return,
    };

    match api.get_user_level(guild_id, &target_id).await {
        Ok(Some(level)) => {
            let progress_bar = make_progress_bar(level.xp_current, level.xp_needed);
            let embed = CreateEmbed::new()
                .title(format!("Niveau de {}", level.username))
                .color(0x5865F2)
                .field("Niveau", format!("**{}**", level.level), true)
                .field("XP Total", format!("{}", level.xp), true)
                .field(
                    "Progression",
                    format!("{}\n{}/{} XP", progress_bar, level.xp_current, level.xp_needed),
                    false,
                );

            let msg = CreateInteractionResponseMessage::new().embed(embed);
            let response = CreateInteractionResponse::Message(msg);
            if let Err(e) = command.create_response(&ctx.http, response).await {
                error!(error = %e, "Erreur reponse level user");
            }
        }
        Ok(None) => {
            respond(ctx, command, "Cet utilisateur n'a pas encore d'XP.").await;
        }
        Err(e) => {
            error!(error = %e, "Erreur API level user");
            respond(ctx, command, "Erreur lors de la recuperation du niveau.").await;
        }
    }
}

async fn handle_top(ctx: &Context, command: &CommandInteraction, guild_id: &str) {
    let sub_options = match get_sub_options(command) {
        Some(opts) => opts,
        None => return,
    };

    let limit = sub_options
        .iter()
        .find(|o| o.name == "limit")
        .and_then(|o| match &o.value {
            CommandDataOptionValue::Integer(n) => Some((*n as u32).clamp(1, 25)),
            _ => None,
        })
        .unwrap_or(10);

    let data = ctx.data.read().await;
    let api = match data.get::<StatsApiKey>() {
        Some(a) => a,
        None => return,
    };

    match api.get_level_leaderboard(guild_id, limit).await {
        Ok(leaderboard) => {
            if leaderboard.is_empty() {
                respond(ctx, command, "Aucun membre n'a encore d'XP.").await;
                return;
            }

            let mut desc = String::new();
            for (i, user) in leaderboard.iter().enumerate() {
                desc.push_str(&format!(
                    "{}. **{}** — Niv. {} ({} XP)\n",
                    i + 1, user.username, user.level, user.xp
                ));
            }

            let embed = CreateEmbed::new()
                .title("Classement des niveaux")
                .description(desc)
                .color(0x57F287);

            let msg = CreateInteractionResponseMessage::new().embed(embed);
            let response = CreateInteractionResponse::Message(msg);
            if let Err(e) = command.create_response(&ctx.http, response).await {
                error!(error = %e, "Erreur reponse level top");
            }
        }
        Err(e) => {
            error!(error = %e, "Erreur API level leaderboard");
            respond(ctx, command, "Erreur lors de la recuperation du classement.").await;
        }
    }
}

/// Genere une barre de progression ASCII.
pub fn make_progress_bar(current: i64, needed: i64) -> String {
    let pct = if needed > 0 {
        (current as f64 / needed as f64).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let filled = (pct * 10.0) as usize;
    let empty = 10 - filled;
    format!(
        "[{}{}] {:.0}%",
        "=".repeat(filled),
        " ".repeat(empty),
        pct * 100.0
    )
}

async fn respond(ctx: &Context, command: &CommandInteraction, content: &str) {
    let msg = CreateInteractionResponseMessage::new()
        .content(content)
        .ephemeral(true);
    let response = CreateInteractionResponse::Message(msg);
    let _ = command.create_response(&ctx.http, response).await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_bar_zero() {
        let bar = make_progress_bar(0, 100);
        assert_eq!(bar, "[          ] 0%");
    }

    #[test]
    fn test_progress_bar_full() {
        let bar = make_progress_bar(100, 100);
        assert_eq!(bar, "[==========] 100%");
    }

    #[test]
    fn test_progress_bar_half() {
        let bar = make_progress_bar(50, 100);
        assert_eq!(bar, "[=====     ] 50%");
    }

    #[test]
    fn test_progress_bar_over_100() {
        // Current > needed : clampe a 100%
        let bar = make_progress_bar(150, 100);
        assert_eq!(bar, "[==========] 100%");
    }

    #[test]
    fn test_progress_bar_needed_zero() {
        // Division par zero protegee
        let bar = make_progress_bar(50, 0);
        assert_eq!(bar, "[          ] 0%");
    }

    #[test]
    fn test_progress_bar_negative_current() {
        let bar = make_progress_bar(-10, 100);
        assert_eq!(bar, "[          ] 0%");
    }

    #[test]
    fn test_progress_bar_30_percent() {
        let bar = make_progress_bar(30, 100);
        assert_eq!(bar, "[===       ] 30%");
    }

    #[test]
    fn test_progress_bar_99_percent() {
        let bar = make_progress_bar(99, 100);
        // 99% → 9.9 → 9 filled, 1 empty
        assert_eq!(bar, "[========= ] 99%");
    }
}
