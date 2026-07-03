use serenity::all::{
    ButtonStyle, CommandInteraction, CommandOptionType, ComponentInteraction, Context,
    CreateActionRow, CreateButton, CreateCommand, CreateCommandOption, CreateInteractionResponse,
    CreateInteractionResponseMessage,
};
use tracing::{info, warn};

use super::ModerationApiKey;

pub const UNWARN_PREFIX: &str = "mod_unwarn:";

pub fn register() -> CreateCommand {
    CreateCommand::new("unwarn")
        .description("Retirer un avertissement d'un utilisateur")
        .default_member_permissions(serenity::all::Permissions::MODERATE_MEMBERS)
        .add_option(CreateCommandOption::new(
            CommandOptionType::User,
            "user",
            "L'utilisateur concerne (ou utilise user_id)",
        ))
        .add_option(CreateCommandOption::new(
            CommandOptionType::String,
            "user_id",
            "ID de l'utilisateur (ex. membre parti / banni)",
        ))
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::Boolean,
                "all",
                "Supprimer TOUS les warns de l'utilisateur (au lieu de choisir)",
            )
            .required(false),
        )
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    if !super::has_mod_permission(command, serenity::all::Permissions::MODERATE_MEMBERS) {
        let _ = command
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content("❌ Permission MODERATE_MEMBERS requise pour /unwarn.")
                        .ephemeral(true),
                ),
            )
            .await;
        warn!(user = %command.user.name, "Tentative /unwarn sans permission");
        return;
    }

    let _ = command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Defer(
                CreateInteractionResponseMessage::new().ephemeral(true),
            ),
        )
        .await;

    let target_id = match super::resolve_target_user_id(command, "user") {
        Some(id) => id,
        None => {
            edit_response(
                ctx,
                command,
                "Indique un membre (`user`) ou un identifiant (`user_id`).",
            )
            .await;
            return;
        }
    };

    let guild_id = match command.guild_id {
        Some(id) => id.to_string(),
        None => {
            edit_response(ctx, command, "Commande serveur uniquement.").await;
            return;
        }
    };

    let unwarn_all =
        crate::shared::discord_helpers::option_bool(&command.data.options, "all").unwrap_or(false);

    let api = match ctx.data.read().await.get::<ModerationApiKey>().cloned() {
        Some(a) => a,
        None => {
            edit_response(ctx, command, "Erreur interne.").await;
            return;
        }
    };

    let history = match api.get_history(&guild_id, &target_id.to_string()).await {
        Ok(h) => h,
        Err(e) => {
            edit_response(ctx, command, &e).await;
            return;
        }
    };

    let warns: Vec<_> = history
        .actions
        .iter()
        .filter(|a| a.action_type == "warn")
        .collect();

    if warns.is_empty() {
        edit_response(
            ctx,
            command,
            &format!("<@{}> n'a aucun avertissement actif.", target_id),
        )
        .await;
        return;
    }

    if unwarn_all {
        let total = warns.len();
        let mut success = 0u32;
        let mut failed = 0u32;

        for w in &warns {
            // `delete_action` retire AUSSI le strike lie a ce warn (via
            // infraction_id = action.id) cote API. On ne fait donc PAS d'appel
            // `reset_strikes` global : celui-ci effacerait aussi les strikes
            // d'origine automod (source != "moderator"), ce qui sur-supprimerait
            // et fausserait l'escalation. En supprimant warn par warn, seuls les
            // strikes manuels lies sont retires, les strikes automod restent.
            match api.delete_action(&w.id).await {
                Ok(true) => success += 1,
                Ok(false) => failed += 1,
                Err(e) => {
                    warn!(error = %e, action_id = %w.id, "Echec suppression warn (all)");
                    failed += 1;
                }
            }
        }

        info!(
            moderator = %command.user.name,
            target = %target_id,
            success, failed, total,
            "/unwarn all execute (strikes manuels lies retires, automod preserves)"
        );

        let summary_embed = serenity::builder::CreateEmbed::new()
            .title("🗑️ Suppression massive d'avertissements")
            .description(format!(
                "**{success}/{total}** avertissements supprimes pour <@{target_id}>\n\n\
                 Moderateur : **{}**",
                command.user.name
            ))
            .field("Reussi", success.to_string(), true)
            .field("Echec", failed.to_string(), true)
            .color(if failed == 0 { 0x2ecc71 } else { 0xf59e0b })
            .timestamp(serenity::model::Timestamp::now())
            .footer(serenity::builder::CreateEmbedFooter::new(
                "Moderation | Sentinel",
            ));

        let _ = command
            .edit_response(
                &ctx.http,
                serenity::builder::EditInteractionResponse::new().embed(summary_embed.clone()),
            )
            .await;

        super::log_to_channel(ctx, &guild_id, summary_embed).await;

        // DM d'information a la personne : tous ses avertissements ont ete retires.
        if let Some(gid) = command.guild_id {
            notify_unwarn_dm(ctx, gid, target_id).await;
        }
        return;
    }

    let mut description = format!(
        "**{} avertissement(s) pour <@{}>**\n\n",
        warns.len(),
        target_id
    );

    let mut buttons = Vec::new();

    for (i, w) in warns.iter().enumerate() {
        let date = &w.created_at[..10.min(w.created_at.len())];
        let gravity = w.gravity.as_deref().unwrap_or("?");
        let reason_short: String = w.reason.chars().take(50).collect();

        description.push_str(&format!(
            "**#{}** — {} | Gravite: **{}** | {}\n> Raison : {}\n\n",
            i + 1,
            date,
            gravity,
            w.moderator_name,
            reason_short
        ));

        if buttons.len() < 25 {
            buttons.push(
                CreateButton::new(format!("{}{}", UNWARN_PREFIX, w.id))
                    .label(format!("❌ #{}", i + 1))
                    .style(ButtonStyle::Danger),
            );
        }
    }

    let rows: Vec<CreateActionRow> = buttons
        .chunks(5)
        .map(|chunk| CreateActionRow::Buttons(chunk.to_vec()))
        .collect();

    let embed = serenity::builder::CreateEmbed::new()
        .title("🗑️ Retirer un avertissement")
        .description(&description)
        .color(0xf59e0b)
        .footer(serenity::builder::CreateEmbedFooter::new(
            "Cliquez sur un bouton pour retirer l'avertissement correspondant",
        ));

    let _ = command
        .edit_response(
            &ctx.http,
            serenity::builder::EditInteractionResponse::new()
                .embed(embed)
                .components(rows),
        )
        .await;
}

/// Handler du bouton pour supprimer un warn specifique.
pub async fn handle_button(ctx: &Context, component: &ComponentInteraction) {
    let action_id = match component.data.custom_id.strip_prefix(UNWARN_PREFIX) {
        Some(id) => id.to_string(),
        None => return,
    };

    let has_permission = component
        .member
        .as_ref()
        .and_then(|m| m.permissions)
        .map(|p| {
            p.contains(serenity::all::Permissions::MODERATE_MEMBERS)
                || p.contains(serenity::all::Permissions::ADMINISTRATOR)
        })
        .unwrap_or(false);

    if !has_permission {
        let _ = component
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content("❌ Seul un moderateur peut retirer un avertissement.")
                        .ephemeral(true),
                ),
            )
            .await;
        return;
    }

    let api = {
        let data = ctx.data.read().await;
        match data.get::<ModerationApiKey>() {
            Some(a) => a.clone(),
            None => return,
        }
    };

    match api.delete_action(&action_id).await {
        Ok(true) => {
            info!(action_id = %action_id, moderator = %component.user.name, "Warn supprime via /unwarn");

            let embed = serenity::builder::CreateEmbed::new()
                .title("✅ Avertissement retire")
                .description(format!(
                    "Warn `{}` supprime par **{}**.",
                    &action_id[..8.min(action_id.len())],
                    component.user.name
                ))
                .color(0x2ecc71);

            let _ = component
                .create_response(
                    &ctx.http,
                    CreateInteractionResponse::UpdateMessage(
                        CreateInteractionResponseMessage::new()
                            .embed(embed)
                            .components(vec![]),
                    ),
                )
                .await;

            if let Some(guild_id) = component.guild_id {
                let log_embed = serenity::builder::CreateEmbed::new()
                    .title("🗑️ Unwarn")
                    .description(format!(
                        "Warn `{}` retire par <@{}>",
                        &action_id[..8.min(action_id.len())],
                        component.user.id
                    ))
                    .color(0x2ecc71)
                    .timestamp(serenity::model::Timestamp::now())
                    .footer(serenity::builder::CreateEmbedFooter::new(
                        "Moderation | Sentinel",
                    ));
                super::log_to_channel(ctx, &guild_id.to_string(), log_embed).await;

                // DM d'information a la personne : un avertissement a ete retire.
                if let Some(uid) = component
                    .message
                    .embeds
                    .first()
                    .and_then(|e| e.description.as_deref())
                    .and_then(first_user_mention)
                {
                    notify_unwarn_dm(ctx, guild_id, uid).await;
                }
            }
        }
        Ok(false) => {
            let _ = component
                .create_response(
                    &ctx.http,
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new()
                            .content("⚠️ Avertissement introuvable (deja supprime ?).")
                            .ephemeral(true),
                    ),
                )
                .await;
        }
        Err(e) => {
            warn!(error = %e, "Echec suppression warn");
            let _ = component
                .create_response(
                    &ctx.http,
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new()
                            .content(format!("Erreur : {e}"))
                            .ephemeral(true),
                    ),
                )
                .await;
        }
    }
}

async fn edit_response(ctx: &Context, command: &CommandInteraction, content: &str) {
    let _ = command
        .edit_response(
            &ctx.http,
            serenity::builder::EditInteractionResponse::new().content(content),
        )
        .await;
}

/// Extrait le 1er user mentionne (`<@id>` ou `<@!id>`) d'un texte.
fn first_user_mention(text: &str) -> Option<serenity::model::id::UserId> {
    let start = text.find("<@")?;
    let rest = &text[start + 2..];
    let end = rest.find('>')?;
    let digits: String = rest[..end].chars().filter(|c| c.is_ascii_digit()).collect();
    digits
        .parse::<u64>()
        .ok()
        .map(serenity::model::id::UserId::new)
}

/// Envoie un DM a la personne pour l'informer qu'un de ses avertissements a
/// ete retire.
async fn notify_unwarn_dm(
    ctx: &Context,
    guild_id: serenity::model::id::GuildId,
    user_id: serenity::model::id::UserId,
) {
    let guild_name = guild_id
        .to_partial_guild(&ctx.http)
        .await
        .map(|g| g.name)
        .unwrap_or_else(|_| "le serveur".into());
    if let Ok(user) = user_id.to_user(&ctx.http).await {
        if let Ok(dm) = user.create_dm_channel(&ctx.http).await {
            let embed = serenity::builder::CreateEmbed::new()
                .title(format!("✅ Un avertissement retire sur **{guild_name}**"))
                .description("Un de tes avertissements a ete retire par la moderation.")
                .color(0x2ecc71)
                .timestamp(serenity::model::Timestamp::now());
            if let Err(e) = dm
                .send_message(
                    &ctx.http,
                    serenity::builder::CreateMessage::new().embed(embed),
                )
                .await
            {
                warn!(error = %e, "Failed to send unwarn DM to user");
            }
        }
    }
}
