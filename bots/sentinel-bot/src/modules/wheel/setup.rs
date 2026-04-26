//! Commande slash /wheel-setup — pose le panel persistant.

use serenity::all::{
    ButtonStyle, CommandInteraction, Context, CreateActionRow, CreateButton, CreateCommand,
    CreateEmbed, CreateEmbedFooter, CreateInteractionResponse, CreateInteractionResponseMessage,
    CreateMessage,
};
use tracing::{info, warn};

pub const PANEL_SPIN_ID: &str = "wheel_panel_spin";

pub fn register() -> CreateCommand {
    CreateCommand::new("wheel-setup")
        .description("Deployer le panel Roue du Destin dans ce salon (admin)")
        .default_member_permissions(serenity::all::Permissions::MANAGE_GUILD)
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    let channel_id = command.channel_id;

    let embed = CreateEmbed::new()
        .title("\u{1f300} La Roue du Destin")
        .description(
            "🪙 **Une fois par jour**, tente ta chance.\n\n\
             Le destin peut te rendre **riche** (jackpot, licorne) ou **ridicule**\n\
             (PQ, ruine, bombe). Un seul spin par jour, alors choisis bien... ou pas.\n\n\
             *Le resultat est annonce publiquement. Tout le serveur en parle.*"
        )
        .color(0xf1c40f)
        .footer(CreateEmbedFooter::new("Roue du Destin | Sentinel"));

    let btn = CreateButton::new(PANEL_SPIN_ID)
        .label("Tirer la Roue")
        .emoji(serenity::model::channel::ReactionType::Unicode("\u{1f300}".into()))
        .style(ButtonStyle::Success);

    let row = CreateActionRow::Buttons(vec![btn]);

    if let Err(e) = channel_id
        .send_message(&ctx.http, CreateMessage::new().embed(embed).components(vec![row]))
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
