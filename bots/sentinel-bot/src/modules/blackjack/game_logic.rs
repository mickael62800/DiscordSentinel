//! Commande slash Blackjack (legacy solo) + handler des boutons.
//!
//! Eclaté en sous-modules :
//! - `embeds` : construction de l'embed de partie (+ helpers cartes, `is_game_over`)
//! - `buttons` : boutons d'action (Tirer / Rester / Doubler)
//! - `messages` : phrases fun piochees aleatoirement en fin de partie

pub use super::buttons::build_buttons;
pub use super::embeds::{build_game_message, is_game_over};

use serenity::all::{
    CommandDataOptionValue, CommandInteraction, CommandOptionType, ComponentInteraction, Context,
    CreateCommand, CreateCommandOption, CreateInteractionResponse,
    CreateInteractionResponseMessage,
};

use sentinel_shared::discord_helpers::{
    component_reply_ephemeral as reply_component_ephemeral, reply_ephemeral,
};

use super::GameApiKey;

// ── Slash command registration (legacy solo — conserve pour reference) ──

#[allow(dead_code)]
pub fn register() -> CreateCommand {
    CreateCommand::new("blackjack")
        .description("Joue au Blackjack ! Tente d'atteindre 21 sans depasser.")
        .add_option(
            CreateCommandOption::new(CommandOptionType::Integer, "mise", "Montant a miser")
                .required(true)
                .min_int_value(10),
        )
}

// ── Slash command handler (legacy solo) ──

#[allow(dead_code)]
pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    let guild_id = match command.guild_id {
        Some(id) => id.to_string(),
        None => {
            reply_ephemeral(ctx, command, "Commande serveur uniquement.").await;
            return;
        }
    };

    let user_id = command.user.id.to_string();
    let username = command.user.name.clone();

    let mise = command
        .data
        .options
        .iter()
        .find(|o| o.name == "mise")
        .and_then(|o| match &o.value {
            CommandDataOptionValue::Integer(v) => Some(*v),
            _ => None,
        })
        .unwrap_or(10);

    let data = ctx.data.read().await;
    let api = match data.get::<GameApiKey>() {
        Some(a) => a,
        None => {
            reply_ephemeral(ctx, command, "Erreur interne : API non disponible.").await;
            return;
        }
    };

    // Verifier s'il y a une partie en cours
    match api.get_active(&guild_id, &user_id).await {
        Ok(Some(game)) if game.status == "playing" => {
            // Reprendre la partie en cours
            let (embed, attachment) = build_game_message(&game);
            let components = build_buttons(&game);
            let mut msg = CreateInteractionResponseMessage::new()
                .embed(embed)
                .components(components);
            if let Some(a) = attachment {
                msg = msg.add_file(a);
            }
            command
                .create_response(&ctx.http, CreateInteractionResponse::Message(msg))
                .await
                .ok();
            return;
        }
        _ => {}
    }

    // Nouvelle partie
    let game = match api.start_game(&guild_id, &user_id, &username, mise).await {
        Ok(g) => g,
        Err(e) => {
            reply_ephemeral(ctx, command, &format!("Erreur : {e}")).await;
            return;
        }
    };

    let (embed, attachment) = build_game_message(&game);
    let components = if is_game_over(&game.status) {
        vec![]
    } else {
        build_buttons(&game)
    };

    let mut msg = CreateInteractionResponseMessage::new()
        .embed(embed)
        .components(components);
    if let Some(a) = attachment {
        msg = msg.add_file(a);
    }
    command
        .create_response(&ctx.http, CreateInteractionResponse::Message(msg))
        .await
        .ok();
}

// ── Component (button) handler ──

pub async fn handle_component(ctx: &Context, component: &ComponentInteraction) {
    let custom_id = &component.data.custom_id;

    let game_id = if let Some(id) = custom_id.split(':').nth(1) {
        id.to_string()
    } else {
        reply_component_ephemeral(ctx, component, "ID de partie invalide.").await;
        return;
    };

    let action = if custom_id.starts_with("bj_hit:") {
        "hit"
    } else if custom_id.starts_with("bj_stand:") {
        "stand"
    } else if custom_id.starts_with("bj_double:") {
        "double"
    } else {
        return;
    };

    let guild_id = match component.guild_id {
        Some(g) => g.to_string(),
        None => return,
    };

    let data = ctx.data.read().await;
    let api = match data.get::<GameApiKey>() {
        Some(a) => a,
        None => {
            reply_component_ephemeral(ctx, component, "Erreur interne.").await;
            return;
        }
    };

    // SECURITE : on verifie que le clicker est bien proprietaire de la
    // partie ciblee. `hit/stand/double_down` cote API ne prennent que le
    // game_id, donc sans ce garde-fou n'importe qui voyant les boutons
    // pourrait agir sur la main d'un autre joueur dans un salon partage.
    // On fetch la partie active de l'utilisateur et on compare a game_id.
    let user_id_str = component.user.id.to_string();
    let owns_game = match api.get_active(&guild_id, &user_id_str).await {
        Ok(Some(g)) => g.id == game_id,
        _ => false,
    };
    if !owns_game {
        reply_component_ephemeral(
            ctx,
            component,
            "Ce n'est pas ta partie — tu ne peux pas jouer a la place d'un autre.",
        )
        .await;
        return;
    }

    let result = match action {
        "hit" => api.hit(&game_id).await,
        "stand" => api.stand(&game_id).await,
        "double" => api.double_down(&game_id).await,
        _ => return,
    };

    let game = match result {
        Ok(g) => g,
        Err(e) => {
            reply_component_ephemeral(ctx, component, &format!("Erreur : {e}")).await;
            return;
        }
    };

    let (embed, attachment) = build_game_message(&game);
    let components = if is_game_over(&game.status) {
        vec![]
    } else {
        build_buttons(&game)
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
}

