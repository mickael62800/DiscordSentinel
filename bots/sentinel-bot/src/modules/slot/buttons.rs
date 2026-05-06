//! Handlers des boutons slot.
//!
//! - `handle_open_machine` (panel global) : cree ou retrouve le salon perso
//!   du user et y poste un message d accueil avec les boutons d action.
//! - `handle_spin_in_channel` / `handle_daily_in_channel` : dans le salon
//!   perso, lance un spin (avec animation suspense 6s) et re-poste un nouveau
//!   message avec les boutons d action.
//! - `handle_close_channel` : ferme et supprime le salon perso.

use std::sync::Arc;
use std::time::Duration;

use serenity::all::{
    ButtonStyle, ChannelType, ComponentInteraction, Context, CreateActionRow, CreateButton,
    CreateChannel, CreateInteractionResponse, CreateInteractionResponseMessage, CreateMessage,
    PermissionOverwrite, PermissionOverwriteType,
};
use serenity::builder::EditMessage;
use serenity::model::id::{ChannelId, RoleId, UserId};
use serenity::model::permissions::Permissions;
use tracing::{error, info, warn};

use sentinel_shared::api_client::BaseApiClient;
use sentinel_shared::heartbeat::ApiClientKey;

use super::animation::{frame_symbols, FRAME_DELAY_MS, TOTAL_REVEAL_FRAMES};
use super::api_client::{self, DailyRequest, SpinRequest, SpinResponse};
use super::embeds;
use super::setup;
use super::SlotChannelManagerKey;

const FALLBACK_DEFAULT_BET: i64 = 50;

// ══════════════════════════════════════════════════════════
// 1. Click sur le panel global "Ouvrir ma machine"
// ══════════════════════════════════════════════════════════

pub async fn handle_open_machine(ctx: &Context, component: &ComponentInteraction) {
    let guild_id = match component.guild_id {
        Some(g) => g,
        None => return,
    };
    let user_id = component.user.id;

    // Verifie si l user a deja un salon ouvert (et qu il existe encore Discord-side).
    let existing = {
        let data = ctx.data.read().await;
        data.get::<SlotChannelManagerKey>()
            .and_then(|mgr| mgr.get(user_id))
    };
    if let Some(active) = existing {
        let still_exists = active.channel_id.to_channel(&ctx.http).await.is_ok();
        if still_exists {
            // Redirige vers le salon existant.
            let resp = CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content(format!(
                        "Tu as deja un salon ouvert : <#{}>",
                        active.channel_id
                    ))
                    .ephemeral(true),
            );
            let _ = component.create_response(&ctx.http, resp).await;
            return;
        }
        // Salon orphelin : purge.
        let data = ctx.data.read().await;
        if let Some(mgr) = data.get::<SlotChannelManagerKey>() {
            mgr.remove(user_id);
        }
    }

    // Reponse immediate ephemerale.
    let resp = CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::new()
            .content("\u{1f3b0} Creation de ton salon prive...")
            .ephemeral(true),
    );
    if let Err(e) = component.create_response(&ctx.http, resp).await {
        warn!(error = %e, "Echec defer panel open");
        return;
    }

    let everyone_role = RoleId::new(guild_id.get());
    let channel_name = format!(
        "slot-{}",
        component.user.name.chars().take(15).collect::<String>().to_lowercase()
    );

    // Categorie ou regrouper les salons slot temp (config slot_category_id).
    let category_id: Option<u64> = {
        let api_arc = {
            let data = ctx.data.read().await;
            data.get::<sentinel_shared::heartbeat::ApiClientKey>()
                .map(std::sync::Arc::clone)
        };
        match api_arc {
            Some(api) => {
                let cfg = api
                    .get_guild_config_for(&guild_id.to_string(), super::MODULE_BOT_NAME)
                    .await
                    .unwrap_or_default();
                cfg.get("slot_category_id").and_then(|v| v.parse::<u64>().ok())
            }
            None => None,
        }
    };

    let mut channel_builder = CreateChannel::new(&channel_name)
        .kind(ChannelType::Text)
        .topic(format!("[slot:{}]", user_id))
        .permissions(vec![
            PermissionOverwrite {
                allow: Permissions::empty(),
                deny: Permissions::VIEW_CHANNEL,
                kind: PermissionOverwriteType::Role(everyone_role),
            },
            PermissionOverwrite {
                allow: Permissions::VIEW_CHANNEL
                    | Permissions::SEND_MESSAGES
                    | Permissions::READ_MESSAGE_HISTORY,
                deny: Permissions::empty(),
                kind: PermissionOverwriteType::Member(user_id),
            },
        ]);
    if let Some(cat) = category_id {
        channel_builder = channel_builder.category(ChannelId::new(cat));
    }

    let channel = match guild_id
        .create_channel(&ctx.http, channel_builder)
        .await
    {
        Ok(ch) => ch,
        Err(e) => {
            error!(error = %e, "Echec creation salon slot");
            let edit = serenity::builder::EditInteractionResponse::new()
                .content("Erreur lors de la creation du salon.");
            let _ = component.edit_response(&ctx.http, edit).await;
            return;
        }
    };

    // Enregistre le salon.
    {
        let data = ctx.data.read().await;
        if let Some(mgr) = data.get::<SlotChannelManagerKey>() {
            mgr.register(user_id, channel.id, guild_id);
        }
    }

    // Message d accueil + boutons d action.
    let welcome = CreateMessage::new()
        .embed(
            serenity::all::CreateEmbed::new()
                .title(format!("\u{1f3b0} Salut <@{}> !", user_id))
                .description("Bienvenue dans ta machine a sous personnelle.\n\nClique sur **Tirer** pour lancer un spin, **Daily Bonus** pour ton spin gratuit du jour, ou **Fermer** pour quitter.")
                .color(0xf1c40f),
        )
        .components(action_buttons_row());

    if let Err(e) = channel.id.send_message(&ctx.http, welcome).await {
        warn!(error = %e, "Echec envoi message accueil slot");
    }

    // Met a jour la reponse ephemerale.
    let edit = serenity::builder::EditInteractionResponse::new()
        .content(format!("Ton salon est pret : <#{}>", channel.id));
    let _ = component.edit_response(&ctx.http, edit).await;

    info!(channel = %channel.id, user = %user_id, "Salon slot cree");
}

// ══════════════════════════════════════════════════════════
// 2. Spin (depuis le salon perso) — avec animation suspense
// ══════════════════════════════════════════════════════════

pub async fn handle_spin_in_channel(ctx: &Context, component: &ComponentInteraction) {
    let guild_id = match component.guild_id {
        Some(g) => g.to_string(),
        None => return,
    };
    let user_id = component.user.id;
    let username = component.user.name.clone();

    // Defer ephemeral pour acquitter le clic ; le vrai resultat arrive via
    // un nouveau message non-ephemeral dans le salon.
    if let Err(e) = component
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Defer(
                CreateInteractionResponseMessage::new().ephemeral(true),
            ),
        )
        .await
    {
        warn!(error = %e, "Echec defer spin in channel");
        return;
    }

    let base = match get_api_client(ctx).await {
        Some(c) => c,
        None => {
            edit_ephemeral_error(ctx, component, "API indisponible").await;
            return;
        }
    };

    let mise = read_default_bet(&base, &guild_id).await;
    let req = SpinRequest {
        user_id: user_id.to_string(),
        username: username.clone(),
        mise,
    };

    let response = match api_client::spin(&base, &guild_id, &req).await {
        Ok(r) => r,
        Err(e) => {
            edit_ephemeral_error(ctx, component, &humanize_api_error(&e)).await;
            return;
        }
    };

    // Touch activity tracker.
    touch_activity(ctx, user_id).await;

    // Lance l animation dans le salon perso.
    play_spin_in_channel(ctx, component.channel_id, &response, &username).await;

    // Acquitte le clic ephemeral.
    let edit = serenity::builder::EditInteractionResponse::new().content("Spin lance !");
    let _ = component.edit_response(&ctx.http, edit).await;
}

// ══════════════════════════════════════════════════════════
// 3. Daily bonus (depuis le salon perso) — avec animation
// ══════════════════════════════════════════════════════════

pub async fn handle_daily_in_channel(ctx: &Context, component: &ComponentInteraction) {
    let guild_id = match component.guild_id {
        Some(g) => g.to_string(),
        None => return,
    };
    let user_id = component.user.id;
    let username = component.user.name.clone();

    if let Err(e) = component
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Defer(
                CreateInteractionResponseMessage::new().ephemeral(true),
            ),
        )
        .await
    {
        warn!(error = %e, "Echec defer daily in channel");
        return;
    }

    let base = match get_api_client(ctx).await {
        Some(c) => c,
        None => {
            edit_ephemeral_error(ctx, component, "API indisponible").await;
            return;
        }
    };

    let req = DailyRequest {
        user_id: user_id.to_string(),
        username: username.clone(),
    };
    let response = match api_client::daily(&base, &guild_id, &req).await {
        Ok(r) => r,
        Err(e) => {
            edit_ephemeral_error(ctx, component, &humanize_api_error(&e)).await;
            return;
        }
    };

    touch_activity(ctx, user_id).await;
    play_spin_in_channel(ctx, component.channel_id, &response, &username).await;

    let edit = serenity::builder::EditInteractionResponse::new().content("Daily lance !");
    let _ = component.edit_response(&ctx.http, edit).await;
}

// ══════════════════════════════════════════════════════════
// 4. Fermer le salon
// ══════════════════════════════════════════════════════════

pub async fn handle_close_channel(ctx: &Context, component: &ComponentInteraction) {
    let user_id = component.user.id;
    let channel_id = component.channel_id;

    // Verifie que le user qui clique est bien l owner du salon.
    let owner_id = {
        let data = ctx.data.read().await;
        data.get::<SlotChannelManagerKey>()
            .and_then(|mgr| mgr.find_by_channel(channel_id))
            .map(|(uid, _)| uid)
    };

    let is_owner = owner_id == Some(user_id);
    let is_admin = component
        .member
        .as_ref()
        .and_then(|m| m.permissions)
        .map(|p| p.contains(Permissions::MANAGE_GUILD) || p.contains(Permissions::ADMINISTRATOR))
        .unwrap_or(false);

    if !is_owner && !is_admin {
        let resp = CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new()
                .content("Seul le proprietaire du salon peut le fermer.")
                .ephemeral(true),
        );
        let _ = component.create_response(&ctx.http, resp).await;
        return;
    }

    // Defer ephemeral.
    if let Err(e) = component
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Defer(
                CreateInteractionResponseMessage::new().ephemeral(true),
            ),
        )
        .await
    {
        warn!(error = %e, "Echec defer close");
        return;
    }

    // Retire du tracker.
    if let Some(uid) = owner_id {
        let data = ctx.data.read().await;
        if let Some(mgr) = data.get::<SlotChannelManagerKey>() {
            mgr.remove(uid);
        }
    }

    // Supprime le salon Discord.
    if let Err(e) = channel_id.delete(&ctx.http).await {
        warn!(error = %e, channel = %channel_id, "Echec suppression salon slot");
    }

    info!(channel = %channel_id, user = %user_id, "Salon slot ferme");
}

// ══════════════════════════════════════════════════════════
// Animation : lance les 4 frames + re-poste les boutons d action
// ══════════════════════════════════════════════════════════

async fn play_spin_in_channel(
    ctx: &Context,
    channel_id: ChannelId,
    response: &SpinResponse,
    username: &str,
) {
    // Sanity : on s attend a 3 symboles. Si l API en renvoie autre chose,
    // on affiche direct le resultat sans animation.
    let final_syms: [String; 3] = match response.symbols.as_slice() {
        [a, b, c] => [a.clone(), b.clone(), c.clone()],
        _ => {
            let _ = channel_id
                .send_message(&ctx.http, build_result_message(response, username))
                .await;
            return;
        }
    };

    // Frame 0 : tout spinning.
    let initial_frame = frame_symbols(&final_syms, 0);
    let mut sent = match channel_id
        .send_message(
            &ctx.http,
            CreateMessage::new().embed(embeds::build_spinning_embed(&initial_frame)),
        )
        .await
    {
        Ok(m) => m,
        Err(e) => {
            warn!(error = %e, "Echec envoi frame 0 slot");
            return;
        }
    };

    // Frames 1..=TOTAL_REVEAL_FRAMES : revele progressif.
    for i in 1..=TOTAL_REVEAL_FRAMES {
        tokio::time::sleep(Duration::from_millis(FRAME_DELAY_MS)).await;

        let edit = if i == TOTAL_REVEAL_FRAMES {
            // Frame finale : remplace par l embed resultat complet.
            EditMessage::new().embed(embeds::build_spin_result_embed(response, username))
        } else {
            let f = frame_symbols(&final_syms, i);
            EditMessage::new().embed(embeds::build_spinning_embed(&f))
        };

        if let Err(e) = sent.edit(&ctx.http, edit).await {
            warn!(error = %e, frame = i, "Echec edit frame slot");
            return;
        }
    }

    // Re-poste les boutons d action en bas.
    let _ = channel_id
        .send_message(
            &ctx.http,
            CreateMessage::new()
                .content("\u{1f3b0} Pret pour le prochain spin ?")
                .components(action_buttons_row()),
        )
        .await;
}

fn build_result_message(response: &SpinResponse, username: &str) -> CreateMessage {
    CreateMessage::new()
        .embed(embeds::build_spin_result_embed(response, username))
        .components(action_buttons_row())
}

/// Row de 3 boutons : Tirer / Daily / Fermer.
pub(super) fn action_buttons_row() -> Vec<CreateActionRow> {
    let spin = CreateButton::new(setup::CHANNEL_SPIN_ID)
        .label("Tirer")
        .emoji(serenity::model::channel::ReactionType::Unicode("\u{1f3b0}".into()))
        .style(ButtonStyle::Success);
    let daily = CreateButton::new(setup::CHANNEL_DAILY_ID)
        .label("Daily Bonus")
        .emoji(serenity::model::channel::ReactionType::Unicode("\u{1f381}".into()))
        .style(ButtonStyle::Primary);
    let close = CreateButton::new(setup::CHANNEL_CLOSE_ID)
        .label("Fermer")
        .emoji(serenity::model::channel::ReactionType::Unicode("\u{274c}".into()))
        .style(ButtonStyle::Danger);
    vec![CreateActionRow::Buttons(vec![spin, daily, close])]
}

// ══════════════════════════════════════════════════════════
// Helpers
// ══════════════════════════════════════════════════════════

async fn get_api_client(ctx: &Context) -> Option<Arc<BaseApiClient>> {
    let data = ctx.data.read().await;
    data.get::<ApiClientKey>().map(Arc::clone)
}

async fn read_default_bet(base: &BaseApiClient, guild_id: &str) -> i64 {
    base.get_guild_config_for(guild_id, super::MODULE_BOT_NAME)
        .await
        .ok()
        .and_then(|cfg| cfg.get("default_bet").and_then(|v| v.parse::<i64>().ok()))
        .unwrap_or(FALLBACK_DEFAULT_BET)
}

async fn touch_activity(ctx: &Context, user_id: UserId) {
    let data = ctx.data.read().await;
    if let Some(mgr) = data.get::<SlotChannelManagerKey>() {
        mgr.touch(user_id);
    }
}

async fn edit_ephemeral_error(ctx: &Context, component: &ComponentInteraction, message: &str) {
    let edit = serenity::builder::EditInteractionResponse::new()
        .embed(embeds::build_error_embed(message));
    if let Err(e) = component.edit_response(&ctx.http, edit).await {
        warn!(error = %e, "Echec edit error response slot");
    }
}

/// Convertit une erreur de l API en message lisible. Tests dans tests/buttons.rs.
pub(crate) fn humanize_api_error(raw: &str) -> String {
    if let Some(start) = raw.find("Cooldown") {
        return raw[start..].split('"').next().unwrap_or(raw).to_string();
    }
    if let Some(start) = raw.find("Mise hors borne") {
        return raw[start..].split('"').next().unwrap_or(raw).to_string();
    }
    if raw.contains("Solde insuffisant") {
        return "Solde insuffisant pour cette mise.".to_string();
    }
    if raw.contains("desactive") {
        return "Daily bonus desactive sur ce serveur.".to_string();
    }
    if raw.contains("deja reclame") {
        return "Tu as deja reclame ton daily bonus aujourd hui.".to_string();
    }
    if raw.contains("Config slot-bot invalide") {
        return "Configuration slot invalide. Contacte un admin.".to_string();
    }
    "Erreur lors du spin. Reessaie dans quelques instants.".to_string()
}

#[cfg(test)]
#[path = "tests/buttons.rs"]
mod tests;

