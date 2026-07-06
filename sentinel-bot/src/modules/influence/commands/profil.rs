//! Commande `/influence-profil [joueur]` — profil d'un citoyen.
//!
//! Applique le principe « chiffre pour soi / palier pour les autres » : quand
//! on consulte son propre profil, les valeurs exactes s'affichent ; sinon on ne
//! voit que des paliers narratifs.

use serenity::all::{
    CommandInteraction, CommandOptionType, Context, CreateCommand, CreateCommandOption,
    CreateEmbed, CreateEmbedFooter,
};

use crate::modules::influence::api_client;
use crate::shared::heartbeat::ApiClientKey;
use crate::shared::discord_helpers::{option_user, reply_ephemeral, reply_ephemeral_embed, require_guild_id};

pub fn register() -> CreateCommand {
    CreateCommand::new("influence-profil")
        .description("Affiche le profil Influence d'un citoyen (capitaux)")
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::User,
                "joueur",
                "Le citoyen a consulter (par defaut : toi)",
            )
            .required(false),
        )
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    let Some(guild_id) = require_guild_id(ctx, command).await else {
        return;
    };

    let viewer_id = command.user.id.to_string();
    // Cible : l'option "joueur" si fournie, sinon soi-meme.
    let (target_id, target_name) = match option_user(&command.data.options, "joueur") {
        Some(uid) => {
            let name = command
                .data
                .resolved
                .users
                .get(&uid)
                .map(|u| u.name.clone())
                .unwrap_or_else(|| uid.to_string());
            (uid.to_string(), name)
        }
        None => (viewer_id.clone(), command.user.name.clone()),
    };

    let api = {
        let data = ctx.data.read().await;
        match data.get::<ApiClientKey>() {
            Some(a) => a.clone(),
            None => return,
        }
    };

    let profile = match api_client::view_profile(&api, &guild_id, &viewer_id, &target_id, &target_name).await
    {
        Ok(p) => p,
        Err(e) => {
            reply_ephemeral(ctx, command, &format!("Erreur : {e}")).await;
            return;
        }
    };

    reply_ephemeral_embed(ctx, command, build_embed(&profile)).await;
}

/// Formatte une valeur de capital : chiffre exact si connu, sinon palier.
fn cap_line(view: &api_client::CapitalView) -> String {
    match view.exact {
        Some(v) => format!("{}  ({}  ·  **{v}**)", view.stars, view.tier),
        None => format!("{}  ({})", view.stars, view.tier),
    }
}

fn build_embed(p: &api_client::ProfileView) -> CreateEmbed {
    let titre = if p.is_self {
        format!("🎖️ Ton profil — {}", p.username)
    } else {
        format!("🎖️ Profil de {}", p.username)
    };

    let reputation = match p.reputation_exact {
        Some(v) => format!("{}  (**{v}**)", p.reputation_tier),
        None => p.reputation_tier.clone(),
    };

    let footer = if p.is_self {
        "Tu vois tes chiffres exacts — les autres ne voient que tes paliers."
    } else {
        "Tu ne vois que des paliers : les chiffres exacts restent prives."
    };

    let mut embed = CreateEmbed::new()
        .title(titre)
        .color(0x8E44AD)
        .field("🏛️ Influence", cap_line(&p.influence), false)
        .field("💰 Argent", cap_line(&p.money), false)
        .field("⭐ Réputation", reputation, false)
        .field("🕵️ Information", cap_line(&p.information), false)
        .field("🤝 Réseau", cap_line(&p.network), false);
    // Reputation multi-dimensionnelle (seulement sur son propre profil).
    if let Some(d) = &p.reputation_dims {
        embed = embed.field(
            "📊 Réputation détaillée",
            format!(
                "Fiabilité **{}** · Popularité **{}** · Notoriété **{}** · Transparence **{}**",
                d.reliability, d.popularity, d.notoriety, d.transparency
            ),
            false,
        );
    }
    embed.footer(CreateEmbedFooter::new(footer))
}
