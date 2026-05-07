//! Commande `/honneur` — invoque la dette d honneur (cf. roadmap 5.3).
//!
//! Si la cible a refuse 3+ fois les combats du caller, ce dernier peut
//! invoquer la dette d honneur pour la traiter publiquement de lache.
//! Le compteur de refus est reset a la suite. Pour rester simple, cette
//! premiere version est purement declarative (annonce publique humiliante)
//! sans creation effective de combat force — la cible reste libre de
//! refuser, mais elle expose son score.

use serenity::all::{
    CommandDataOptionValue, CommandInteraction, CommandOptionType, Context, CreateCommand,
    CreateCommandOption, CreateEmbed, CreateEmbedFooter,
};

use crate::shared::discord_helpers::{reply_ephemeral, require_guild_id, reply_api_err};

use crate::modules::coude::load_guild_config;
use crate::modules::coude::GameApiKey;

const HONOR_DEBT_THRESHOLD: i32 = 3;

pub fn register() -> CreateCommand {
    CreateCommand::new("honneur")
        .description("Invoque la dette d honneur contre un joueur qui te refuse trop")
        .add_option(
            CreateCommandOption::new(CommandOptionType::User, "cible", "Le lache qui te doit un combat")
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
    let requester_id = command.user.id.to_string();
    let target_id_str = target_id.to_string();

    if requester_id == target_id_str {
        reply_ephemeral(ctx, command, "Tu ne peux pas t invoquer toi-meme !").await;
        return;
    }

    let data = ctx.data.read().await;
    let api = data.get::<GameApiKey>().unwrap();

    let resp = match api
        .get_refusal_count(&guild_id, &requester_id, &target_id_str)
        .await
    {
        Ok(r) => r,
        Err(e) => {
            reply_api_err(ctx, command, e).await;
            return;
        }
    };

    if resp.count < HONOR_DEBT_THRESHOLD {
        reply_ephemeral(
            ctx,
            command,
            &format!(
                "<@{}> ne t a refuse que **{}** fois. Il en faut au moins **{}** pour invoquer la dette d honneur.",
                target_id, resp.count, HONOR_DEBT_THRESHOLD
            ),
        )
        .await;
        return;
    }

    // Reset le compteur (best-effort) avant l annonce.
    api.reset_refusal(&guild_id, &requester_id, &target_id_str).await;

    let embed = CreateEmbed::new()
        .title("\u{1f5e1}\u{fe0f} DETTE D HONNEUR")
        .description(format!(
            "<@{}> invoque la **dette d honneur** contre <@{}> ! ({} refus accumules)\n\n\
             Tu es publiquement declare lache notoire. Tout le monde t a vu fuir.\n\n\
             _Le serveur attend ta riposte. /coude est ouvert. Le compteur a ete reset._",
            command.user.id, target_id, resp.count
        ))
        .color(0xC0392B)
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
