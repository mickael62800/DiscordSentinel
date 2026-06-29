//! Commande slash /blackjack-setup — deploie le panneau de jeu.

use serenity::all::{
    ButtonStyle, CommandInteraction, Context, CreateActionRow, CreateButton, CreateCommand,
    CreateEmbed, CreateEmbedFooter, CreateInteractionResponse, CreateInteractionResponseMessage,
    CreateMessage,
};
use tracing::{info, warn};

pub const PANEL_BUTTON_ID: &str = "bj_panel_play";

pub fn register() -> CreateCommand {
    CreateCommand::new("blackjack-setup")
        .description("Deployer le panneau de Blackjack dans ce salon (admin)")
        .default_member_permissions(serenity::all::Permissions::MANAGE_GUILD)
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    let channel_id = command.channel_id;

    let embed = CreateEmbed::new()
        .title("\u{1f0cf} Blackjack — Casino Sentinel")
        .description(
            "**Tente ta chance au Blackjack !**\n\n\
             \u{1f3b0} Clique sur le bouton ci-dessous pour ouvrir une table privee.\n\
             \u{1f4b0} Tu choisis ta mise, puis tu joues avec les boutons.\n\
             \u{23f1}\u{fe0f} La table se ferme automatiquement apres 30 minutes d'inactivite.\n\n\
             *Bonne chance !*",
        )
        .color(0xF1C40F)
        .footer(CreateEmbedFooter::new(
            crate::shared::branding::BLACKJACK_TAGLINE,
        ));

    let button = CreateButton::new(PANEL_BUTTON_ID)
        .label("Jouer au Blackjack")
        .emoji(serenity::model::channel::ReactionType::Unicode(
            "\u{1f0cf}".into(),
        ))
        .style(ButtonStyle::Success);

    let row = CreateActionRow::Buttons(vec![button]);

    if let Err(e) = channel_id
        .send_message(
            &ctx.http,
            CreateMessage::new().embed(embed).components(vec![row]),
        )
        .await
    {
        warn!(error = %e, "Echec envoi panel blackjack");
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
            .content("Panel de Blackjack deploye !")
            .ephemeral(true),
    );
    let _ = command.create_response(&ctx.http, resp).await;

    info!(channel = %channel_id, "Panel blackjack deploye");
}
