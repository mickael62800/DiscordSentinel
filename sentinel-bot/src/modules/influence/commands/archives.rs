//! Commandes `/actu` et `/archives` — la memoire du serveur (Phase 5).

use serenity::all::{
    CommandInteraction, Context, CreateCommand, CreateEmbed, CreateEmbedFooter,
};

use crate::modules::influence::api_client;
use crate::shared::discord_helpers::{reply_ephemeral, reply_ephemeral_embed, require_guild_id};
use crate::shared::heartbeat::ApiClientKey;

pub fn register_actu() -> CreateCommand {
    CreateCommand::new("actu").description("Le fil d'actualite du serveur (derniers evenements)")
}

pub fn register_archives() -> CreateCommand {
    CreateCommand::new("archives").description("La memoire du serveur : grands evenements passes")
}

pub async fn handle_actu(ctx: &Context, command: &CommandInteraction) {
    render(ctx, command, "📰 Fil d'actualité").await;
}

pub async fn handle_archives(ctx: &Context, command: &CommandInteraction) {
    render(ctx, command, "📚 Archives du serveur").await;
}

async fn render(ctx: &Context, command: &CommandInteraction, title: &str) {
    let Some(guild_id) = require_guild_id(ctx, command).await else {
        return;
    };
    let api = {
        let data = ctx.data.read().await;
        match data.get::<ApiClientKey>() {
            Some(a) => a.clone(),
            None => return,
        }
    };

    match api_client::list_archives(&api, &guild_id, None).await {
        Ok(entries) => {
            let body = if entries.is_empty() {
                "*Rien pour l'instant. L'histoire du serveur reste a ecrire.*".to_string()
            } else {
                entries
                    .iter()
                    .map(|e| e.summary.clone())
                    .collect::<Vec<_>>()
                    .join("\n")
            };
            let embed = CreateEmbed::new()
                .title(title)
                .color(0x8E44AD)
                .description(body)
                .footer(CreateEmbedFooter::new(
                    "Les archives racontent l'histoire du serveur.",
                ));
            reply_ephemeral_embed(ctx, command, embed).await;
        }
        Err(e) => reply_ephemeral(ctx, command, &format!("Erreur : {e}")).await,
    }
}
