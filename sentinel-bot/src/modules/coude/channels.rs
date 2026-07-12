//! Coude — salons de DOMAINE : un salon texte par entité du jeu (combat,
//! personnage, économie, fun), rangés sous une catégorie configurable, avec
//! RESTRICTION des commandes à leur salon.
//!
//! Toute la mécanique est portée par le moteur générique
//! [`crate::shared::game_channels`] ; ce module ne fait que décrire Coude.

use serenity::all::{CommandInteraction, Context, GuildId};

use crate::shared::game_channels::{deploy, enforce as engine_enforce, DomainSpec, GameChannelsSpec};

/// Commande slash Coude -> clé de domaine (salon) auquel elle est rattachée.
/// `aide` et `taunts-channel` (config admin) restent non restreintes.
fn command_domain(cmd: &str) -> Option<&'static str> {
    match cmd {
        "coude" | "coude-amical" | "accepter" | "refuser" | "annuler" => Some("combat"),
        "profil" | "classe" | "train" | "hp" | "repos" | "potion" | "reset-stats" => {
            Some("personnage")
        }
        "voler" | "donner" | "tout-ou-rien" | "cagnotte" | "shop" | "resume" | "leaderboard"
        | "memorial" => Some("economie"),
        "prank" | "no-taunts" => Some("fun"),
        _ => None,
    }
}

static SPEC: GameChannelsSpec = GameChannelsSpec {
    bot_name: "coude-bot",
    enabled_key: "coude_domain_channels_enabled",
    category_key: "coude_domain_category_id",
    category_name: "🥊 Coude",
    domains: &[
        DomainSpec {
            key: "combat",
            channel_name: "combat",
            topic: "Défis, combats amicaux et gestion des duels en cours.",
        },
        DomainSpec {
            key: "personnage",
            channel_name: "personnage",
            topic: "Profil, classe, entraînement, PV, repos et potions.",
        },
        DomainSpec {
            key: "economie",
            channel_name: "economie",
            topic: "Vols, dons, paris, cagnotte, boutique et classement.",
        },
        DomainSpec {
            key: "fun",
            channel_name: "fun",
            topic: "Pranks et gestion des taunts.",
        },
    ],
    command_domain,
};

/// Déploie les salons de domaine Coude au démarrage.
pub async fn deploy_domain_channels(ctx: &Context, guild_ids: &[GuildId]) {
    deploy(ctx, &SPEC, guild_ids).await;
}

/// Restreint une commande Coude à son salon de domaine. Voir
/// [`crate::shared::game_channels::enforce`].
pub async fn enforce(ctx: &Context, command: &CommandInteraction) -> bool {
    engine_enforce(ctx, &SPEC, command).await
}
