//! Moteur générique de « salons de DOMAINE » pour les jeux : un salon texte par
//! entité du jeu, rangés sous une catégorie configurable, avec RESTRICTION des
//! commandes à leur salon. Piloté par une [`GameChannelsSpec`] fournie par
//! chaque jeu (Influence, Coude, …) — aucune logique dupliquée.
//!
//! Opt-in via la config `<enabled_key>`. Idempotent : les salons sont retrouvés
//! par nom (créés si absents) au démarrage du bot.

use std::collections::HashMap;
use std::sync::LazyLock;

use dashmap::DashMap;
use serenity::all::{
    Channel, ChannelId, ChannelType, CommandInteraction, Context, CreateChannel,
    CreateInteractionResponse, CreateInteractionResponseMessage, GuildId,
};
use tracing::{info, warn};

use crate::shared::api_client::BaseApiClient;
use crate::shared::discord_helpers::guild_config_or_default;

/// Un domaine = une entité du jeu = un salon texte.
pub struct DomainSpec {
    /// Identifiant stable du domaine (clé de cache, jamais affiché).
    pub key: &'static str,
    /// Nom du salon (Discord normalise les lettres en minuscules/tirets).
    pub channel_name: &'static str,
    /// Sujet (topic) du salon.
    pub topic: &'static str,
}

/// Descripteur d'un jeu pour le moteur de salons de domaine.
pub struct GameChannelsSpec {
    /// Nom du module bot (ex. `"influence-bot"`), pour lire la config.
    pub bot_name: &'static str,
    /// Clé config booléenne activant les salons de domaine.
    pub enabled_key: &'static str,
    /// Clé config de la catégorie (id) où ranger les salons.
    pub category_key: &'static str,
    /// Nom de la catégorie créée/réutilisée si `category_key` est vide.
    pub category_name: &'static str,
    /// Domaines du jeu.
    pub domains: &'static [DomainSpec],
    /// Commande slash -> clé de domaine (ou `None` si non restreinte).
    pub command_domain: fn(&str) -> Option<&'static str>,
}

/// Cache (guild_id, bot_name, domain_key) -> salon. Peuplé au démarrage + à la volée.
static REGISTRY: LazyLock<DashMap<(u64, &'static str, &'static str), ChannelId>> =
    LazyLock::new(DashMap::new);

fn enabled(cfg: &HashMap<String, String>, spec: &GameChannelsSpec) -> bool {
    BaseApiClient::config_bool(cfg, "enabled", false)
        && BaseApiClient::config_bool(cfg, spec.enabled_key, false)
}

/// Déploie les salons de domaine au démarrage, pour chaque serveur où le jeu +
/// les salons de domaine sont activés.
pub async fn deploy(ctx: &Context, spec: &GameChannelsSpec, guild_ids: &[GuildId]) {
    for &gid in guild_ids {
        let cfg = guild_config_or_default(ctx, &gid.to_string(), spec.bot_name).await;
        if !enabled(&cfg, spec) {
            continue;
        }
        let Some(category) = resolve_category(ctx, spec, &cfg, gid).await else {
            continue;
        };
        for dom in spec.domains {
            if let Some(id) = ensure_channel(ctx, gid, category, dom).await {
                REGISTRY.insert((gid.get(), spec.bot_name, dom.key), id);
            }
        }
        info!(guild_id = %gid, game = spec.bot_name, "salons de domaine déployés");
    }
}

/// Vérifie qu'une commande de domaine est utilisée dans SON salon. Renvoie
/// `true` si OK (commande non restreinte, fonctionnalité désactivée, salon
/// introuvable, ou bon salon : fail-open pour ne jamais bloquer à tort). Sinon
/// répond un pointeur éphémère vers le bon salon et renvoie `false`.
pub async fn enforce(ctx: &Context, spec: &GameChannelsSpec, command: &CommandInteraction) -> bool {
    let Some(domain_key) = (spec.command_domain)(command.data.name.as_str()) else {
        return true; // commande non rattachée à un domaine
    };
    let Some(gid) = command.guild_id else {
        return true;
    };
    let cfg = guild_config_or_default(ctx, &gid.to_string(), spec.bot_name).await;
    if !enabled(&cfg, spec) {
        return true; // fonctionnalité désactivée : pas de restriction
    }
    let Some(target) = resolve_domain_channel(ctx, spec, &cfg, gid, domain_key).await else {
        return true; // salon introuvable : on ne bloque pas
    };
    if command.channel_id == target {
        return true;
    }
    let _ = command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .ephemeral(true)
                    .content(format!("↪️ Cette commande s'utilise dans <#{target}>.")),
            ),
        )
        .await;
    false
}

/// Salon d'un domaine : cache, sinon recherche par nom sous la catégorie.
async fn resolve_domain_channel(
    ctx: &Context,
    spec: &GameChannelsSpec,
    cfg: &HashMap<String, String>,
    gid: GuildId,
    domain_key: &'static str,
) -> Option<ChannelId> {
    if let Some(e) = REGISTRY.get(&(gid.get(), spec.bot_name, domain_key)) {
        return Some(*e);
    }
    let category = resolve_category(ctx, spec, cfg, gid).await?;
    let dom = spec.domains.iter().find(|d| d.key == domain_key)?;
    let id = find_channel(ctx, gid, category, dom).await?;
    REGISTRY.insert((gid.get(), spec.bot_name, domain_key), id);
    Some(id)
}

/// Catégorie des salons : config `category_key`, sinon `category_name`
/// (réutilisée ou créée).
async fn resolve_category(
    ctx: &Context,
    spec: &GameChannelsSpec,
    cfg: &HashMap<String, String>,
    gid: GuildId,
) -> Option<ChannelId> {
    if let Some(raw) = cfg.get(spec.category_key).filter(|s| !s.trim().is_empty()) {
        if let Ok(id) = raw.trim().parse::<u64>() {
            if let Ok(Channel::Guild(ch)) = ChannelId::new(id).to_channel(&ctx.http).await {
                if ch.kind == ChannelType::Category {
                    return Some(ch.id);
                }
            }
        }
    }
    let channels = gid.channels(&ctx.http).await.ok()?;
    if let Some(ch) = channels
        .values()
        .find(|c| c.kind == ChannelType::Category && c.name == spec.category_name)
    {
        return Some(ch.id);
    }
    gid.create_channel(
        &ctx.http,
        CreateChannel::new(spec.category_name).kind(ChannelType::Category),
    )
    .await
    .ok()
    .map(|c| c.id)
}

/// Cherche le salon d'un domaine sous la catégorie (par nom).
async fn find_channel(
    ctx: &Context,
    gid: GuildId,
    category: ChannelId,
    dom: &DomainSpec,
) -> Option<ChannelId> {
    let channels = gid.channels(&ctx.http).await.ok()?;
    channels
        .values()
        .find(|c| {
            c.parent_id == Some(category)
                && c.kind == ChannelType::Text
                && c.name == dom.channel_name
        })
        .map(|c| c.id)
}

/// Retrouve ou crée le salon d'un domaine sous la catégorie.
async fn ensure_channel(
    ctx: &Context,
    gid: GuildId,
    category: ChannelId,
    dom: &DomainSpec,
) -> Option<ChannelId> {
    if let Some(id) = find_channel(ctx, gid, category, dom).await {
        return Some(id);
    }
    match gid
        .create_channel(
            &ctx.http,
            CreateChannel::new(dom.channel_name)
                .kind(ChannelType::Text)
                .topic(dom.topic)
                .category(category),
        )
        .await
    {
        Ok(c) => Some(c.id),
        Err(e) => {
            warn!(guild_id = %gid, channel = dom.channel_name, error = %e, "création salon de domaine échouée");
            None
        }
    }
}
