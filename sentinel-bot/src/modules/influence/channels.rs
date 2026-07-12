//! Influence — salons de DOMAINE : un salon texte par entité du jeu (citoyen,
//! organisations, lois-votes, renseignement, actualité), rangés sous une
//! catégorie configurable, avec RESTRICTION des commandes à leur salon.
//!
//! Toute la mécanique est portée par le moteur générique
//! [`crate::shared::game_channels`] ; ce module ne fait que décrire Influence.

use serenity::all::{CommandInteraction, Context, GuildId};

use crate::shared::game_channels::{deploy, enforce as engine_enforce, DomainSpec, GameChannelsSpec};

/// Commande slash Influence -> clé de domaine (salon) auquel elle est rattachée.
fn command_domain(cmd: &str) -> Option<&'static str> {
    match cmd {
        "influence-profil" | "capital" | "transfert" => Some("citoyen"),
        "org" => Some("organisations"),
        "loi" | "vote" => Some("lois-votes"),
        "enquete" | "dossier" | "reveler" => Some("renseignement"),
        "actu" | "archives" => Some("actualite"),
        _ => None,
    }
}

static SPEC: GameChannelsSpec = GameChannelsSpec {
    bot_name: "influence-bot",
    enabled_key: "influence_domain_channels_enabled",
    category_key: "influence_domain_category_id",
    category_name: "🏛️ Influence",
    domains: &[
        DomainSpec {
            key: "citoyen",
            channel_name: "citoyen",
            topic: "Profil, capital et transferts des citoyens.",
        },
        DomainSpec {
            key: "organisations",
            channel_name: "organisations",
            topic: "Fonder / rejoindre une organisation, annuaire et classement.",
        },
        DomainSpec {
            key: "lois-votes",
            channel_name: "lois-votes",
            topic: "Propositions de lois, financement et votes.",
        },
        DomainSpec {
            key: "renseignement",
            channel_name: "renseignement",
            topic: "Enquêtes, dossiers et révélations.",
        },
        DomainSpec {
            key: "actualite",
            channel_name: "actualite",
            topic: "Fil d'actualité et archives du serveur.",
        },
    ],
    command_domain,
};

/// Déploie les salons de domaine Influence au démarrage.
pub async fn deploy_domain_channels(ctx: &Context, guild_ids: &[GuildId]) {
    deploy(ctx, &SPEC, guild_ids).await;
}

/// Restreint une commande Influence à son salon de domaine. Voir
/// [`crate::shared::game_channels::enforce`].
pub async fn enforce(ctx: &Context, command: &CommandInteraction) -> bool {
    engine_enforce(ctx, &SPEC, command).await
}
