use serenity::all::{
    ButtonStyle, CommandInteraction, ComponentInteraction, Context, CreateActionRow, CreateButton,
    CreateCommand, CreateEmbed, CreateEmbedFooter, CreateInteractionResponse,
    CreateInteractionResponseMessage,
};

use crate::shared::discord_helpers::{reply_ephemeral, require_guild_id};

use crate::modules::coude::load_guild_config;
use crate::modules::coude::GameApiKey;

/// custom_id du bouton "Mettre a jour" du panneau de cagnotte. Comme le
/// leaderboard : message unique persistant, le bouton edite ce meme message.
pub const REFRESH_ID: &str = "coude_cagnotte_refresh";

pub fn register() -> CreateCommand {
    CreateCommand::new("cagnotte")
        .description("Affiche la caisse communautaire (avec bouton Mettre a jour)")
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    let Some(guild_id) = require_guild_id(ctx, command).await else {
        return;
    };

    let config = load_guild_config(ctx, &guild_id).await;
    if !config.enabled() {
        reply_ephemeral(
            ctx,
            command,
            "Le jeu Coup de Coude est desactive sur ce serveur.",
        )
        .await;
        return;
    }

    let embed = build_cagnotte_embed(ctx, &guild_id).await;

    if let Err(e) = command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .embed(embed)
                    .components(vec![refresh_row()]),
            ),
        )
        .await
    {
        tracing::warn!(error = %e, "Echec response cagnotte");
    }
}

/// Handler du bouton "Mettre a jour" : reconstruit la cagnotte et edite le
/// message en place (UpdateMessage).
pub async fn handle_refresh(ctx: &Context, component: &ComponentInteraction) {
    let guild_id = match component.guild_id {
        Some(g) => g.to_string(),
        None => return,
    };

    let embed = build_cagnotte_embed(ctx, &guild_id).await;

    if let Err(e) = component
        .create_response(
            &ctx.http,
            CreateInteractionResponse::UpdateMessage(
                CreateInteractionResponseMessage::new()
                    .embed(embed)
                    .components(vec![refresh_row()]),
            ),
        )
        .await
    {
        tracing::warn!(error = %e, "Echec update panneau cagnotte");
    }
}

async fn build_cagnotte_embed(ctx: &Context, guild_id: &str) -> CreateEmbed {
    let data = ctx.data.read().await;
    let api = data.get::<GameApiKey>().unwrap();

    match api.get_cashbox(guild_id).await {
        Ok(c) => CreateEmbed::new()
            .title("\u{1f3e6} Caisse communautaire")
            .color(0x2ECC71)
            .field(
                "\u{1fa99} Solde actuel",
                format!("**{}** coins", c.balance),
                false,
            )
            .field(
                "\u{1f4e5} Total collecte",
                format!("{} coins", c.total_collected),
                true,
            )
            .field(
                "\u{1f4e4} Total redistribue",
                format!("{} coins", c.total_redistributed),
                true,
            )
            .footer(CreateEmbedFooter::new(format!(
                "{} \u{2014} clique sur Mettre a jour pour rafraichir",
                crate::shared::branding::COUDE_TAGLINE_SHORT
            )))
            .timestamp(serenity::model::Timestamp::now()),
        Err(e) => {
            tracing::warn!(error = %e, "Echec get_cashbox");
            CreateEmbed::new()
                .title("\u{1f3e6} Caisse communautaire")
                .description("Caisse momentanement indisponible. Reessaie dans un instant.")
                .color(0xE74C3C)
                .timestamp(serenity::model::Timestamp::now())
        }
    }
}

/// Row contenant le bouton "Mettre a jour".
fn refresh_row() -> CreateActionRow {
    CreateActionRow::Buttons(vec![CreateButton::new(REFRESH_ID)
        .label("Mettre a jour")
        .emoji(serenity::model::channel::ReactionType::Unicode(
            "\u{1f504}".into(),
        ))
        .style(ButtonStyle::Primary)])
}
