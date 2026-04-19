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

use super::api_client::{ApiClient, BlackjackGameDto, TauntEvent};

/// Detecte l'etat final d'une partie et declenche les hooks taunts
/// appropries. Fire-and-forget : un echec reseau ne bloque pas le jeu.
async fn maybe_dispatch_bj_taunts(
    ctx: &Context,
    api: &ApiClient,
    guild_id_raw: &str,
    game: &BlackjackGameDto,
) {
    let guild_id_num: u64 = match guild_id_raw.parse() {
        Ok(v) => v,
        Err(_) => return,
    };
    let guild_id = serenity::all::GuildId::new(guild_id_num);
    let user_id = &game.user_id;

    let mut events: Vec<TauntEvent> = Vec::new();

    match game.status.as_str() {
        "player_blackjack" => {
            if let Ok(Some(ev)) = api.track_bj_natural(guild_id_raw, user_id).await {
                events.push(ev);
            }
            if game.payout > 0 {
                if let Ok(Some(ev)) = api.track_jackpot(guild_id_raw, user_id, game.payout).await {
                    events.push(ev);
                }
            }
        }
        "player_win" | "dealer_bust" => {
            if let Ok(Some(ev)) = api.track_bj_hand_won(guild_id_raw, user_id).await {
                events.push(ev);
            }
            if game.payout > 0 {
                if let Ok(Some(ev)) = api.track_jackpot(guild_id_raw, user_id, game.payout).await {
                    events.push(ev);
                }
            }
        }
        "player_bust" => {
            if let Ok(Some(ev)) = api.track_bj_hand_bust(guild_id_raw, user_id).await {
                events.push(ev);
            }
        }
        _ => {}
    }

    for ev in events {
        dispatch_blackjack_taunt(ctx, guild_id, ev).await;
    }
}

/// Post + rename pour un TauntEvent (mini copie de coude::taunts_dispatch,
/// pour eviter un couplage cross-module). Fire-and-forget.
async fn dispatch_blackjack_taunt(
    ctx: &Context,
    guild_id: serenity::all::GuildId,
    ev: TauntEvent,
) {
    use serenity::all::{
        ChannelId, CreateEmbed, CreateEmbedFooter, CreateMessage, EditMember, UserId,
    };

    if let Ok(channel_id) = ev.channel_id.parse::<u64>() {
        let embed = CreateEmbed::new()
            .title("\u{1f3b0} Raillerie automatique")
            .description(&ev.message)
            .color(0xE67E22)
            .footer(CreateEmbedFooter::new(format!(
                "Serie : {} × {}",
                ev.streak_kind, ev.streak_value
            )));
        let _ = ChannelId::new(channel_id)
            .send_message(&ctx.http, CreateMessage::new().embed(embed))
            .await;
    }

    if let Ok(uid) = ev.target_user_id.parse::<u64>() {
        let user = UserId::new(uid);
        if let Ok(member) = guild_id.member(&ctx.http, user).await {
            let current = member
                .nick
                .clone()
                .unwrap_or_else(|| member.user.name.clone());
            if !current.ends_with(&ev.nickname_suffix) {
                const MAX: usize = 32;
                let suffix_len = ev.nickname_suffix.chars().count();
                let max_base = MAX.saturating_sub(suffix_len);
                let base: String = current.chars().take(max_base).collect();
                let new_nick = format!("{}{}", base, ev.nickname_suffix);
                let _ = guild_id
                    .edit_member(&ctx.http, user, EditMember::new().nickname(&new_nick))
                    .await;
            }
        }
    }
}

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
    let game_over = is_game_over(&game.status);
    let components = if game_over { vec![] } else { build_buttons(&game) };

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

    if game_over {
        maybe_dispatch_bj_taunts(ctx, api, &guild_id, &game).await;
    }
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
    let game_over = is_game_over(&game.status);
    let components = if game_over { vec![] } else { build_buttons(&game) };

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

    // Migration 139 : si la main se termine, declenche les taunts appropries
    // (natural 21, win, bust + eventuel jackpot sur le payout). Le lock
    // sur `ctx.data` a deja ete relache a la fin du bloc precedent.
    if game_over {
        let api_clone = {
            let data = ctx.data.read().await;
            data.get::<GameApiKey>().cloned()
        };
        if let Some(api) = api_clone {
            maybe_dispatch_bj_taunts(ctx, &api, &guild_id, &game).await;
        }
    }
}

