//! Commande slash Blackjack (legacy solo) + handler des boutons.
//!
//! Eclaté en sous-modules :
//! - `embeds` : construction de l'embed de partie (+ helpers cartes, `is_game_over`)
//! - `buttons` : boutons d'action (Tirer / Rester / Doubler)
//! - `messages` : phrases fun piochees aleatoirement en fin de partie

pub use super::buttons::build_buttons;
pub use super::embeds::{build_game_message, flavor_key_for_status, is_game_over};

/// Helper async pour pre-fetch le template flavor cote API selon le statut
/// final d'une partie. Retourne `None` si :
///   - la partie n'est pas en etat final (ex: en cours, push)
///   - l'API ne connait pas la cle
///   - l'API est indisponible (best-effort, l'embed retombe sur un texte
///     neutre — pas de fallback local de template).
pub async fn fetch_flavor_for_status(
    api: &super::api_client::ApiClient,
    status: &str,
) -> Option<String> {
    let key = flavor_key_for_status(status)?;
    api.random_flavor(key, "fr").await.ok().flatten()
}

use serenity::all::{
    CommandDataOptionValue, CommandInteraction, CommandOptionType, ComponentInteraction, Context,
    CreateCommand, CreateCommandOption, CreateInteractionResponse,
    CreateInteractionResponseMessage,
};

use crate::shared::discord_helpers::{
    component_reply_ephemeral as reply_component_ephemeral, reply_ephemeral, require_guild_id,
};

use super::GameApiKey;

use super::api_client::{ApiClient, BlackjackGameDto, TauntEvent};

/// Detecte l'etat final d'une partie et declenche les hooks taunts
/// specifiques blackjack (natural 21 / win / bust streak). Les taunts
/// economiques (faillite + jackpot) sont desormais detectes cote API par
/// le wallet UC unifie (migration #4) et retournes directement dans la
/// reponse gRPC — on accepte donc une liste de `wallet_taunts` deja
/// resolus a concatener. Fire-and-forget : un echec reseau ne bloque pas
/// le jeu.
///
/// IMPORTANT : l'ancien appel manuel a `track_jackpot` a ete retire pour
/// eviter un double-taunt avec la detection automatique du wallet UC.
async fn maybe_dispatch_bj_taunts(
    ctx: &Context,
    api: &ApiClient,
    guild_id_raw: &str,
    game: &BlackjackGameDto,
    wallet_taunts: Vec<TauntEvent>,
) {
    let guild_id_num: u64 = match guild_id_raw.parse() {
        Ok(v) => v,
        Err(_) => return,
    };
    let guild_id = serenity::all::GuildId::new(guild_id_num);
    let user_id = &game.user_id;

    let mut events: Vec<TauntEvent> = wallet_taunts;

    match game.status.as_str() {
        "player_blackjack" => {
            if let Ok(Some(ev)) = api.track_bj_natural(guild_id_raw, user_id).await {
                events.push(ev);
            }
        }
        "player_win" | "dealer_bust" => {
            if let Ok(Some(ev)) = api.track_bj_hand_won(guild_id_raw, user_id).await {
                events.push(ev);
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

/// Re-export pour les autres sous-modules du blackjack (ex: `game.rs`).
pub(super) async fn dispatch_blackjack_taunt_pub(
    ctx: &Context,
    guild_id: serenity::all::GuildId,
    ev: TauntEvent,
) {
    dispatch_blackjack_taunt(ctx, guild_id, ev).await;
}

/// Post + rename pour un TauntEvent (mini copie de coude::taunts_dispatch,
/// pour eviter un couplage cross-module). Fire-and-forget.
async fn dispatch_blackjack_taunt(ctx: &Context, guild_id: serenity::all::GuildId, ev: TauntEvent) {
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
                .min_int_value(10)
                // Borne haute : evite une mise absurde (i64::MAX) qui pourrait
                // overflow un calcul serveur (bet*2 sur un double down, etc.).
                .max_int_value(1_000_000_000),
        )
}

// ── Slash command handler (legacy solo) ──

#[allow(dead_code)]
pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    let Some(guild_id) = require_guild_id(ctx, command).await else {
        return;
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
            // Reprendre la partie en cours — fetch le solde live pour
            // l'afficher a cote de l'embed.
            let wallet_balance = api
                .get_wallet(&guild_id, &user_id)
                .await
                .map(|w| w.coins)
                .unwrap_or(0);
            // En cours -> pas de flavor (None, l'embed n'en utilise pas).
            let (embed, attachment) = build_game_message(&game, wallet_balance, None);
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
    let (game, wallet_taunts, wallet_balance) =
        match api.start_game(&guild_id, &user_id, &username, mise).await {
            Ok(g) => g,
            Err(e) => {
                reply_ephemeral(ctx, command, &format!("Erreur : {e}")).await;
                return;
            }
        };

    let flavor = fetch_flavor_for_status(api, &game.status).await;
    let (embed, attachment) = build_game_message(&game, wallet_balance, flavor.as_deref());
    let game_over = is_game_over(&game.status);
    let components = if game_over {
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

    if game_over {
        maybe_dispatch_bj_taunts(ctx, api, &guild_id, &game, wallet_taunts).await;
    } else if !wallet_taunts.is_empty() {
        // Rare mais possible : un taunt wallet (faillite) peut se declencher
        // au debit meme si la partie continue. On dispatche direct.
        let gid = serenity::all::GuildId::new(guild_id.parse().unwrap_or(0));
        for ev in wallet_taunts {
            dispatch_blackjack_taunt(ctx, gid, ev).await;
        }
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

    // Defer UPDATE_MESSAGE : 2 gRPC (get_active + hit/stand/double) avant la
    // mise a jour finale, risque de timeout 3s. Acquittement sans loader.
    // La mise a jour finale utilisera `edit_response` pour editer le message
    // d'origine (embed + boutons). Les erreurs ephemeral deviennent followup.
    if let Err(e) = component
        .create_response(&ctx.http, CreateInteractionResponse::Acknowledge)
        .await
    {
        tracing::warn!(error = %e, "Echec defer blackjack handle_component");
    }

    async fn followup_err(ctx: &Context, component: &ComponentInteraction, msg: String) {
        let _ = component
            .create_followup(
                &ctx.http,
                serenity::all::CreateInteractionResponseFollowup::new()
                    .embed(crate::shared::embeds::feedback_embed(msg))
                    .ephemeral(true),
            )
            .await;
    }

    let data = ctx.data.read().await;
    let api = match data.get::<GameApiKey>() {
        Some(a) => a,
        None => {
            followup_err(ctx, component, "Erreur interne.".to_string()).await;
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
        followup_err(
            ctx,
            component,
            "Ce n'est pas ta partie — tu ne peux pas jouer a la place d'un autre.".to_string(),
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

    let (game, wallet_taunts, wallet_balance) = match result {
        Ok(g) => g,
        Err(e) => {
            followup_err(ctx, component, format!("Erreur : {e}")).await;
            return;
        }
    };

    let flavor = fetch_flavor_for_status(api, &game.status).await;
    let (embed, attachment) = build_game_message(&game, wallet_balance, flavor.as_deref());
    let game_over = is_game_over(&game.status);
    let components = if game_over {
        vec![]
    } else {
        build_buttons(&game)
    };

    let mut edit = serenity::all::EditInteractionResponse::new()
        .embed(embed)
        .components(components);
    if let Some(a) = attachment {
        edit = edit.new_attachment(a);
    }
    if let Err(e) = component.edit_response(&ctx.http, edit).await {
        tracing::warn!(error = %e, "Echec edit_response blackjack");
    }

    // Migration 139 + #4 : si la main se termine, declenche les taunts
    // specifiques blackjack (natural 21, win, bust streak) et concatene les
    // taunts wallet (faillite/jackpot) deja retournes par l'API. Le lock
    // sur `ctx.data` a deja ete relache a la fin du bloc precedent.
    if game_over {
        let api_clone = {
            let data = ctx.data.read().await;
            data.get::<GameApiKey>().cloned()
        };
        if let Some(api) = api_clone {
            maybe_dispatch_bj_taunts(ctx, &api, &guild_id, &game, wallet_taunts).await;
        }
    } else if !wallet_taunts.is_empty() {
        // Faillite potentielle sur le debit d'un double down sans game_over.
        let gid = serenity::all::GuildId::new(guild_id.parse().unwrap_or(0));
        for ev in wallet_taunts {
            dispatch_blackjack_taunt(ctx, gid, ev).await;
        }
    }
}
