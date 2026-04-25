//! Commande slash /slot-setup — deploie le panel persistant Tirer/Daily.

use serenity::all::{
    ButtonStyle, CommandInteraction, Context, CreateActionRow, CreateButton, CreateCommand,
    CreateEmbed, CreateEmbedFooter, CreateInteractionResponse, CreateInteractionResponseMessage,
    CreateMessage,
};
use tracing::{info, warn};

/// Bouton unique du panel global : ouvre (ou retrouve) le salon perso du user.
pub const PANEL_OPEN_ID: &str = "slot_panel_open";

/// Boutons dans le salon perso : spin payant, daily, fermer le salon.
pub const CHANNEL_SPIN_ID: &str = "slot_ch_spin";
pub const CHANNEL_DAILY_ID: &str = "slot_ch_daily";
pub const CHANNEL_CLOSE_ID: &str = "slot_ch_close";

pub fn register() -> CreateCommand {
    CreateCommand::new("slot-setup")
        .description("Deployer le panel Machine a sous dans ce salon (admin)")
        .default_member_permissions(serenity::all::Permissions::MANAGE_GUILD)
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    let channel_id = command.channel_id;

    let embed = CreateEmbed::new()
        .title("\u{1f3b0} Machine a sous")
        .description(
            "Tente ta chance ! Clique sur **Ouvrir ma machine** pour creer ton salon prive.\n\n\
             Tu pourras y faire des spins en suspense, voir tes gains et reclamer\n\
             ton Daily Bonus quotidien.\n\n\
             3 symboles identiques = jackpot (multiplie ta mise).\n\
             2 identiques = mise remboursee.\n\
             3x \u{0037}\u{fe0f}\u{20e3} = pool jackpot progressif !"
        )
        .color(0xf1c40f)
        .footer(CreateEmbedFooter::new("Slot Machine | Sentinel"));

    let open_btn = CreateButton::new(PANEL_OPEN_ID)
        .label("Ouvrir ma machine")
        .emoji(serenity::model::channel::ReactionType::Unicode("\u{1f3b0}".into()))
        .style(ButtonStyle::Success);

    let row = CreateActionRow::Buttons(vec![open_btn]);

    if let Err(e) = channel_id
        .send_message(&ctx.http, CreateMessage::new().embed(embed).components(vec![row]))
        .await
    {
        warn!(error = %e, "Echec envoi panel slot");
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
            .content("Panel Machine a sous deploye !")
            .ephemeral(true),
    );
    let _ = command.create_response(&ctx.http, resp).await;

    info!(channel = %channel_id, "Panel slot deploye");
}
