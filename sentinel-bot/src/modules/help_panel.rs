//! Panneau d'aide auto-genere, reparti en 3 salons par AUDIENCE.
//!
//! - **Admin** : commandes exigeant ADMINISTRATOR / MANAGE_GUILD.
//! - **Modération** : commandes exigeant KICK/BAN/MODERATE/MANAGE_* (salons,
//!   messages, roles).
//! - **Membres** : commandes sans restriction de permission (tout le monde).
//!
//! Le classement est AUTOMATIQUE, dérivé de `default_member_permissions` de
//! chaque commande — donc toute nouvelle commande apparait dans le bon salon
//! sans intervention. Les 3 salons sont rangés sous une CATÉGORIE Discord
//! (configurable ; sinon le bot en crée une). A l'intérieur d'un salon, les
//! commandes restent groupées par catégorie de module (Modération, Jeux…).
//!
//! Publié au demarrage du bot (une fois/process). Idempotent : supprime ses
//! anciens messages (marqueur en pied d'embed) puis reposte à jour.

use serenity::all::{
    Channel, ChannelId, ChannelType, CreateChannel, CreateEmbed, CreateEmbedFooter, CreateMessage,
    GetMessages, GuildId, UserId,
};
use serenity::prelude::Context;
use tracing::{info, warn};

use crate::command_registry::module_commands;
use crate::shared::api_client::BaseApiClient;
use crate::shared::discord_helpers::{guild_config_or_default, is_bot_enabled};
use crate::shared::heartbeat::ApiClientKey;

const BOT_NAME: &str = "help-bot";
const MARKER: &str = "Sentinel · Panneau d'aide";
const CATEGORY_NAME: &str = "Aide commandes";
const DESC_MAX: usize = 4000;

// ── Permissions Discord (bits) pour classer les commandes ──
const ADMINISTRATOR: u64 = 1 << 3;
const MANAGE_GUILD: u64 = 1 << 5;
const KICK_MEMBERS: u64 = 1 << 1;
const BAN_MEMBERS: u64 = 1 << 2;
const MANAGE_CHANNELS: u64 = 1 << 4;
const MANAGE_MESSAGES: u64 = 1 << 13;
const MANAGE_ROLES: u64 = 1 << 28;
const MODERATE_MEMBERS: u64 = 1 << 40;

const ADMIN_MASK: u64 = ADMINISTRATOR | MANAGE_GUILD;
const MOD_MASK: u64 =
    KICK_MEMBERS | BAN_MEMBERS | MANAGE_CHANNELS | MANAGE_MESSAGES | MANAGE_ROLES | MODERATE_MEMBERS;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Audience {
    Admin,
    Moderation,
    Members,
}

impl Audience {
    const ALL: [Audience; 3] = [Audience::Admin, Audience::Moderation, Audience::Members];

    fn idx(self) -> usize {
        match self {
            Audience::Admin => 0,
            Audience::Moderation => 1,
            Audience::Members => 2,
        }
    }
    /// Nom du salon dédié (préfixe emoji + ・, comme demandé). Discord conserve
    /// l'emoji et le ・, normalise juste les lettres en minuscules.
    fn channel_name(self) -> &'static str {
        match self {
            Audience::Admin => "👑・commandes-admin",
            Audience::Moderation => "🔨・commandes-moderation",
            Audience::Members => "💬・commandes-membres",
        }
    }
    /// Clé de config (help-bot) portant la catégorie où ranger ce salon.
    fn category_key(self) -> &'static str {
        match self {
            Audience::Admin => "admin_category_id",
            Audience::Moderation => "moderation_category_id",
            Audience::Members => "membres_category_id",
        }
    }
    fn header_title(self) -> &'static str {
        match self {
            Audience::Admin => "🔐 Commandes — Administration",
            Audience::Moderation => "🛡️ Commandes — Modération",
            Audience::Members => "💬 Commandes — Membres",
        }
    }
    fn header_desc(self) -> &'static str {
        match self {
            Audience::Admin => "Commandes réservées aux administrateurs du serveur.",
            Audience::Moderation => "Commandes réservées à l'équipe de modération.",
            Audience::Members => "Commandes utilisables par tous les membres.",
        }
    }
}

/// Classe une commande d'après son `default_member_permissions`.
fn classify(perms: u64) -> Audience {
    if perms & ADMIN_MASK != 0 {
        Audience::Admin
    } else if perms & MOD_MASK != 0 {
        Audience::Moderation
    } else {
        Audience::Members
    }
}

/// Catégories de MODULE (groupement interne à chaque salon), dans l'ordre.
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

/// Deploie les panneaux sur toutes les guilds connues (appele une fois au boot).
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

    // ── Construit les buckets : audience -> Vec<(module_label, lignes)> ──
    // On n'appelle is_bot_enabled qu'une fois par module.
    let mut buckets: [Vec<(&'static str, Vec<String>)>; 3] =
        [Vec::new(), Vec::new(), Vec::new()];
    for (mod_label, bot_names) in CATEGORIES {
        let mut per_aud: [Vec<String>; 3] = [Vec::new(), Vec::new(), Vec::new()];
        for bot_name in *bot_names {
            if !is_bot_enabled(api, &gid, bot_name).await {
                continue;
            }
            for cmd in module_commands(bot_name) {
                let json = match serde_json::to_value(&cmd) {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                let audience = classify(command_perms(&json));
                for (name, desc) in extract_commands(&json) {
                    per_aud[audience.idx()].push(format!("**`{name}`** — {desc}"));
                }
            }
        }
        for aud in Audience::ALL {
            let lines = std::mem::take(&mut per_aud[aud.idx()]);
            if !lines.is_empty() {
                buckets[aud.idx()].push((mod_label, lines));
            }
        }
    }

    // ── Config (catégorie par audience) ──
    let cfg = guild_config_or_default(ctx, &gid, BOT_NAME).await;
    // Catégorie par défaut partagée, créée à la demande si une audience n'a pas
    // de catégorie configurée (évite d'en créer plusieurs).
    let mut default_category: Option<ChannelId> = None;

    // ── Un salon par audience non vide, sous SA catégorie ──
    for aud in Audience::ALL {
        let entries = &buckets[aud.idx()];
        if entries.is_empty() {
            continue;
        }
        let category_id =
            resolve_audience_category(ctx, &cfg, guild_id, aud, &mut default_category).await?;
        let channel_id = resolve_channel(ctx, guild_id, category_id, aud).await?;
        purge_old_panels(ctx, channel_id, bot_id).await;

        // En-tete.
        let header = CreateEmbed::new()
            .title(aud.header_title())
            .description(aud.header_desc())
            .footer(CreateEmbedFooter::new(MARKER));
        let _ = channel_id
            .send_message(ctx, CreateMessage::new().embed(header))
            .await;

        // Un embed par categorie de module presente.
        for (mod_label, lines) in entries {
            let mut body = lines.join("\n");
            if body.chars().count() > DESC_MAX {
                body = body.chars().take(DESC_MAX).collect::<String>() + "\n…";
            }
            let embed = CreateEmbed::new()
                .title(*mod_label)
                .description(body)
                .footer(CreateEmbedFooter::new(MARKER));
            let _ = channel_id
                .send_message(ctx, CreateMessage::new().embed(embed))
                .await;
        }
        info!(guild_id = %guild_id, channel = %channel_id, "help_panel: audience publiee");
    }
    Ok(())
}

/// Catégorie où ranger le salon d'une audience : celle configurée pour cette
/// audience (dropdown `<audience>_category_id`) si valide, sinon une catégorie
/// par défaut partagée (créée/réutilisée une seule fois).
async fn resolve_audience_category(
    ctx: &Context,
    cfg: &std::collections::HashMap<String, String>,
    guild_id: GuildId,
    aud: Audience,
    default_category: &mut Option<ChannelId>,
) -> Result<ChannelId, String> {
    // 1. Catégorie configurée pour CETTE audience.
    if let Some(raw) = cfg.get(aud.category_key()) {
        if let Ok(id) = raw.trim().parse::<u64>() {
            if id != 0 {
                if let Ok(Channel::Guild(ch)) = ChannelId::new(id).to_channel(&ctx.http).await {
                    if ch.kind == ChannelType::Category {
                        return Ok(ch.id);
                    }
                }
            }
        }
    }
    // 2. Repli : catégorie par défaut partagée (mémorisée pour ne pas la recréer).
    if let Some(id) = default_category {
        return Ok(*id);
    }
    let id = find_or_create_default_category(ctx, guild_id).await?;
    *default_category = Some(id);
    Ok(id)
}

/// Catégorie par défaut `CATEGORY_NAME` : réutilise l'existante ou la crée.
async fn find_or_create_default_category(
    ctx: &Context,
    guild_id: GuildId,
) -> Result<ChannelId, String> {
    let channels = guild_id
        .channels(&ctx.http)
        .await
        .map_err(|e| format!("channels: {e}"))?;
    if let Some(ch) = channels
        .values()
        .find(|c| c.kind == ChannelType::Category && c.name == CATEGORY_NAME)
    {
        return Ok(ch.id);
    }
    guild_id
        .create_channel(
            &ctx.http,
            CreateChannel::new(CATEGORY_NAME).kind(ChannelType::Category),
        )
        .await
        .map(|c| c.id)
        .map_err(|e| format!("create category: {e}"))
}

/// Salon d'une audience sous la catégorie : réutilise s'il existe, sinon crée.
async fn resolve_channel(
    ctx: &Context,
    guild_id: GuildId,
    category_id: ChannelId,
    aud: Audience,
) -> Result<ChannelId, String> {
    let channels = guild_id
        .channels(&ctx.http)
        .await
        .map_err(|e| format!("channels: {e}"))?;
    if let Some(ch) = channels.values().find(|c| {
        c.parent_id == Some(category_id)
            && c.kind == ChannelType::Text
            && c.name == aud.channel_name()
    }) {
        return Ok(ch.id);
    }
    guild_id
        .create_channel(
            &ctx.http,
            CreateChannel::new(aud.channel_name())
                .kind(ChannelType::Text)
                .category(category_id),
        )
        .await
        .map(|c| c.id)
        .map_err(|e| format!("create channel: {e}"))
}

/// Supprime nos anciens messages de panneau dans un salon (idempotence).
async fn purge_old_panels(ctx: &Context, channel_id: ChannelId, bot_id: UserId) {
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
}

fn is_panel_message(m: &serenity::all::Message) -> bool {
    m.embeds.iter().any(|e| {
        e.footer
            .as_ref()
            .map(|f| f.text.contains(MARKER))
            .unwrap_or(false)
    })
}

/// Lit `default_member_permissions` (bitfield en chaine) d'une commande sérialisée.
fn command_perms(v: &serde_json::Value) -> u64 {
    v.get("default_member_permissions")
        .and_then(|x| x.as_str())
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0)
}

/// Extrait (nom affichable, description) d'une commande sérialisée. Sous-commandes
/// listées individuellement (`/cmd sub`).
fn extract_commands(v: &serde_json::Value) -> Vec<(String, String)> {
    let name = v.get("name").and_then(|x| x.as_str()).unwrap_or("");
    let desc = v
        .get("description")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();
    if name.is_empty() {
        return Vec::new();
    }
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
    use serenity::all::{CommandOptionType, CreateCommand, CreateCommandOption, Permissions};

    fn json(cmd: &CreateCommand) -> serde_json::Value {
        serde_json::to_value(cmd).unwrap()
    }

    #[test]
    fn classify_by_permission() {
        assert!(matches!(classify(ADMINISTRATOR), Audience::Admin));
        assert!(matches!(classify(MANAGE_GUILD), Audience::Admin));
        assert!(matches!(classify(BAN_MEMBERS), Audience::Moderation));
        assert!(matches!(classify(MODERATE_MEMBERS), Audience::Moderation));
        assert!(matches!(classify(MANAGE_CHANNELS), Audience::Moderation));
        assert!(matches!(classify(0), Audience::Members));
    }

    #[test]
    fn command_perms_reads_default_member_permissions() {
        let admin = CreateCommand::new("backup")
            .description("x")
            .default_member_permissions(Permissions::ADMINISTRATOR);
        assert!(matches!(classify(command_perms(&json(&admin))), Audience::Admin));

        let modo = CreateCommand::new("ban")
            .description("x")
            .default_member_permissions(Permissions::BAN_MEMBERS);
        assert!(matches!(
            classify(command_perms(&json(&modo))),
            Audience::Moderation
        ));

        let public = CreateCommand::new("confess").description("x");
        assert!(matches!(
            classify(command_perms(&json(&public))),
            Audience::Members
        ));
    }

    #[test]
    fn extract_simple_and_subcommands() {
        let simple = json(&CreateCommand::new("kick").description("Expulse"));
        let out = extract_commands(&simple);
        assert_eq!(out, vec![("/kick".to_string(), "Expulse".to_string())]);

        let sub = json(&CreateCommand::new("security")
            .description("Sécurité")
            .add_option(CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "panic",
                "Panique",
            )));
        let out = extract_commands(&sub);
        assert_eq!(out, vec![("/security panic".to_string(), "Panique".to_string())]);
    }

    #[test]
    fn every_registry_bot_name_has_a_category() {
        use crate::command_registry::BOT_NAMES_WITH_COMMANDS;
        for bot_name in BOT_NAMES_WITH_COMMANDS {
            let found = CATEGORIES.iter().any(|(_, names)| names.contains(bot_name));
            assert!(found, "module sans categorie dans le panneau : {bot_name}");
        }
    }
}
