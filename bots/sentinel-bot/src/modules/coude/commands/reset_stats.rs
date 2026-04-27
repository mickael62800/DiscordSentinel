use serenity::all::{
    CommandInteraction, Context, CreateCommand, CreateEmbed, CreateEmbedFooter,
    CreateInteractionResponse, CreateInteractionResponseMessage,
};

use sentinel_shared::discord_helpers::{reply_ephemeral, require_guild_id, reply_api_err};

use crate::modules::coude::catalog::CatalogCacheKey;
use crate::modules::coude::load_guild_config;
use crate::modules::coude::GameApiKey;


pub fn register() -> CreateCommand {
    CreateCommand::new("reset-stats")
        .description("Redistribue tous tes points de stats (300 coins)")
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    let Some(guild_id) = require_guild_id(ctx, command).await else { return; };

    let config = load_guild_config(ctx, &guild_id).await;
    if !crate::modules::coude::channel_check::check_channel(ctx, command, config.channel_profil()).await {
        return;
    }

    let data = ctx.data.read().await;
    let api = match data.get::<GameApiKey>() {
        Some(a) => a,
        None => return,
    };
    let catalog = match data.get::<CatalogCacheKey>() {
        Some(c) => c.clone(),
        None => return,
    };

    let player = match api
        .get_or_create_player(&guild_id, &command.user.id.to_string(), &command.user.name)
        .await
    {
        Ok(p) => p,
        Err(e) => {
            reply_api_err(ctx, command, e).await;
            return;
        }
    };

    let total_points_spent = player.atk + player.def;
    if total_points_spent == 0 {
        reply_ephemeral(ctx, command, "Tu n'as aucun point distribue a reset !").await;
        return;
    }

    if player.coins < config.reset_stats_cost() {
        reply_ephemeral(ctx, command, &format!(
            "Le reset coute **{} coins**. Tu n'as que {} coins.", config.reset_stats_cost(), player.coins
        )).await;
        return;
    }

    // Reset atomique cote API : deduit le cout, remet ATK/DEF a 0,
    // et restitue les points dans stat_points en une seule UPDATE.
    if let Err(e) = api
        .reset_stats(&guild_id, &command.user.id.to_string(), config.reset_stats_cost())
        .await
    {
        reply_api_err(ctx, command, e).await;
        return;
    }

    // Phase 9 : le cout du reset alimente la caisse communautaire.
    if let Err(e) = api
        .deposit_cashbox(
            &guild_id,
            config.reset_stats_cost(),
            crate::modules::coude::api_client::CashboxDepositSource::ResetStatsCost,
        )
        .await
    {
        tracing::warn!(error = %e, guild_id, "Echec deposit cashbox reset_stats");
    }

    let class = catalog.get_class(player.class.as_deref().unwrap_or("bourrin"));

    let embed = CreateEmbed::new()
        .title("\u{1f504} Stats remises a zero !")
        .description(format!(
            "<@{}> a redistribue ses points de stats ! (-{} coins)\n\n\
            **{} points** ont ete recuperes.\n\
            Utilise `/train atk` ou `/train def` pour les reassigner.\n\n\
            Stats de base ({} {}) : ATK {} | DEF {}",
            command.user.id, config.reset_stats_cost(), total_points_spent,
            class.emoji, class.name, class.base_atk, class.base_def
        ))
        .color(0x3498DB)
        .footer(CreateEmbedFooter::new(sentinel_shared::branding::COUDE_TAGLINE_SHORT))
        .timestamp(serenity::model::Timestamp::now());

    command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new().embed(embed),
            ),
        )
        .await
        .ok();
}

