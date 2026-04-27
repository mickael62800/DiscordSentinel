//! Commande /no-taunts on|off (Phase 9 Part D).
//!
//! Thin : appelle l'API pour set l'opt-out. Aucune logique cote bot.

use serenity::all::{
    CommandDataOptionValue, CommandInteraction, CommandOptionType, Context, CreateCommand,
    CreateCommandOption,
};

use sentinel_shared::discord_helpers::{reply_ephemeral, require_guild_id, reply_api_err};

use crate::modules::coude::GameApiKey;

pub fn register() -> CreateCommand {
    CreateCommand::new("no-taunts")
        .description("Active/desactive les railleries automatiques te concernant")
        .add_option(
            CreateCommandOption::new(CommandOptionType::String, "etat", "on ou off")
                .required(true)
                .add_string_choice("on — je ne veux plus etre raille", "on")
                .add_string_choice("off — railleries activees", "off"),
        )
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    let Some(guild_id) = require_guild_id(ctx, command).await else { return; };

    let state = command
        .data
        .options
        .iter()
        .find(|o| o.name == "etat")
        .and_then(|o| match &o.value {
            CommandDataOptionValue::String(s) => Some(s.clone()),
            _ => None,
        });
    let opted_out = match state.as_deref() {
        Some("on") => true,
        Some("off") => false,
        _ => {
            reply_ephemeral(ctx, command, "Parametre invalide.").await;
            return;
        }
    };

    let user_id = command.user.id.to_string();

    let data = ctx.data.read().await;
    let api = match data.get::<GameApiKey>() {
        Some(a) => a,
        None => return,
    };

    if let Err(e) = api.set_taunts_opt_out(&guild_id, &user_id, opted_out).await {
        reply_api_err(ctx, command, e).await;
        return;
    }

    let msg = if opted_out {
        "\u{1f4ac} Tu n'apparaitras plus dans les railleries automatiques. Repose en paix."
    } else {
        "\u{1f4e3} Les railleries automatiques sont reactivees pour toi. Bonne chance."
    };
    reply_ephemeral(ctx, command, msg).await;
}
