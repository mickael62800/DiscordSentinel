//! Commande slash /wheel-setup — pose le panel persistant.

use serenity::all::{
    ButtonStyle, ChannelId, CommandInteraction, Context, CreateActionRow, CreateButton,
    CreateCommand, CreateEmbed, CreateEmbedFooter, CreateInteractionResponse,
    CreateInteractionResponseMessage, CreateMessage, GetMessages, MessageId,
};
use tracing::{info, warn};

pub const PANEL_SPIN_ID: &str = "wheel_panel_spin";

/// Titre EXACT du panneau, utilise pour reperer les anciens panneaux a
/// supprimer lors du repost collant. Ne doit pas etre un prefixe d'un autre
/// embed du module (les embeds spin/resultat ont des titres differents).
pub const PANEL_TITLE: &str = "\u{1f300} La Roue du Destin";

pub fn register() -> CreateCommand {
    CreateCommand::new("wheel-setup")
        .description("Deployer le panel Roue du Destin dans ce salon (admin)")
        .default_member_permissions(serenity::all::Permissions::MANAGE_GUILD)
}

/// Construit le message du panneau (embed + bouton). Partage entre le deploy
/// initial (`/wheel-setup`) et le repost collant apres chaque spin.
pub fn build_panel_message() -> CreateMessage {
    let embed = CreateEmbed::new()
        .title(PANEL_TITLE)
        .description(
            "🪙 **Une fois par jour**, tente ta chance.\n\n\
             Le destin peut te rendre **riche** (jackpot, licorne) ou **ridicule**\n\
             (PQ, ruine, bombe). Un seul spin par jour, alors choisis bien... ou pas.\n\n\
             *Le resultat est annonce publiquement. Tout le serveur en parle.*",
        )
        .color(0xf1c40f)
        .footer(CreateEmbedFooter::new(
            crate::shared::branding::WHEEL_TAGLINE,
        ));

    let btn = CreateButton::new(PANEL_SPIN_ID)
        .label("Tirer la Roue")
        .emoji(serenity::model::channel::ReactionType::Unicode(
            "\u{1f300}".into(),
        ))
        .style(ButtonStyle::Success);

    CreateMessage::new()
        .embed(embed)
        .components(vec![CreateActionRow::Buttons(vec![btn])])
}

/// "Message collant" : republie le panneau EN BAS du salon et supprime les
/// anciens panneaux du bot, pour que la Roue reste toujours le dernier
/// message (sinon chaque resultat de spin la repousse vers le haut).
///
/// Best-effort : si le scan ou une suppression echoue, on log et on continue.
/// Le nouveau panneau est toujours poste tant que l'envoi reussit.
pub async fn repost_panel(ctx: &Context, channel_id: ChannelId) {
    let bot_id = ctx.cache.current_user().id;

    // 1. Reperer les anciens panneaux AVANT de poster le nouveau (sinon on
    //    risquerait de supprimer celui qu'on vient de poster).
    let old_panels: Vec<MessageId> = match channel_id
        .messages(&ctx.http, GetMessages::new().limit(50))
        .await
    {
        Ok(msgs) => msgs
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
            warn!(error = %e, "Echec scan anciens panneaux wheel");
            Vec::new()
        }
    };

    // 2. Poster le nouveau panneau en bas.
    if let Err(e) = channel_id
        .send_message(&ctx.http, build_panel_message())
        .await
    {
        warn!(error = %e, "Echec repost panneau wheel");
        return;
    }

    // 3. Supprimer les anciens panneaux (best-effort, ignore si deja absent).
    for mid in old_panels {
        if let Err(e) = channel_id.delete_message(&ctx.http, mid).await {
            warn!(error = %e, message_id = %mid, "Echec suppression ancien panneau wheel");
        }
    }
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    let channel_id = command.channel_id;

    if let Err(e) = channel_id
        .send_message(&ctx.http, build_panel_message())
        .await
    {
        warn!(error = %e, "Echec envoi panel wheel");
        let resp = CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new()
                .content("Erreur lors du deploiement du panel.")
                .ephemeral(true),
        );
        let _ = command.create_response(&ctx.http, resp).await;
        return;
    }

    let resp = CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::new()
            .content("Panel Roue du Destin deploye !")
            .ephemeral(true),
    );
    let _ = command.create_response(&ctx.http, resp).await;
    info!(channel = %channel_id, "Panel wheel deploye");
}
