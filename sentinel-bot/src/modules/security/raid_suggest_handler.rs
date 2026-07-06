//! Suggestions anti-raid (mode `suggest` / `hybrid` sous le seuil).
//!
//! Au lieu d'appliquer directement la reponse GUILD-WIDE (lockdown + slowmode),
//! le bot poste une alerte staff avec deux boutons : Confirmer / Ignorer.
//! - Confirmer -> applique lockdown et/ou slowmode via les managers existants.
//! - Ignorer   -> annule la suggestion.
//! L'action est gatee aux membres MANAGE_GUILD / ADMINISTRATOR.

use serenity::all::{ComponentInteraction, Context};
use serenity::builder::{
    CreateActionRow, CreateButton, CreateInteractionResponse, CreateInteractionResponseMessage,
    CreateMessage, EditMessage,
};
use serenity::model::id::{ChannelId, GuildId};
use tracing::{error, warn};

use crate::shared::embeds::danger_embed;

use super::{LockdownKey, RaidSuggestGuardKey, SlowmodeKey};

/// Prefixe du bouton "Confirmer" : `raid_confirm_{guild}_{lockdown}_{slowmode}`.
pub const RAID_CONFIRM_PREFIX: &str = "raid_confirm_";
/// Prefixe du bouton "Ignorer" : `raid_dismiss_{guild}`.
pub const RAID_DISMISS_PREFIX: &str = "raid_dismiss_";

/// Retourne true si ce custom_id est une interaction de suggestion anti-raid.
pub fn handles_component(cid: &str) -> bool {
    cid.starts_with(RAID_CONFIRM_PREFIX) || cid.starts_with(RAID_DISMISS_PREFIX)
}

/// Poste l'alerte staff (embed + boutons) dans le salon donne. Best-effort.
/// La deduplication (garde par serveur) est appliquee par l'appelant.
#[allow(clippy::too_many_arguments)]
pub async fn post_suggestion(
    ctx: &Context,
    channel: ChannelId,
    guild_id: GuildId,
    raid_score: u32,
    signals: &str,
    activate_lockdown: bool,
    slowmode_secs: u32,
) {
    let mut proposed = Vec::new();
    if activate_lockdown {
        proposed.push("lockdown (blocage des messages)".to_string());
    }
    if slowmode_secs > 0 {
        proposed.push(format!("slowmode {slowmode_secs}s"));
    }
    let proposed_str = if proposed.is_empty() {
        "aucune".to_string()
    } else {
        proposed.join(" + ")
    };

    let embed = danger_embed("\u{1f6a8} Raid suspecte — confirmation requise")
        .field("Score de raid", format!("{raid_score}/100"), true)
        .field("Reponse proposee", proposed_str, true)
        .field(
            "Signaux",
            if signals.trim().is_empty() {
                "Analyse anti-raid"
            } else {
                signals
            },
            false,
        )
        .field(
            "Action",
            "Confirmez pour appliquer la reponse, ou ignorez pour ne rien faire. La quarantaine des comptes suspects a deja ete appliquee automatiquement.",
            false,
        );

    let confirm_id = format!(
        "{RAID_CONFIRM_PREFIX}{guild_id}_{}_{slowmode_secs}",
        if activate_lockdown { 1 } else { 0 }
    );
    let dismiss_id = format!("{RAID_DISMISS_PREFIX}{guild_id}");

    let buttons = CreateActionRow::Buttons(vec![
        CreateButton::new(confirm_id)
            .label("Confirmer")
            .style(serenity::all::ButtonStyle::Danger),
        CreateButton::new(dismiss_id)
            .label("Ignorer")
            .style(serenity::all::ButtonStyle::Secondary),
    ]);

    let msg = CreateMessage::new().embed(embed).components(vec![buttons]);
    if let Err(e) = channel.send_message(&ctx.http, msg).await {
        error!(error = %e, guild_id = %guild_id, "Impossible de poster la suggestion anti-raid");
        // Le garde a deja ete acquis par l'appelant ; on le libere pour ne pas
        // bloquer indefiniment les alertes suite a un echec d'envoi.
        if let Some(guard) = ctx.data.read().await.get::<RaidSuggestGuardKey>() {
            guard.release(guild_id);
        }
    }
}

/// Gere les boutons Confirmer / Ignorer d'une suggestion anti-raid.
pub(super) async fn on_component(ctx: &Context, component: &ComponentInteraction) {
    let cid = component.data.custom_id.clone();

    // ── Gating staff (MANAGE_GUILD / ADMINISTRATOR) ──
    let has_perm = component
        .member
        .as_ref()
        .and_then(|m| m.permissions)
        .map(|p| {
            p.contains(serenity::all::Permissions::MANAGE_GUILD)
                || p.contains(serenity::all::Permissions::ADMINISTRATOR)
        })
        .unwrap_or(false);

    if !has_perm {
        let _ = component
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content("Seul un membre staff (Gérer le serveur) peut valider une réponse anti-raid.")
                        .ephemeral(true),
                ),
            )
            .await;
        warn!(
            user = %component.user.name,
            "Tentative de validation anti-raid sans permission"
        );
        return;
    }

    let staff = format!("<@{}>", component.user.id);

    if let Some(rest) = cid.strip_prefix(RAID_DISMISS_PREFIX) {
        let guild_id = parse_guild(rest);
        if let Some(g) = guild_id {
            release_guard(ctx, g).await;
        }
        finalize(ctx, component, &format!("\u{274c} Ignoré par {staff}")).await;
        return;
    }

    if let Some(rest) = cid.strip_prefix(RAID_CONFIRM_PREFIX) {
        // Format : {guild}_{lockdown 0|1}_{slowmode_secs}
        let parts: Vec<&str> = rest.split('_').collect();
        let (guild_id, lockdown, slowmode_secs) = match parts.as_slice() {
            [g, l, s] => (
                g.parse::<u64>().ok().map(GuildId::new),
                *l == "1",
                s.parse::<u32>().unwrap_or(0),
            ),
            _ => (None, false, 0),
        };

        let guild_id = match guild_id {
            Some(g) => g,
            None => {
                warn!(custom_id = %cid, "custom_id de confirmation anti-raid invalide");
                finalize(ctx, component, "\u{26a0}\u{fe0f} Suggestion invalide").await;
                return;
            }
        };

        // F3 : la permission verifiee plus haut porte sur `component.guild_id`.
        // On exige que le serveur cible (encode dans le custom_id) soit CELUI de
        // l'interaction -> sinon un staff du serveur A pourrait declencher un
        // lockdown sur le serveur B (custom_id forge/rejoue).
        if component.guild_id != Some(guild_id) {
            warn!(user = %component.user.name, target = %guild_id, "Confirmation anti-raid cross-serveur refusee");
            finalize(ctx, component, "\u{26a0}\u{fe0f} Action non autorisée sur ce serveur").await;
            return;
        }

        // Applique la reponse via les managers existants (idempotents : le
        // "already active" evite les doublons).
        {
            let data = ctx.data.read().await;
            // Durees de vie persistees : refletent la config security (env),
            // identiques a celles utilisees par les boucles de revert locales.
            let env_config = data.get::<super::SecurityConfigKey>();
            let lockdown_duration = env_config.map(|c| c.lockdown_duration_secs).unwrap_or(300);
            let slowmode_duration = env_config.map(|c| c.slowmode_duration_secs).unwrap_or(300);
            if lockdown {
                if let Some(mgr) = data.get::<LockdownKey>() {
                    if let Ok(mut guild) = guild_id.to_partial_guild(&ctx.http).await {
                        let edit = serenity::builder::EditGuild::new()
                            .verification_level(serenity::model::guild::VerificationLevel::Higher);
                        if let Err(e) = guild.edit(&ctx.http, edit).await {
                            error!(error = %e, "Impossible d'activer le lockdown (confirmation)");
                        }
                    }
                    mgr.activate(ctx, guild_id, lockdown_duration).await;
                }
            }
            if slowmode_secs > 0 {
                if let Some(mgr) = data.get::<SlowmodeKey>() {
                    mgr.activate(ctx, guild_id, slowmode_secs as u16, slowmode_duration)
                        .await;
                }
            }
        }

        release_guard(ctx, guild_id).await;
        finalize(ctx, component, &format!("\u{2705} Confirmé par {staff}")).await;
    }
}

fn parse_guild(s: &str) -> Option<GuildId> {
    s.parse::<u64>().ok().map(GuildId::new)
}

async fn release_guard(ctx: &Context, guild_id: GuildId) {
    if let Some(guard) = ctx.data.read().await.get::<RaidSuggestGuardKey>() {
        guard.release(guild_id);
    }
}

/// Edite le message d'alerte : remplace la description, retire les boutons.
async fn finalize(ctx: &Context, component: &ComponentInteraction, status: &str) {
    // Accuse reception (update du message) et retire les boutons.
    let embed = danger_embed("\u{1f6a8} Suggestion anti-raid").description(status);
    let resp = CreateInteractionResponse::UpdateMessage(
        CreateInteractionResponseMessage::new()
            .embed(embed)
            .components(vec![]),
    );
    if let Err(e) = component.create_response(&ctx.http, resp).await {
        warn!(error = %e, "Echec update message suggestion anti-raid");
        // Repli : tenter d'editer le message directement.
        let mut msg = component.message.clone();
        let _ = msg
            .edit(
                &ctx.http,
                EditMessage::new()
                    .components(vec![])
                    .embed(danger_embed("\u{1f6a8} Suggestion anti-raid").description(status)),
            )
            .await;
    }
}
