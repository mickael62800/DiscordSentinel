use serenity::all::{CommandInteraction, Context, CreateCommand, CreateEmbed, CreateEmbedFooter};

use crate::shared::discord_helpers::{reply_api_err, reply_embed, reply_ephemeral};

pub fn register() -> CreateCommand {
    CreateCommand::new("repos").description("Repose-toi pour recuperer tous tes HP (cooldown 12h)")
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    let Some((guild_id, _config, api)) = crate::modules::coude::command_prelude::coude_prelude(
        ctx,
        command,
        |c| c.channel_profil(),
        false,
    )
    .await
    else {
        return;
    };

    let user_id = command.user.id.to_string();

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

    // Cooldown effectif resolu server-side (palier "Convalescence" niveau 15+
    // -> plafond 8h). Le bot ne calcule plus la regle.
    let cooldown_hours = match api
        .effective_repos_cooldown_hours(&guild_id, &user_id)
        .await
    {
        Ok(h) => h,
        Err(e) => {
            reply_api_err(ctx, command, e).await;
            return;
        }
    };
    if let Some(ref last_used) = player.repos_last_used {
        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(last_used) {
            let elapsed = chrono::Utc::now().signed_duration_since(dt.with_timezone(&chrono::Utc));
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
        .footer(CreateEmbedFooter::new(
            crate::shared::branding::COUDE_TAGLINE_SHORT,
        ))
        .timestamp(serenity::model::Timestamp::now());

    reply_embed(ctx, command, embed).await;
}
