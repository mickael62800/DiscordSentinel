//! Influence — salons de DOMAINE : un salon texte par entité du jeu (citoyen,
//! organisations, lois-votes, renseignement, actualité), rangés sous une
//! catégorie configurable, avec RESTRICTION des commandes à leur salon.
//!
//! Opt-in via la config `influence_domain_channels_enabled`. Idempotent : les
//! salons sont retrouvés par nom (créés si absents) au démarrage du bot.

use std::sync::LazyLock;

use dashmap::DashMap;
use serenity::all::{
    Channel, ChannelId, ChannelType, CommandInteraction, Context, CreateChannel,
    CreateInteractionResponse, CreateInteractionResponseMessage, GuildId,
};
use tracing::{info, warn};

use super::MODULE_BOT_NAME;
use crate::shared::api_client::BaseApiClient;
use crate::shared::discord_helpers::guild_config_or_default;

const CATEGORY_NAME: &str = "🏛️ Influence";

/// Entités du jeu = salons de domaine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Domain {
    Citoyen,
    Organisations,
    LoisVotes,
    Renseignement,
    Actualite,
}

impl Domain {
    const ALL: [Domain; 5] = [
        Domain::Citoyen,
        Domain::Organisations,
        Domain::LoisVotes,
        Domain::Renseignement,
        Domain::Actualite,
    ];

    /// Nom du salon (Discord normalise en minuscules/tirets).
    fn channel_name(self) -> &'static str {
        match self {
            Domain::Citoyen => "citoyen",
            Domain::Organisations => "organisations",
            Domain::LoisVotes => "lois-votes",
            Domain::Renseignement => "renseignement",
            Domain::Actualite => "actualite",
        }
    }
    fn topic(self) -> &'static str {
        match self {
            Domain::Citoyen => "Profil, capital et transferts des citoyens.",
            Domain::Organisations => "Fonder / rejoindre une organisation, annuaire et classement.",
            Domain::LoisVotes => "Propositions de lois, financement et votes.",
            Domain::Renseignement => "Enquêtes, dossiers et révélations.",
            Domain::Actualite => "Fil d'actualité et archives du serveur.",
        }
    }
}

/// Commande slash Influence -> domaine (salon) auquel elle est rattachée.
pub fn domain_for_command(cmd: &str) -> Option<Domain> {
    match cmd {
        "influence-profil" | "capital" | "transfert" => Some(Domain::Citoyen),
        "org" => Some(Domain::Organisations),
        "loi" | "vote" => Some(Domain::LoisVotes),
        "enquete" | "dossier" | "reveler" => Some(Domain::Renseignement),
        "actu" | "archives" => Some(Domain::Actualite),
        _ => None,
    }
}

/// Cache (guild_id, domaine) -> salon. Peuplé au démarrage + à la volée.
static REGISTRY: LazyLock<DashMap<(u64, Domain), ChannelId>> = LazyLock::new(DashMap::new);

fn domain_channels_enabled(cfg: &std::collections::HashMap<String, String>) -> bool {
    BaseApiClient::config_bool(cfg, "enabled", false)
        && BaseApiClient::config_bool(cfg, "influence_domain_channels_enabled", false)
}

/// Déploie les salons de domaine au démarrage (une fois), pour chaque serveur
/// où Influence + les salons de domaine sont activés.
pub async fn deploy_domain_channels(ctx: &Context, guild_ids: &[GuildId]) {
    for &gid in guild_ids {
        let cfg = guild_config_or_default(ctx, &gid.to_string(), MODULE_BOT_NAME).await;
        if !domain_channels_enabled(&cfg) {
            continue;
        }
        let Some(category) = resolve_category(ctx, &cfg, gid).await else {
            continue;
        };
        for domain in Domain::ALL {
            if let Some(id) = ensure_channel(ctx, gid, category, domain).await {
                REGISTRY.insert((gid.get(), domain), id);
            }
        }
        info!(guild_id = %gid, "influence: salons de domaine déployés");
    }
}

/// Vérifie qu'une commande de domaine est utilisée dans SON salon. Renvoie
/// `true` si OK (ou si la fonctionnalité est désactivée / salon introuvable :
/// fail-open pour ne jamais bloquer un joueur à tort). Sinon répond un pointeur
/// éphémère vers le bon salon et renvoie `false`.
pub async fn enforce(ctx: &Context, command: &CommandInteraction, domain: Domain) -> bool {
    let Some(gid) = command.guild_id else {
        return true;
    };
    let cfg = guild_config_or_default(ctx, &gid.to_string(), MODULE_BOT_NAME).await;
    if !domain_channels_enabled(&cfg) {
        return true; // fonctionnalité désactivée : pas de restriction
    }
    let Some(target) = resolve_domain_channel(ctx, &cfg, gid, domain).await else {
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
    cfg: &std::collections::HashMap<String, String>,
    gid: GuildId,
    domain: Domain,
) -> Option<ChannelId> {
    if let Some(e) = REGISTRY.get(&(gid.get(), domain)) {
        return Some(*e);
    }
    let category = resolve_category(ctx, cfg, gid).await?;
    let id = find_channel(ctx, gid, category, domain).await?;
    REGISTRY.insert((gid.get(), domain), id);
    Some(id)
}

/// Catégorie des salons de domaine : config `influence_domain_category_id`,
/// sinon catégorie `CATEGORY_NAME` (réutilisée ou créée).
async fn resolve_category(
    ctx: &Context,
    cfg: &std::collections::HashMap<String, String>,
    gid: GuildId,
) -> Option<ChannelId> {
    if let Some(raw) = cfg
        .get("influence_domain_category_id")
        .filter(|s| !s.trim().is_empty())
    {
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
        .find(|c| c.kind == ChannelType::Category && c.name == CATEGORY_NAME)
    {
        return Some(ch.id);
    }
    gid.create_channel(
        &ctx.http,
        CreateChannel::new(CATEGORY_NAME).kind(ChannelType::Category),
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
    domain: Domain,
) -> Option<ChannelId> {
    let channels = gid.channels(&ctx.http).await.ok()?;
    channels
        .values()
        .find(|c| {
            c.parent_id == Some(category)
                && c.kind == ChannelType::Text
                && c.name == domain.channel_name()
        })
        .map(|c| c.id)
}

/// Retrouve ou crée le salon d'un domaine sous la catégorie.
async fn ensure_channel(
    ctx: &Context,
    gid: GuildId,
    category: ChannelId,
    domain: Domain,
) -> Option<ChannelId> {
    if let Some(id) = find_channel(ctx, gid, category, domain).await {
        return Some(id);
    }
    match gid
        .create_channel(
            &ctx.http,
            CreateChannel::new(domain.channel_name())
                .kind(ChannelType::Text)
                .topic(domain.topic())
                .category(category),
        )
        .await
    {
        Ok(c) => Some(c.id),
        Err(e) => {
            warn!(guild_id = %gid, error = %e, "influence: création salon de domaine échouée");
            None
        }
    }
}
