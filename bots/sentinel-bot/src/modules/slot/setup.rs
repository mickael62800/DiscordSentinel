//! Commande slash /slot-setup — deploie le panel persistant Tirer/Daily.

use serenity::all::{
    ButtonStyle, CommandInteraction, Context, CreateActionRow, CreateButton, CreateCommand,
    CreateEmbed, CreateEmbedFooter, CreateInteractionResponse, CreateInteractionResponseMessage,
    CreateMessage,
};
use tracing::{info, warn};

pub const PANEL_SPIN_ID: &str = "slot_panel_spin";
pub const PANEL_DAILY_ID: &str = "slot_panel_daily";

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
            "Tente ta chance ! Clique sur **Tirer** pour faire un spin a la mise par defaut.\n\n\
             3 symboles identiques = jackpot (multiplie ta mise).\n\
             2 identiques = mise remboursee.\n\
             3x \u{0037}\u{fe0f}\u{20e3} = pool jackpot progressif !\n\n\
             Tu peux aussi reclamer ton **Daily Bonus** (1 spin gratuit / jour)."
        )
        .color(0xf1c40f)
        .footer(CreateEmbedFooter::new("Slot Machine | Sentinel"));

    let spin_btn = CreateButton::new(PANEL_SPIN_ID)
        .label("Tirer")
        .emoji(serenity::model::channel::ReactionType::Unicode("\u{1f3b0}".into()))
        .style(ButtonStyle::Success);

    let daily_btn = CreateButton::new(PANEL_DAILY_ID)
        .label("Daily Bonus")
        .emoji(serenity::model::channel::ReactionType::Unicode("\u{1f381}".into()))
        .style(ButtonStyle::Primary);

    let row = CreateActionRow::Buttons(vec![spin_btn, daily_btn]);

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
