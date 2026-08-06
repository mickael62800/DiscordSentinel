//! Commande `/bump-statut` : force le rafraichissement de la carte d'etat des
//! bumps/votes dans le salon configure, sans attendre le cycle de 5 min.

use serenity::all::{CommandInteraction, Context, CreateCommand};

use crate::shared::discord_helpers::{defer_ephemeral, followup_ephemeral_embed};
use crate::shared::embeds::{moderate_embed, success_embed};

use super::{refresh_guild_card, CardRefresh};

pub fn register() -> CreateCommand {
    CreateCommand::new("bump-statut")
        .description("Rafraichir la carte d'etat des bumps & votes (admin)")
        .default_member_permissions(serenity::all::Permissions::ADMINISTRATOR)
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    let Some(guild_id) = command.guild_id else {
        let embed = moderate_embed("Erreur")
            .description("Cette commande ne peut etre utilisee que sur un serveur.");
        crate::shared::discord_helpers::reply_ephemeral_embed(ctx, command, embed).await;
        return;
    };

    // Fail-closed : sans permission ADMINISTRATOR fournie par Discord, on refuse.
    let is_admin = command
        .member
        .as_ref()
        .and_then(|m| m.permissions)
        .map(|p| p.administrator())
        .unwrap_or(false);
    if !is_admin {
        let embed =
            moderate_embed("Erreur").description("Vous n'avez pas la permission **Administrateur**.");
        crate::shared::discord_helpers::reply_ephemeral_embed(ctx, command, embed).await;
        return;
    }

    // Le rafraichissement fait un appel API + un post/edit Discord : au-dela des
    // 3 s d'Discord, d'ou le defer.
    defer_ephemeral(ctx, command).await;

    let embed = match refresh_guild_card(ctx, guild_id.get()).await {
        CardRefresh::Posted => success_embed("Carte rafraichie")
            .description("La carte d'etat des bumps a ete mise a jour dans le salon configure."),
        CardRefresh::Disabled => moderate_embed("Module desactive")
            .description("Le module Bump est desactive sur ce serveur (voir **Composants**)."),
        CardRefresh::NoChannel => moderate_embed("Salon manquant").description(
            "Aucun **salon des bumps** n'est configure. Renseigne-le dans **Composants**.",
        ),
        CardRefresh::NoPlatforms => moderate_embed("Aucune plateforme").description(
            "Aucune plateforme de bump n'est activee. Active-en au moins une dans **Composants**.",
        ),
    };
    followup_ephemeral_embed(ctx, command, embed).await;
}
