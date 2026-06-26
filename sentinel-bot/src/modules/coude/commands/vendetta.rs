//! Commande `/vendetta` — declare une vengeance officielle (cf. roadmap 5.3).
//!
//! Implementation light : declaration + persist + annonce. La resolution
//! mecanique (bonus +100% sur revanche victorieuse, rename "Bourreau" sur
//! revanche perdue) est branchee dans le service de combat — pour l instant
//! seul le contrat est en place.

use serenity::all::{
    CommandDataOptionValue, CommandInteraction, CommandOptionType, Context, CreateCommand,
    CreateCommandOption, CreateEmbed, CreateEmbedFooter,
};

use crate::shared::discord_helpers::{reply_ephemeral, require_guild_id, reply_api_err};

use crate::modules::coude::load_guild_config;
use crate::modules::coude::GameApiKey;

pub fn register() -> CreateCommand {
    CreateCommand::new("vendetta")
        .description("Declare une vendetta officielle contre un joueur (7 jours)")
        .add_option(
            CreateCommandOption::new(CommandOptionType::User, "cible", "Le joueur a viser")
                .required(true),
        )
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    let Some(guild_id) = require_guild_id(ctx, command).await else { return; };

    let config = load_guild_config(ctx, &guild_id).await;
    if !crate::modules::coude::channel_check::check_channel(ctx, command, config.channel_activites()).await {
        return;
    }

    let target_id = match command
        .data
        .options
        .iter()
        .find(|o| o.name == "cible")
        .and_then(|o| match &o.value {
            CommandDataOptionValue::User(id) => Some(*id),
            _ => None,
        }) {
        Some(id) => id,
        None => {
            reply_ephemeral(ctx, command, "Cible manquante.").await;
            return;
        }
    };

    let challenger_id = command.user.id.to_string();
    let target_id_str = target_id.to_string();

    if challenger_id == target_id_str {
        reply_ephemeral(ctx, command, "Tu ne peux pas te vendetter toi-meme !").await;
        return;
    }

    let target_user = match target_id.to_user(&ctx.http).await {
        Ok(u) => u,
        Err(_) => {
            reply_ephemeral(ctx, command, "Utilisateur introuvable.").await;
            return;
        }
    };
    if target_user.bot {
        reply_ephemeral(ctx, command, "Pas de vendetta contre un bot !").await;
        return;
    }

    let data = ctx.data.read().await;
    let api = data.get::<GameApiKey>().unwrap();

    if let Err(e) = api
        .get_or_create_player(&guild_id, &challenger_id, &command.user.name)
        .await
    {
        reply_api_err(ctx, command, e).await;
        return;
    }
    if let Err(e) = api
        .get_or_create_player(&guild_id, &target_id_str, &target_user.name)
        .await
    {
        reply_api_err(ctx, command, e).await;
        return;
    }

    match api
        .declare_vendetta(&guild_id, &challenger_id, &target_id_str)
        .await
    {
        Ok(_out) => {
            let embed = CreateEmbed::new()
                .title("\u{2694}\u{fe0f} VENDETTA DECLAREE !")
                .description(format!(
                    "<@{}> jure publiquement de se venger de <@{}> !\n\n\
                     Pendant **7 jours** : si <@{}> bat <@{}> en combat, son gain est **double** (+100% bonus). Sinon... humiliation publique en vue.\n\n\
                     _Le branchement mecanique du bonus dans la resolution arrive dans un commit suivant._",
                    command.user.id, target_id, command.user.id, target_id
                ))
                .color(0x8B0000) // rouge sang
                .footer(CreateEmbedFooter::new(
                    crate::shared::branding::COUDE_TAGLINE_SHORT,
                ))
                .timestamp(serenity::model::Timestamp::now());

            crate::modules::coude::channel_check::post_activity(
                ctx,
                command,
                config.channel_activites(),
                embed,
            )
            .await;
        }
        Err(e) => {
            let msg = if e.contains("deja active") {
                "Tu as deja une vendetta active contre cette cible.".to_string()
            } else if e.contains("toi-meme") {
                "Tu ne peux pas te vendetter toi-meme.".to_string()
            } else {
                e.clone()
            };
            reply_ephemeral(ctx, command, &msg).await;
        }
    }
}
