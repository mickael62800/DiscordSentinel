//! Commande `/prestige` — active un prestige (cf. COUPE_AMELIORATIONS 3.3).
//!
//! Disponible au niveau 25, 5 prestiges max. Reset le niveau a 1 mais
//! ajoute +5% de gains permanents (cumul). Affiche des etoiles a cote
//! du pseudo dans /profil.

use serenity::all::{
    CommandInteraction, Context, CreateCommand, CreateEmbed, CreateEmbedFooter,
};

use crate::shared::discord_helpers::{reply_ephemeral, require_guild_id, reply_api_err};

use crate::modules::coude::load_guild_config;
use crate::modules::coude::GameApiKey;

pub fn register() -> CreateCommand {
    CreateCommand::new("prestige")
        .description("Active un prestige (niveau 25+, reset au niveau 1, +5% gains permanents)")
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    let Some(guild_id) = require_guild_id(ctx, command).await else { return; };
    let config = load_guild_config(ctx, &guild_id).await;
    if !crate::modules::coude::channel_check::check_channel(ctx, command, config.channel_activites()).await {
        return;
    }

    let user_id = command.user.id.to_string();
    let data = ctx.data.read().await;
    let api = data.get::<GameApiKey>().unwrap();

    let player = match api
        .get_or_create_player(&guild_id, &user_id, &command.user.name)
        .await
    {
        Ok(p) => p,
        Err(e) => {
            reply_api_err(ctx, command, e).await;
            return;
        }
    };
    let unlock_level = config.prestige_unlock_level();
    if player.level < unlock_level {
        reply_ephemeral(
            ctx,
            command,
            &format!(
                "Tu dois etre niveau **{}+** pour Prestige (tu es niveau {}).",
                unlock_level, player.level
            ),
        )
        .await;
        return;
    }

    match api.prestige_player(&guild_id, &user_id).await {
        Ok(out) => {
            let embed = CreateEmbed::new()
                .title(format!("\u{2728} PRESTIGE ! {}", out.stars))
                .description(format!(
                    "<@{}> active son **{}eme Prestige** !\n\n\
                     \u{1f504} Reset au niveau 1.\n\
                     \u{1f4c8} **+{}%** de gains permanents (cumul).\n\
                     {} a cote de ton pseudo, eternellement.\n\n\
                     _La legende grandit._",
                    command.user.id,
                    out.new_prestige_count,
                    out.new_prestige_count * 5,
                    out.stars
                ))
                .color(0xFFD700)
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
            let msg = if e.contains("indisponible") {
                "Prestige indisponible : verifie ton niveau (25+) et le cap (5 prestiges max).".to_string()
            } else {
                e.clone()
            };
            reply_ephemeral(ctx, command, &msg).await;
        }
    }
}
