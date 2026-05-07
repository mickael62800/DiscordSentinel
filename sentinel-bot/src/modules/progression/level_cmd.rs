use serenity::all::{
    CommandDataOptionValue, CommandInteraction, CommandOptionType, Context, CreateCommand,
    CreateCommandOption, CreateEmbed, CreateInteractionResponse,
    CreateInteractionResponseMessage,
};
use tracing::error;

use crate::shared::discord_helpers::reply_ephemeral as respond;

use super::StatsApiKey;

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
                "Classement global (XP total)",
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
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "top_text",
                "Classement XP texte",
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
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "top_voice",
                "Classement XP vocal",
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

    let sub = &command.data.options[0];

    match sub.name.as_str() {
        "user" => handle_user(ctx, command, &guild_id).await,
        "top" => handle_top(ctx, command, &guild_id, None, "Classement global").await,
        "top_text" => handle_top(ctx, command, &guild_id, Some("text"), "Classement Texte").await,
        "top_voice" => handle_top(ctx, command, &guild_id, Some("voice"), "Classement Vocal").await,
        _ => {}
    }
}

async fn handle_user(ctx: &Context, command: &CommandInteraction, guild_id: &str) {
    let sub_options = match &command.data.options[0].value {
        CommandDataOptionValue::SubCommand(opts) => opts,
        _ => return,
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
            let text_bar = make_progress_bar(level.xp_text_current, level.xp_text_needed);
            let voice_bar = make_progress_bar(level.xp_voice_current, level.xp_voice_needed);

            let embed = CreateEmbed::new()
                .title(format!("Niveaux de {}", level.username))
                .color(0x5865F2)
                .field(
                    "\u{1f4dd} Texte",
                    format!(
                        "**Niveau {}**\n{}\n{}/{} XP ({} XP total)",
                        level.level_text, text_bar, level.xp_text_current, level.xp_text_needed, level.xp_text
                    ),
                    true,
                )
                .field(
                    "\u{1f3a4} Vocal",
                    format!(
                        "**Niveau {}**\n{}\n{}/{} XP ({} XP total)",
                        level.level_voice, voice_bar, level.xp_voice_current, level.xp_voice_needed, level.xp_voice
                    ),
                    true,
                )
                .field(
                    "\u{1f4ca} Global",
                    format!("Niveau **{}** | {} XP total", level.level, level.xp),
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

async fn handle_top(ctx: &Context, command: &CommandInteraction, guild_id: &str, source: Option<&str>, title: &str) {
    let sub_options = match &command.data.options[0].value {
        CommandDataOptionValue::SubCommand(opts) => opts,
        _ => return,
    };

    let limit = sub_options
        .iter()
        .find(|o| o.name == "limit")
        .and_then(|o| match &o.value {
            CommandDataOptionValue::Integer(n) => Some(*n as u32),
            _ => None,
        })
        .unwrap_or(10);

    let data = ctx.data.read().await;
    let api = match data.get::<StatsApiKey>() {
        Some(a) => a,
        None => return,
    };

    match api.get_level_leaderboard(guild_id, limit, source).await {
        Ok(leaderboard) => {
            if leaderboard.is_empty() {
                respond(ctx, command, "Aucun membre n'a encore d'XP.").await;
                return;
            }

            let mut desc = String::new();
            for (i, user) in leaderboard.iter().enumerate() {
                let medal = match i {
                    0 => "\u{1f947} ",
                    1 => "\u{1f948} ",
                    2 => "\u{1f949} ",
                    _ => "",
                };

                let (level_display, xp_display) = match source {
                    Some("text") => (user.level_text, user.xp_text),
                    Some("voice") => (user.level_voice, user.xp_voice),
                    _ => (user.level, user.xp),
                };

                desc.push_str(&format!(
                    "{}{}. **{}** — Niv. {} ({} XP)\n",
                    medal, i + 1, user.username, level_display, xp_display
                ));
            }

            let (emoji, color) = match source {
                Some("text") => ("\u{1f4dd} ", 0x3498DB),
                Some("voice") => ("\u{1f3a4} ", 0xE91E63),
                _ => ("", 0x57F287),
            };

            let embed = CreateEmbed::new()
                .title(format!("{}{}", emoji, title))
                .description(desc)
                .color(color);

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

fn make_progress_bar(current: i64, needed: i64) -> String {
    let pct = if needed > 0 {
        (current as f64 / needed as f64).min(1.0)
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

