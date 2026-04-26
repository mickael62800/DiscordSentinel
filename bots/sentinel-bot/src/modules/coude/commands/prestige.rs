//! Commande `/prestige` — active un prestige (cf. COUPE_AMELIORATIONS 3.3).
//!
//! Disponible au niveau 25, 5 prestiges max. Reset le niveau a 1 mais
//! ajoute +5% de gains permanents (cumul). Affiche des etoiles a cote
//! du pseudo dans /profil.

use serenity::all::{
    CommandInteraction, Context, CreateCommand, CreateEmbed, CreateEmbedFooter,
};

use sentinel_shared::discord_helpers::reply_ephemeral;

use crate::modules::coude::load_guild_config;
use crate::modules::coude::GameApiKey;

const PRESTIGE_UNLOCK_LEVEL: i32 = 25;

pub fn register() -> CreateCommand {
    CreateCommand::new("prestige")
        .description("Active un prestige (niveau 25+, reset au niveau 1, +5% gains permanents)")
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
            reply_ephemeral(ctx, command, &format!("Erreur API : {e}")).await;
            return;
        }
    };
    if player.level < PRESTIGE_UNLOCK_LEVEL {
        reply_ephemeral(
            ctx,
            command,
            &format!(
                "Tu dois etre niveau **{}+** pour Prestige (tu es niveau {}).",
                PRESTIGE_UNLOCK_LEVEL, player.level
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
                    sentinel_shared::branding::COUDE_TAGLINE_SHORT,
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
                format!("Erreur API : {e}")
            };
            reply_ephemeral(ctx, command, &msg).await;
        }
    }
}
