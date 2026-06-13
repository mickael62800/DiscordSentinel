//! Commande /tama-setup : deploie le panneau public Ouvrir mon compagnon.

use serenity::all::{
    ButtonStyle, CommandInteraction, Context, CreateActionRow, CreateButton, CreateCommand,
    CreateEmbed, CreateEmbedFooter, CreateInteractionResponse, CreateInteractionResponseMessage,
    CreateMessage,
};
use tracing::{info, warn};

pub const PANEL_OPEN_ID: &str = "tama_open";

pub fn register() -> CreateCommand {
    CreateCommand::new("tama-setup")
        .description("Deployer le panneau Tamagotchi dans ce salon (admin)")
        .default_member_permissions(serenity::all::Permissions::MANAGE_GUILD)
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    let embed = CreateEmbed::new()
        .title("🐾 Ton compagnon")
        .description(
            "Adopte et eleve ton compagnon ! Nourris-le, joue avec lui, fais-le dormir...\n\n\
             Clique sur **Ouvrir mon compagnon** pour creer ton salon prive.\n\
             ⚠️ Si tu le negliges, il peut tomber malade et mourir.",
        )
        .color(0x9b59b6)
        .footer(CreateEmbedFooter::new("Tamagotchi"));

    let btn = CreateButton::new(PANEL_OPEN_ID)
        .label("Ouvrir mon compagnon")
        .emoji(serenity::model::channel::ReactionType::Unicode("🐾".into()))
        .style(ButtonStyle::Success);
    let row = CreateActionRow::Buttons(vec![btn]);

    if let Err(e) = command
        .channel_id
        .send_message(&ctx.http, CreateMessage::new().embed(embed).components(vec![row]))
        .await
    {
        warn!(error = %e, "Echec envoi panel tamagotchi");
        let _ = command
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content("Erreur lors du deploiement.")
                        .ephemeral(true),
                ),
            )
            .await;
        return;
    }

    let _ = command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content("Panneau Tamagotchi deploye !")
                    .ephemeral(true),
            ),
        )
        .await;
    info!(channel = %command.channel_id, "Panel tamagotchi deploye");
}
