use serenity::all::{
    CommandDataOptionValue, CommandInteraction, CommandOptionType, Context, CreateCommand,
    CreateCommandOption, CreateEmbed, CreateEmbedFooter, CreateInteractionResponse,
    CreateInteractionResponseMessage,
};

use crate::game::classes;
use crate::game::progression;
use crate::handler::{GameDbKey, load_guild_config};

pub fn register() -> CreateCommand {
    CreateCommand::new("profil")
        .description("Affiche ton profil Coup de Coude")
        .add_option(
            CreateCommandOption::new(CommandOptionType::User, "user", "Joueur a consulter")
                .required(false),
        )
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    let guild_id = match command.guild_id {
        Some(id) => id.to_string(),
        None => {
            reply_ephemeral(ctx, command, "Commande serveur uniquement.").await;
            return;
        }
    };

    let config = load_guild_config(ctx, &guild_id).await;
    if !crate::channel_check::check_channel(ctx, command, config.channel_profil()).await {
        return;
    }
    if !config.enabled() {
        reply_ephemeral(ctx, command, "Le jeu Coup de Coude est desactive sur ce serveur.").await;
        return;
    }

    let target_user = command
        .data
        .options
        .iter()
        .find(|o| o.name == "user")
        .and_then(|o| match &o.value {
            CommandDataOptionValue::User(id) => Some(*id),
            _ => None,
        })
        .unwrap_or(command.user.id);

    let target = match target_user.to_user(&ctx.http).await {
        Ok(u) => u,
        Err(_) => {
            reply_ephemeral(ctx, command, "Utilisateur introuvable.").await;
            return;
        }
    };

    let data = ctx.data.read().await;
    let db = data.get::<GameDbKey>().unwrap();

    let player = match db.get_or_create_player(&guild_id, &target.id.to_string(), &target.name).await {
        Ok(p) => p,
        Err(e) => {
            reply_ephemeral(ctx, command, &format!("Erreur base de donnees : {e}")).await;
            return;
        }
    };

    let class = classes::get_class(&player.class);
    let title = progression::title_for_level(player.level);

    let effective_atk = class.base_atk + (player.level - 1) * class.atk_growth + player.atk;
    let effective_def = class.base_def + (player.level - 1) * class.def_growth + player.def;
    let hp = progression::display_hp(effective_def);

    let xp_needed = if player.level >= progression::MAX_LEVEL {
        0
    } else {
        progression::xp_for_level(player.level)
    };

    let xp_display = if player.level >= progression::MAX_LEVEL {
        "MAX".to_string()
    } else {
        format!("{} / {}", player.xp, xp_needed)
    };

    let class_name_cap = {
        let mut c = class.name.chars();
        match c.next() {
            None => String::new(),
            Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        }
    };

    let embed = CreateEmbed::new()
        .title(format!(
            "\u{2694}\u{fe0f} {} — {} Niv.{} \u{300c}{}\u{300d}",
            target.name, class_name_cap, player.level, title
        ))
        .color(0x3498DB)
        .thumbnail(target.face())
        .description(format!(
            "\u{2764}\u{fe0f} HP: **{}**  |  \u{2694}\u{fe0f} ATK: **{}**  |  \u{1f6e1}\u{fe0f} DEF: **{}**\n\
             \u{1fa99} **{}** coins  |  \u{1f3c6} {}W / {}L / {}D\n\
             \u{1f4ca} XP: {}  |  \u{1f3af} Points: **{}**\n\
             \u{1f414} Lachete: {}  |  \u{1f300} Chaos: {}",
            hp,
            effective_atk,
            effective_def,
            player.coins,
            player.total_wins,
            player.total_losses,
            player.total_draws,
            xp_display,
            player.stat_points,
            player.cowardice_count,
            player.chaos_events,
        ))
        .field(
            "Classe",
            format!("{} **{}** — {}", class.emoji, class.name, class.description),
            false,
        )
        .field(
            "\u{1f4b0} Gains/Pertes",
            format!("+{} / -{}", player.total_earned, player.total_lost),
            true,
        )
        .field(
            "\u{1f5e1}\u{fe0f} Total vole",
            format!("{}", player.total_stolen),
            true,
        )
        .field(
            "\u{1f3b0} Casino W/L",
            format!("{}/{}", player.casino_wins, player.casino_losses),
            true,
        )
        .footer(CreateEmbedFooter::new("Coup de Coude | Sentinel"))
        .timestamp(serenity::model::Timestamp::now());

    if let Err(e) = command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new().embed(embed),
            ),
        )
        .await
    {
        tracing::warn!(error = %e, "Echec response Discord");
    }
}

async fn reply_ephemeral(ctx: &Context, command: &CommandInteraction, content: &str) {
    if let Err(e) = command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content(content)
                    .ephemeral(true),
            ),
        )
        .await
    {
        tracing::warn!(error = %e, "Echec response Discord");
    }
}
