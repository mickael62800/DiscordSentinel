//! Commandes Information (Phase 4) : `/enquete`, `/dossier`, `/reveler`.

use serenity::all::{
    CommandInteraction, CommandOptionType, Context, CreateCommand, CreateCommandOption,
    CreateEmbed, CreateEmbedFooter, CreateInteractionResponse, CreateInteractionResponseMessage,
};

use crate::modules::influence::api_client;
use crate::shared::discord_helpers::{
    option_str, option_user, reply_ephemeral, reply_ephemeral_embed, require_guild_id,
};
use crate::shared::heartbeat::ApiClientKey;

// ── /enquete ──

pub fn register_enquete() -> CreateCommand {
    CreateCommand::new("enquete")
        .description("Lance une enquete sur un citoyen (payant, resultat differe)")
        .add_option(
            CreateCommandOption::new(CommandOptionType::User, "cible", "Citoyen a enqueter")
                .required(true),
        )
        .add_option(
            CreateCommandOption::new(CommandOptionType::String, "sujet", "Ce que tu cherches")
                .required(true),
        )
}

pub async fn handle_enquete(ctx: &Context, command: &CommandInteraction) {
    let Some(guild_id) = require_guild_id(ctx, command).await else {
        return;
    };
    let Some(target) = option_user(&command.data.options, "cible") else {
        return;
    };
    let target_name = command
        .data
        .resolved
        .users
        .get(&target)
        .map(|u| u.name.clone())
        .unwrap_or_default();
    let subject = option_str(&command.data.options, "sujet").unwrap_or("");

    let api = match api(ctx).await {
        Some(a) => a,
        None => return,
    };
    match api_client::open_investigation(
        &api,
        &guild_id,
        &command.user.id.to_string(),
        &command.user.name,
        &target.to_string(),
        &target_name,
        subject,
    )
    .await
    {
        Ok(inv) => {
            reply_ephemeral(
                ctx,
                command,
                &format!(
                    "🔎 Enquete lancee sur **{}** : « {} ». Tu recevras le resultat par message prive une fois l'enquete terminee.",
                    inv.target_username, inv.subject
                ),
            )
            .await
        }
        Err(e) => reply_ephemeral(ctx, command, &format!("Enquete impossible : {e}")).await,
    }
}

// ── /dossier ──

pub fn register_dossier() -> CreateCommand {
    CreateCommand::new("dossier").description("Affiche tes informations secretes (intel)")
}

pub async fn handle_dossier(ctx: &Context, command: &CommandInteraction) {
    let Some(guild_id) = require_guild_id(ctx, command).await else {
        return;
    };
    let api = match api(ctx).await {
        Some(a) => a,
        None => return,
    };
    match api_client::list_intel(&api, &guild_id, &command.user.id.to_string(), &command.user.name)
        .await
    {
        Ok(list) => {
            let body = if list.is_empty() {
                "*Aucune information. Lance une `/enquete`.*".to_string()
            } else {
                list.iter()
                    .map(|i| format!("**{}**\n{}\n`/reveler info:{}`", i.target_username, i.content, i.id))
                    .collect::<Vec<_>>()
                    .join("\n\n")
            };
            let embed = CreateEmbed::new()
                .title("🕵️ Ton dossier secret")
                .color(0x34495E)
                .description(body)
                .footer(CreateEmbedFooter::new(
                    "Revele une info au bon moment pour declencher un scandale.",
                ));
            reply_ephemeral_embed(ctx, command, embed).await;
        }
        Err(e) => reply_ephemeral(ctx, command, &format!("Erreur : {e}")).await,
    }
}

// ── /reveler ──

pub fn register_reveler() -> CreateCommand {
    CreateCommand::new("reveler")
        .description("Revele une information : declenche un scandale public")
        .add_option(
            CreateCommandOption::new(CommandOptionType::String, "info", "Identifiant de l'info (voir /dossier)")
                .required(true),
        )
}

pub async fn handle_reveler(ctx: &Context, command: &CommandInteraction) {
    let Some(guild_id) = require_guild_id(ctx, command).await else {
        return;
    };
    let info_id = option_str(&command.data.options, "info").unwrap_or("");
    let api = match api(ctx).await {
        Some(a) => a,
        None => return,
    };
    match api_client::reveal(&api, &guild_id, &command.user.id.to_string(), &command.user.name, info_id)
        .await
    {
        Ok(o) => {
            let mut embed = CreateEmbed::new()
                .title("💥 SCANDALE")
                .color(0xE74C3C)
                .description(&o.content);
            if o.reputation_loss > 0 {
                embed = embed.field(
                    "Conséquence",
                    format!("**{}** perd **{}** de réputation.", o.target_username, o.reputation_loss),
                    false,
                );
            }
            embed = embed.footer(CreateEmbedFooter::new(format!(
                "Révélé par {}",
                command.user.name
            )));
            // Message PUBLIC (un scandale se sait).
            if let Err(e) = command
                .create_response(
                    &ctx.http,
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new().embed(embed),
                    ),
                )
                .await
            {
                tracing::warn!(error = %e, "Echec annonce scandale");
            }
            // Une du journal : le scandale fait la presse.
            let headline = if o.target_username.is_empty() {
                "💥 SCANDALE !".to_string()
            } else {
                format!("💥 SCANDALE — {} éclaboussé !", o.target_username)
            };
            crate::modules::influence::press::publish_news(ctx, &guild_id, &headline, &o.content).await;
        }
        Err(e) => reply_ephemeral(ctx, command, &format!("Revelation impossible : {e}")).await,
    }
}

async fn api(ctx: &Context) -> Option<std::sync::Arc<crate::shared::api_client::BaseApiClient>> {
    ctx.data.read().await.get::<ApiClientKey>().cloned()
}
