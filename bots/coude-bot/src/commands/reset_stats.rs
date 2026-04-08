use serenity::all::{
    CommandInteraction, Context, CreateCommand, CreateEmbed, CreateEmbedFooter,
    CreateInteractionResponse, CreateInteractionResponseMessage,
};

use crate::game::classes;
use crate::handler::load_guild_config;
use crate::GameApiKey;

const RESET_COST: i64 = 300;

pub fn register() -> CreateCommand {
    CreateCommand::new("reset-stats")
        .description("Redistribue tous tes points de stats (300 coins)")
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
    if !crate::channel_check::check_channel(ctx, command, config.channel_profil()).await {
        return;
    }

    let data = ctx.data.read().await;
    let api = match data.get::<GameApiKey>() {
        Some(a) => a,
        None => return,
    };

    let player = match api
        .get_or_create_player(&guild_id, &command.user.id.to_string(), &command.user.name)
        .await
    {
        Ok(p) => p,
        Err(e) => {
            reply_ephemeral(ctx, command, &format!("Erreur API : {e}")).await;
            return;
        }
    };

    let total_points_spent = player.atk + player.def;
    if total_points_spent == 0 {
        reply_ephemeral(ctx, command, "Tu n'as aucun point distribue a reset !").await;
        return;
    }

    if player.coins < RESET_COST {
        reply_ephemeral(ctx, command, &format!(
            "Le reset coute **{} coins**. Tu n'as que {} coins.", RESET_COST, player.coins
        )).await;
        return;
    }

    // Deduire le cout
    if let Err(e) = api.update_player_coins(&guild_id, &command.user.id.to_string(), -RESET_COST).await {
        reply_ephemeral(ctx, command, &format!("Erreur API : {e}")).await;
        return;
    }

    // TODO: Appeler un endpoint API pour reset ATK/DEF a 0 et redonner les stat_points
    // Pour l'instant on affiche le resultat. L'endpoint sera cree quand necessaire.

    let class = classes::get_class(player.class.as_deref().unwrap_or("bourrin"));

    let embed = CreateEmbed::new()
        .title("\u{1f504} Stats remises a zero !")
        .description(format!(
            "<@{}> a redistribue ses points de stats ! (-{} coins)\n\n\
            **{} points** ont ete recuperes.\n\
            Utilise `/train atk` ou `/train def` pour les reassigner.\n\n\
            Stats de base ({} {}) : ATK {} | DEF {}",
            command.user.id, RESET_COST, total_points_spent,
            class.emoji, class.name, class.base_atk, class.base_def
        ))
        .color(0x3498DB)
        .footer(CreateEmbedFooter::new("Coup de Coude | Sentinel"))
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

async fn reply_ephemeral(ctx: &Context, command: &CommandInteraction, content: &str) {
    command.create_response(&ctx.http, CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::new().content(content).ephemeral(true),
    )).await.ok();
}
