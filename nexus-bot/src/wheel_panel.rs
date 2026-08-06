//! Panneau permanent de la Roue du Destin.
//!
//! Porte depuis `sentinel-bot/src/modules/wheel/{setup,buttons}.rs`, supprime
//! au commit ff6e8a46 quand les jeux ont quitte sentinel. Le portage vers
//! nexus n'avait repris que `/roue` ; le panneau manquait depuis.
//!
//! Pourquoi un panneau plutot que la seule commande : `/roue` suppose de
//! connaitre la commande. Un bouton epingle en bas du salon se voit, et c'est
//! ce qui fait revenir les gens tous les jours.
//!
//! Deux differences assumees avec l'original :
//!
//!   - la duree de l'animation est une constante. Dans sentinel elle etait
//!     lue dans la config du serveur ; nexus n'a pas encore de config par
//!     guilde cote bot, et un reglage qu'aucun service ne lit vaut moins que
//!     pas de reglage du tout.
//!   - pas de journal de jeu : nexus n'a pas de salon de logs configurable.

use std::time::Duration;

use serenity::all::{
    ButtonStyle, ChannelId, CommandInteraction, ComponentInteraction, Context, CreateActionRow,
    CreateButton, CreateCommand, CreateInteractionResponse, CreateInteractionResponseMessage,
    CreateMessage, EditMessage, GetMessages, MessageId, Permissions,
};
use tracing::{info, warn};

use crate::api_client::ApiClient;
use crate::embeds;

/// `custom_id` du bouton. Stable et sans donnee variable : le panneau doit
/// rester cliquable des semaines apres avoir ete poste, y compris apres un
/// redemarrage du bot.
pub const PANEL_SPIN_ID: &str = "roue_panel_spin";

/// Titre EXACT du panneau. Sert a reperer les anciens panneaux a supprimer
/// lors du repost collant : il ne doit donc jamais etre le titre d'un autre
/// embed du bot, sous peine de voir les resultats de tirage disparaitre.
pub const PANEL_TITLE: &str = "\u{1f300} La Roue du Destin";

/// Duree du suspense entre l'annonce du tirage et son resultat.
const SPIN_ANIMATION_MS: u64 = 4000;

pub fn register() -> CreateCommand {
    CreateCommand::new("roue-panel")
        .description("Deployer le panneau de la Roue du Destin dans ce salon (admin)")
        .default_member_permissions(Permissions::MANAGE_GUILD)
}

pub fn handles_component(custom_id: &str) -> bool {
    custom_id == PANEL_SPIN_ID
}

/// Le message du panneau : embed + bouton. Partage entre le deploiement
/// initial et le repost collant, pour que les deux ne puissent pas diverger.
pub fn build_panel_message() -> CreateMessage {
    let embed = serenity::all::CreateEmbed::new()
        .title(PANEL_TITLE)
        .description(
            "\u{1fa99} **Une fois par jour**, tente ta chance.\n\n\
             Le destin peut te rendre **riche** (jackpot, licorne) ou **ridicule**\n\
             (PQ, ruine, bombe). Un seul tirage par jour, alors choisis bien... ou pas.\n\n\
             *Le resultat est annonce publiquement. Tout le salon en parle.*",
        )
        .color(0xf1c40f);

    let button = CreateButton::new(PANEL_SPIN_ID)
        .label("Tirer la Roue")
        .emoji(serenity::model::channel::ReactionType::Unicode(
            "\u{1f300}".into(),
        ))
        .style(ButtonStyle::Success);

    CreateMessage::new()
        .embed(embed)
        .components(vec![CreateActionRow::Buttons(vec![button])])
}

/// « Message collant » : republie le panneau EN BAS du salon et supprime les
/// anciens. Sans ca, chaque resultat de tirage repousse le panneau vers le
/// haut et il devient introuvable au bout de trois messages.
///
/// Best-effort de bout en bout : un scan ou une suppression qui echoue est
/// journalise puis ignore. Le pire cas est un panneau en double, pas un salon
/// sans panneau.
pub async fn repost_panel(ctx: &Context, channel_id: ChannelId) {
    let bot_id = ctx.cache.current_user().id;

    // Reperer les anciens AVANT de poster le nouveau : dans l'autre ordre on
    // supprimerait celui qu'on vient d'envoyer.
    let old_panels: Vec<MessageId> = match channel_id
        .messages(&ctx.http, GetMessages::new().limit(50))
        .await
    {
        Ok(messages) => messages
            .into_iter()
            .filter(|m| {
                m.author.id == bot_id
                    && m.embeds
                        .iter()
                        .any(|e| e.title.as_deref() == Some(PANEL_TITLE))
            })
            .map(|m| m.id)
            .collect(),
        Err(e) => {
            warn!(error = %e, "scan des anciens panneaux Roue impossible");
            Vec::new()
        }
    };

    if let Err(e) = channel_id
        .send_message(&ctx.http, build_panel_message())
        .await
    {
        warn!(error = %e, "repost du panneau Roue impossible");
        return;
    }

    for id in old_panels {
        if let Err(e) = channel_id.delete_message(&ctx.http, id).await {
            warn!(error = %e, message_id = %id, "suppression d'un ancien panneau Roue impossible");
        }
    }
}

/// `/roue-panel` — pose le panneau dans le salon courant.
pub async fn handle_command(ctx: &Context, cmd: &CommandInteraction) {
    // Fail-closed : sans permission LISIBLE dans l'interaction, on refuse.
    // Discord filtre deja la commande via `default_member_permissions`, mais
    // ce filtre est cote client et se contourne.
    let autorise = cmd
        .member
        .as_ref()
        .and_then(|m| m.permissions)
        .is_some_and(|p| {
            p.contains(Permissions::MANAGE_GUILD) || p.contains(Permissions::ADMINISTRATOR)
        });
    if !autorise {
        reply_ephemeral(ctx, cmd, "Permission « Gerer le serveur » requise.").await;
        return;
    }

    if let Err(e) = cmd
        .channel_id
        .send_message(&ctx.http, build_panel_message())
        .await
    {
        warn!(error = %e, "envoi du panneau Roue impossible");
        reply_ephemeral(ctx, cmd, "Deploiement du panneau impossible.").await;
        return;
    }

    reply_ephemeral(ctx, cmd, "Panneau de la Roue deploye !").await;
    info!(channel = %cmd.channel_id, "panneau Roue deploye");
}

/// Clic sur « Tirer la Roue ».
///
/// L'ordre compte : on appelle l'API AVANT d'annoncer quoi que ce soit. Poster
/// « la roue tourne » puis se prendre un refus laisserait un message mensonger
/// dans le salon. Le refus, lui, reste prive : personne n'a besoin de savoir
/// que quelqu'un a reclique.
pub async fn handle_spin(api: &ApiClient, ctx: &Context, component: &ComponentInteraction) {
    let Some(guild_id) = component.guild_id else {
        return;
    };
    let username = component.user.display_name().to_string();

    if let Err(e) = component
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Defer(
                CreateInteractionResponseMessage::new().ephemeral(true),
            ),
        )
        .await
    {
        warn!(error = %e, "defer du tirage impossible");
        return;
    }

    let response = match api
        .spin_wheel(
            &guild_id.to_string(),
            &component.user.id.to_string(),
            &username,
        )
        .await
    {
        Ok(r) => r,
        Err(message) => {
            let edit = serenity::builder::EditInteractionResponse::new()
                .embed(embeds::build_error_embed(&message));
            let _ = component.edit_response(&ctx.http, edit).await;
            return;
        }
    };

    // Le suspense est public : c'est ce qui fait lever les tetes dans le
    // salon. Le resultat viendra remplacer ce message.
    let mut annonce = match component
        .channel_id
        .send_message(
            &ctx.http,
            CreateMessage::new().embed(embeds::build_spinning_embed(&username)),
        )
        .await
    {
        Ok(m) => m,
        Err(e) => {
            warn!(error = %e, "annonce du tirage impossible");
            return;
        }
    };

    tokio::time::sleep(Duration::from_millis(SPIN_ANIMATION_MS)).await;

    if let Err(e) = annonce
        .edit(
            &ctx.http,
            EditMessage::new().embed(embeds::build_result_embed(&response, &username)),
        )
        .await
    {
        warn!(error = %e, "affichage du resultat impossible");
    }

    // Le resultat vient de repousser le panneau vers le haut : on le remet en
    // bas.
    repost_panel(ctx, component.channel_id).await;

    let edit = serenity::builder::EditInteractionResponse::new()
        .content(format!("\u{1f300} Ton tirage : {}", response.case_label));
    let _ = component.edit_response(&ctx.http, edit).await;
}

async fn reply_ephemeral(ctx: &Context, cmd: &CommandInteraction, message: &str) {
    let response = CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::new()
            .content(message)
            .ephemeral(true),
    );
    let _ = cmd.create_response(&ctx.http, response).await;
}
