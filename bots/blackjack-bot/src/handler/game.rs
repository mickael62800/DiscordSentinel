//! Gestion de la partie : sélection de mise, détection fin de partie, rejouer.

use serenity::all::{
    ButtonStyle, CreateActionRow, CreateButton, CreateEmbed, CreateInteractionResponse,
    CreateInteractionResponseMessage, CreateMessage,
};
use serenity::model::application::ComponentInteraction;
use serenity::model::id::ChannelId;
use serenity::prelude::*;

use super::{ChannelManagerKey, BET_PREFIX, CLOSE_TABLE_ID};
use crate::commands;

/// Sélection d'une mise → démarre la partie.
pub(super) async fn handle_bet_select(ctx: &Context, component: &ComponentInteraction) {
    let bet: i64 = match component
        .data
        .custom_id
        .strip_prefix(BET_PREFIX)
        .and_then(|b| b.parse().ok())
    {
        Some(b) => b,
        None => return,
    };

    let guild_id = match component.guild_id {
        Some(g) => g.to_string(),
        None => return,
    };

    // Touch
    {
        let data = ctx.data.read().await;
        if let Some(mgr) = data.get::<ChannelManagerKey>() {
            mgr.touch(component.user.id);
        }
    }

    let data = ctx.data.read().await;
    let api = match data.get::<crate::GameApiKey>() {
        Some(a) => a,
        None => return,
    };

    let game = match api
        .start_game(
            &guild_id,
            &component.user.id.to_string(),
            &component.user.name,
            bet,
        )
        .await
    {
        Ok(g) => g,
        Err(e) => {
            let resp = CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content(format!("Erreur : {e}"))
                    .ephemeral(true),
            );
            let _ = component.create_response(&ctx.http, resp).await;
            return;
        }
    };

    // Stocker game_id
    if let Some(mgr) = data.get::<ChannelManagerKey>() {
        mgr.set_game_id(component.user.id, game.id.clone());
    }
    drop(data);

    let (embed, attachment) = commands::blackjack::build_game_message(&game);
    let components = if commands::blackjack::is_game_over(&game.status) {
        vec![]
    } else {
        commands::blackjack::build_buttons(&game)
    };

    let mut msg = CreateInteractionResponseMessage::new()
        .embed(embed)
        .components(components);
    if let Some(a) = attachment {
        msg = msg.add_file(a);
    }
    component
        .create_response(&ctx.http, CreateInteractionResponse::UpdateMessage(msg))
        .await
        .ok();

    // Si blackjack naturel → proposer de rejouer
    if commands::blackjack::is_game_over(&game.status) {
        send_replay_buttons(ctx, component.channel_id).await;
    }
}

/// Vérifie si la partie est terminée après un hit/stand/double.
pub(super) async fn check_game_over(ctx: &Context, component: &ComponentInteraction) {
    let guild_id = match component.guild_id {
        Some(g) => g.to_string(),
        None => return,
    };

    let is_over = {
        let data = ctx.data.read().await;
        let api = match data.get::<crate::GameApiKey>() {
            Some(a) => a,
            None => return,
        };
        match api
            .get_active(&guild_id, &component.user.id.to_string())
            .await
        {
            Ok(Some(game)) => commands::blackjack::is_game_over(&game.status),
            Ok(None) => true, // Pas de partie active = terminee
            Err(_) => false,
        }
    };

    if is_over {
        send_replay_buttons(ctx, component.channel_id).await;
    }
}

/// Envoie les boutons "Rejouer" ou "Fermer la table".
async fn send_replay_buttons(ctx: &Context, channel_id: ChannelId) {
    let embed = CreateEmbed::new()
        .description("\u{1f503} **Encore une partie ?**")
        .color(0x3498db);

    let buttons = vec![
        CreateButton::new(format!("{BET_PREFIX}50"))
            .label("50")
            .style(ButtonStyle::Secondary),
        CreateButton::new(format!("{BET_PREFIX}100"))
            .label("100")
            .style(ButtonStyle::Primary),
        CreateButton::new(format!("{BET_PREFIX}250"))
            .label("250")
            .style(ButtonStyle::Primary),
        CreateButton::new(CLOSE_TABLE_ID)
            .label("Fermer la table")
            .emoji(serenity::model::channel::ReactionType::Unicode(
                "\u{274c}".into(),
            ))
            .style(ButtonStyle::Danger),
    ];
    let row = CreateActionRow::Buttons(buttons);

    let _ = channel_id
        .send_message(
            ctx,
            CreateMessage::new().embed(embed).components(vec![row]),
        )
        .await;
}
