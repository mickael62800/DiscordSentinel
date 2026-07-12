//! Panneau d'aide auto-genere : le bot publie et maintient, dans un salon, un
//! catalogue de TOUTES les commandes disponibles (triees par categorie, avec
//! leur description).
//!
//! - Auto-genere depuis le registre de commandes (`command_registry`) : toute
//!   nouvelle commande ajoutee a un module apparait AUTOMATIQUEMENT, sans
//!   copier-coller ni edition manuelle.
//! - Publie au demarrage du bot, une fois par process.
//! - Idempotent : supprime ses anciens messages de panneau (reperes par un
//!   marqueur en pied d'embed) puis reposte a jour — jamais de doublon.
//! - N'affiche que les categories dont le module est ACTIVE pour le serveur.

use serenity::all::{
    ChannelType, CreateChannel, CreateEmbed, CreateEmbedFooter, CreateMessage, GetMessages,
    GuildChannel, GuildId, UserId,
};
use serenity::prelude::Context;
use tracing::{info, warn};

use crate::command_registry::module_commands;
use crate::shared::api_client::BaseApiClient;
use crate::shared::discord_helpers::{guild_config_or_default, is_bot_enabled};
use crate::shared::heartbeat::ApiClientKey;

const BOT_NAME: &str = "help-bot";
/// Marqueur en pied d'embed permettant de retrouver/remplacer nos messages.
const MARKER: &str = "Sentinel · Panneau d'aide";
const DEFAULT_CHANNEL_NAME: &str = "commandes";
const DESC_MAX: usize = 4000; // marge sous la limite Discord (4096)

/// Categories affichees, dans l'ordre. Chaque categorie agrege les commandes
/// d'un ou plusieurs modules (bot_name du command_registry).
const CATEGORIES: &[(&str, &[&str])] = &[
    ("🛡️ Modération", &["moderation-bot"]),
    ("🚨 Sécurité", &["security-bot", "automod-bot"]),
    ("🎫 Tickets", &["ticket-bot"]),
    ("🙊 Confessions", &["confessions"]),
    (
        "💬 Communauté",
        &["community-bot", "progression-bot", "voice-bot", "rotation-bot"],
    ),
    (
        "🎮 Jeux",
        &[
            "game-bot",
            "coude-bot",
            "blackjack-bot",
            "slot-bot",
            "wheel-bot",
            "tamagotchi-bot",
            "influence-bot",
        ],
    ),
    ("💾 Sauvegarde", &["guild-backup-bot"]),
    ("📊 Audit", &["audit-bot"]),
    ("🧹 Nettoyage", &["cleanup-bot"]),
];

/// Deploie le panneau sur toutes les guilds connues (appele une fois au boot).
pub async fn deploy_all(ctx: &Context, bot_id: UserId, guild_ids: &[GuildId]) {
    let api = {
        let data = ctx.data.read().await;
        match data.get::<ApiClientKey>() {
            Some(a) => a.clone(),
            None => {
                warn!("help_panel: ApiClientKey absent, deploiement ignore");
                return;
            }
        }
    };
    for &gid in guild_ids {
        if let Err(e) = deploy_for_guild(ctx, &api, bot_id, gid).await {
            warn!(guild_id = %gid, error = %e, "help_panel: deploiement echoue");
        }
    }
}

async fn deploy_for_guild(
    ctx: &Context,
    api: &BaseApiClient,
    bot_id: UserId,
    guild_id: GuildId,
) -> Result<(), String> {
    let gid = guild_id.to_string();
    if !is_bot_enabled(api, &gid, BOT_NAME).await {
        return Ok(());
    }

    let channel = resolve_channel(ctx, &gid, guild_id).await?;
    let channel_id = channel.id;

    // ── Idempotence : supprime nos anciens messages de panneau ──
    if let Ok(msgs) = channel_id
        .messages(&ctx.http, GetMessages::new().limit(50))
        .await
    {
        for m in msgs {
            if m.author.id == bot_id && is_panel_message(&m) {
                let _ = channel_id.delete_message(&ctx.http, m.id).await;
            }
        }
    }

    // ── Construit + poste un embed par categorie active ──
    let mut posted = 0usize;
    // En-tete.
    let header = CreateEmbed::new()
        .title("📖 Catalogue des commandes")
        .description(
            "Toutes les commandes disponibles sur ce serveur, triées par catégorie. \
             Ce panneau est généré et mis à jour automatiquement par le bot.",
        )
        .footer(CreateEmbedFooter::new(MARKER));
    if channel_id
        .send_message(&ctx.http, CreateMessage::new().embed(header))
        .await
        .is_ok()
    {
        posted += 1;
    }

    for (label, bot_names) in CATEGORIES {
        let mut lines: Vec<String> = Vec::new();
        for bot_name in *bot_names {
            if !is_bot_enabled(api, &gid, bot_name).await {
                continue;
            }
            for cmd in module_commands(bot_name) {
                for (name, desc) in extract_commands(&cmd) {
                    lines.push(format!("**`{name}`** — {desc}"));
                }
            }
        }
        if lines.is_empty() {
            continue;
        }
        let mut body = lines.join("\n");
        if body.chars().count() > DESC_MAX {
            body = body.chars().take(DESC_MAX).collect::<String>() + "\n…";
        }
        let embed = CreateEmbed::new()
            .title(*label)
            .description(body)
            .footer(CreateEmbedFooter::new(MARKER));
        if channel_id
            .send_message(&ctx.http, CreateMessage::new().embed(embed))
            .await
            .is_ok()
        {
            posted += 1;
        }
    }

    info!(guild_id = %guild_id, channel = %channel_id, embeds = posted, "help_panel: publie");
    Ok(())
}

/// Salon cible : config `channel_id` si valide, sinon salon existant nomme
/// `commandes`, sinon on le cree.
async fn resolve_channel(
    ctx: &Context,
    gid: &str,
    guild_id: GuildId,
) -> Result<GuildChannel, String> {
    let cfg = guild_config_or_default(ctx, gid, BOT_NAME).await;

    // 1. Salon configure explicitement.
    if let Some(raw) = cfg.get("channel_id") {
        if let Ok(id) = raw.trim().parse::<u64>() {
            if id != 0 {
                if let Ok(serenity::all::Channel::Guild(ch)) =
                    serenity::all::ChannelId::new(id).to_channel(&ctx.http).await
                {
                    return Ok(ch);
                }
            }
        }
    }

    // 2. Salon existant nomme "commandes".
    let channels = guild_id
        .channels(&ctx.http)
        .await
        .map_err(|e| format!("channels: {e}"))?;
    if let Some(ch) = channels
        .values()
        .find(|c| c.name == DEFAULT_CHANNEL_NAME && c.kind == ChannelType::Text)
    {
        return Ok(ch.clone());
    }

    // 3. Creation d'un salon dedie.
    guild_id
        .create_channel(
            &ctx.http,
            CreateChannel::new(DEFAULT_CHANNEL_NAME)
                .kind(ChannelType::Text)
                .topic("Catalogue des commandes — généré automatiquement par Sentinel."),
        )
        .await
        .map_err(|e| format!("create_channel: {e}"))
}

/// `true` si le message est un de nos panneaux (marqueur en pied d'embed).
fn is_panel_message(m: &serenity::all::Message) -> bool {
    m.embeds.iter().any(|e| {
        e.footer
            .as_ref()
            .map(|f| f.text.contains(MARKER))
            .unwrap_or(false)
    })
}

/// Extrait (nom affichable, description) d'une commande. Si elle a des
/// sous-commandes, on liste chacune (`/cmd sub`). Sinon `/cmd`.
///
/// `CreateCommand` n'expose pas de getters : on le sérialise (format API
/// Discord) et on lit name/description/options.
fn extract_commands(cmd: &serenity::all::CreateCommand) -> Vec<(String, String)> {
    let v = match serde_json::to_value(cmd) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    let name = v.get("name").and_then(|x| x.as_str()).unwrap_or("");
    let desc = v
        .get("description")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();
    if name.is_empty() {
        return Vec::new();
    }

    // Sous-commandes = options de type 1 (SubCommand).
    let subs: Vec<(&str, &str)> = v
        .get("options")
        .and_then(|o| o.as_array())
        .map(|arr| {
            arr.iter()
                .filter(|o| o.get("type").and_then(|t| t.as_u64()) == Some(1))
                .filter_map(|o| {
                    let n = o.get("name").and_then(|x| x.as_str())?;
                    let d = o.get("description").and_then(|x| x.as_str()).unwrap_or("");
                    Some((n, d))
                })
                .collect()
        })
        .unwrap_or_default();

    if subs.is_empty() {
        vec![(format!("/{name}"), desc)]
    } else {
        subs.into_iter()
            .map(|(sn, sd)| (format!("/{name} {sn}"), sd.to_string()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serenity::all::{CommandOptionType, CreateCommand, CreateCommandOption};

    #[test]
    fn extract_simple_command() {
        let cmd = CreateCommand::new("kick").description("Expulse un membre");
        let out = extract_commands(&cmd);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].0, "/kick");
        assert_eq!(out[0].1, "Expulse un membre");
    }

    #[test]
    fn extract_subcommands() {
        let cmd = CreateCommand::new("security")
            .description("Sécurité")
            .add_option(CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "panic",
                "Bouton panique",
            ))
            .add_option(CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "calm",
                "Lève le panique",
            ));
        let out = extract_commands(&cmd);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].0, "/security panic");
        assert_eq!(out[0].1, "Bouton panique");
        assert_eq!(out[1].0, "/security calm");
    }

    #[test]
    fn every_registry_bot_name_has_a_category() {
        // Garde-fou : tout module a commandes doit etre range dans une categorie
        // (sinon ses commandes n'apparaitraient pas dans le panneau).
        use crate::command_registry::BOT_NAMES_WITH_COMMANDS;
        for bot_name in BOT_NAMES_WITH_COMMANDS {
            let found = CATEGORIES
                .iter()
                .any(|(_, names)| names.contains(bot_name));
            assert!(found, "module sans categorie dans le panneau : {bot_name}");
        }
    }
}
