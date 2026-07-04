//! Commande `/capital` — vue detaillee de tes capitaux + historique.
//!
//! Reserve a soi-meme (chiffres exacts), contrairement a `/influence-profil`
//! qui montre des paliers aux tiers. Affiche aussi les derniers mouvements.

use serenity::all::{
    CommandInteraction, Context, CreateCommand, CreateEmbed, CreateEmbedFooter,
};

use crate::modules::influence::api_client;
use crate::shared::discord_helpers::{reply_ephemeral, reply_ephemeral_embed, require_guild_id};
use crate::shared::heartbeat::ApiClientKey;

pub fn register() -> CreateCommand {
    CreateCommand::new("capital").description("Affiche tes capitaux exacts et ton historique")
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    let Some(guild_id) = require_guild_id(ctx, command).await else {
        return;
    };
    let api = {
        let data = ctx.data.read().await;
        match data.get::<ApiClientKey>() {
            Some(a) => a.clone(),
            None => return,
        }
    };

    let ov = match api_client::view_capital(
        &api,
        &guild_id,
        &command.user.id.to_string(),
        &command.user.name,
    )
    .await
    {
        Ok(o) => o,
        Err(e) => {
            reply_ephemeral(ctx, command, &format!("Erreur : {e}")).await;
            return;
        }
    };

    let capitals = ov
        .lines
        .iter()
        .map(|l| format!("{} **{}** : {}", l.emoji, l.label, l.value))
        .collect::<Vec<_>>()
        .join("\n");

    let history = if ov.movements.is_empty() {
        "*Aucun mouvement récent.*".to_string()
    } else {
        ov.movements
            .iter()
            .map(|m| {
                let sign = if m.delta >= 0 { "+" } else { "" };
                format!("{} {}{} — {}", m.emoji, sign, m.delta, m.reason)
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    let embed = CreateEmbed::new()
        .title("💼 Tes capitaux")
        .color(0x8E44AD)
        .field("Soldes", capitals, false)
        .field("Derniers mouvements", history, false)
        .footer(CreateEmbedFooter::new(
            "Transforme tes capitaux avec /transfert.",
        ));
    reply_ephemeral_embed(ctx, command, embed).await;
}
