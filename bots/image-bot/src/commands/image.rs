use serenity::all::{
    CommandInteraction, CommandOptionType, Context, CreateCommand,
    CreateCommandOption,
};

use sentinel_shared::discord_helpers::reply_ephemeral_embed;
use sentinel_shared::embeds::info_embed;

use crate::handler::{HashCacheKey, ProcessedMessagesKey};

pub fn register() -> CreateCommand {
    CreateCommand::new("image")
        .description("Commandes de l'image bot")
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "stats",
                "Affiche les statistiques de l'image bot",
            ),
        )
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    let sub = command
        .data
        .options
        .first()
        .map(|o| o.name.as_str())
        .unwrap_or("");

    if sub == "stats" { handle_stats(ctx, command).await }
}

async fn handle_stats(ctx: &Context, command: &CommandInteraction) {
    let data = ctx.data.read().await;

    let processed_count = data
        .get::<ProcessedMessagesKey>()
        .map(|s| s.len())
        .unwrap_or(0);

    let hash_cache_size = data
        .get::<HashCacheKey>()
        .map(|c| c.len())
        .unwrap_or(0);

    let embed = info_embed("Image Bot — Statistiques")
        .field("Messages traites (cache)", processed_count.to_string(), true)
        .field("Images en cache (hash)", hash_cache_size.to_string(), true);

    reply_ephemeral_embed(ctx, command, embed).await;
}
