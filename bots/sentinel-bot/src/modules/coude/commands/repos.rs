use serenity::all::{
    CommandInteraction, Context, CreateCommand, CreateEmbed, CreateEmbedFooter,
    CreateInteractionResponse, CreateInteractionResponseMessage,
};

use sentinel_shared::discord_helpers::{reply_ephemeral, require_guild_id, reply_api_err};

use crate::modules::coude::load_guild_config;
use crate::modules::coude::GameApiKey;


pub fn register() -> CreateCommand {
    CreateCommand::new("repos")
        .description("Repose-toi pour recuperer tous tes HP (cooldown 12h)")
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    let Some(guild_id) = require_guild_id(ctx, command).await else { return; };

    let config = load_guild_config(ctx, &guild_id).await;
    if !crate::modules::coude::channel_check::check_channel(ctx, command, config.channel_profil()).await {
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

    // Cooldown effectif : palier "Convalescence" (niveau 15+) reduit le
    // cooldown a 8h max (cf. COUPE_AMELIORATIONS 3.2).
    let base_cooldown_hours = config.repos_cooldown_hours();
    let cooldown_hours = crate::modules::coude::milestones::effective_repos_cooldown_hours(
        base_cooldown_hours,
        player.level,
    );
    if let Some(ref last_used) = player.repos_last_used {
        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(last_used) {
            let elapsed =
                chrono::Utc::now().signed_duration_since(dt.with_timezone(&chrono::Utc));
            let cooldown_mins = cooldown_hours * 60;
            if elapsed.num_minutes() < cooldown_mins {
                let remaining_mins = cooldown_mins - elapsed.num_minutes();
                let h = remaining_mins / 60;
                let m = remaining_mins % 60;
                reply_ephemeral(
                    ctx,
                    command,
                    &format!(
                        "Tu dois encore te reposer **{}h{}m** avant de pouvoir utiliser `/repos` !",
                        h, m
                    ),
                )
                .await;
                return;
            }
        }
    }

    let hp_max = player.hp_max.unwrap_or(100);
    let hp_current = player.hp_current.unwrap_or(hp_max);

    if hp_current >= hp_max {
        reply_ephemeral(ctx, command, "Tu es deja a pleine sante !").await;
        return;
    }

    let healed = hp_max - hp_current;

    // Appeler l'API pour full heal + poser le cooldown repos_last_used
    if let Err(e) = api.repos(&guild_id, &user_id).await {
        reply_api_err(ctx, command, e).await;
        return;
    }

    let embed = CreateEmbed::new()
        .title("\u{1f6cf}\u{fe0f} Repos complet !")
        .description(format!(
            "<@{}> se repose et recupere **+{} HP** !\n\n\
             \u{2764}\u{fe0f} **{}/{}** HP\n\n\
             _Prochain repos disponible dans {} heures._",
            command.user.id, healed, hp_max, hp_max, cooldown_hours
        ))
        .color(0x57F287)
        .footer(CreateEmbedFooter::new(sentinel_shared::branding::COUDE_TAGLINE_SHORT))
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

