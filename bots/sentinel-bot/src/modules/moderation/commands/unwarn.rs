use serenity::all::{
    ButtonStyle, CommandDataOptionValue, CommandInteraction, CommandOptionType,
    ComponentInteraction, Context, CreateActionRow, CreateButton, CreateCommand,
    CreateCommandOption, CreateInteractionResponse, CreateInteractionResponseMessage,
};
use tracing::{info, warn};

use super::super::ModerationApiKey;

pub const UNWARN_PREFIX: &str = "mod_unwarn:";

pub fn register() -> CreateCommand {
    CreateCommand::new("unwarn")
        .description("Retirer un avertissement d'un utilisateur")
        .default_member_permissions(serenity::all::Permissions::MODERATE_MEMBERS)
        .add_option(
            CreateCommandOption::new(CommandOptionType::User, "user", "L'utilisateur concerne")
                .required(true),
        )
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
        let _ = command.create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content("❌ Permission MODERATE_MEMBERS requise pour /unwarn.")
                    .ephemeral(true),
            ),
        ).await;
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

    let target_id = match command
        .data
        .options
        .iter()
        .find(|o| o.name == "user")
        .and_then(|o| match &o.value {
            CommandDataOptionValue::User(id) => Some(*id),
            _ => None,
        }) {
        Some(id) => id,
        None => {
            edit_response(ctx, command, "Parametre 'user' manquant.").await;
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

    let unwarn_all = command
        .data
        .options
        .iter()
        .find(|o| o.name == "all")
        .and_then(|o| match &o.value {
            CommandDataOptionValue::Boolean(b) => Some(*b),
            _ => None,
        })
        .unwrap_or(false);

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
            edit_response(ctx, command, &format!("Erreur API : {e}")).await;
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
            match api.delete_action(&w.id).await {
                Ok(true) => success += 1,
                Ok(false) => failed += 1,
                Err(e) => {
                    warn!(error = %e, action_id = %w.id, "Echec suppression warn (all)");
                    failed += 1;
                }
            }
        }

        if let Err(e) = api.reset_strikes(&guild_id, &target_id.to_string()).await {
            warn!(error = %e, "Echec reset_strikes apres unwarn all");
        }

        info!(
            moderator = %command.user.name,
            target = %target_id,
            success, failed, total,
            "/unwarn all execute + strikes reset"
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
        return;
    }

    let mut description = format!(
        "**{} avertissement(s) pour <@{}>**\n\n",
        warns.len(),
        target_id
    );

    let mut buttons = Vec::new();

    for (i, w) in warns.iter().enumerate() {
        let date = &w.created_at[..10];
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
                    &action_id[..8],
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
                    .footer(serenity::builder::CreateEmbedFooter::new("Moderation | Sentinel"));
                super::log_to_channel(ctx, &guild_id.to_string(), log_embed).await;
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
