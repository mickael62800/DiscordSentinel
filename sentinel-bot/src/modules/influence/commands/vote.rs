//! Commande `/vote` — ouvre un vote binaire au sein d'une organisation.
//!
//! Poste un message PUBLIC avec un embed et des boutons Pour/Contre/Abstention,
//! plus un bouton Clôturer reserve a l'auteur. Les bulletins sont enregistres
//! cote API ; le message est mis a jour a chaque vote (cf. `on_component`).

use serenity::all::{
    ButtonStyle, CommandInteraction, CommandOptionType, Context, CreateActionRow, CreateButton,
    CreateCommand, CreateCommandOption, CreateEmbed, CreateEmbedFooter, CreateInteractionResponse,
    CreateInteractionResponseMessage, ReactionType,
};

use crate::modules::influence::api_client::{self, MotionState};
use crate::shared::discord_helpers::{option_str, reply_ephemeral, require_guild_id};
use crate::shared::heartbeat::ApiClientKey;

/// Prefixe des custom_id des boutons de vote : `inf_vote:<motion_id>:<action>`.
pub const PREFIX: &str = "inf_vote:";

pub fn register() -> CreateCommand {
    CreateCommand::new("vote")
        .description("Ouvre un vote au sein de ton organisation")
        .add_option(
            CreateCommandOption::new(CommandOptionType::String, "org", "Organisation concernee")
                .required(true),
        )
        .add_option(
            CreateCommandOption::new(CommandOptionType::String, "sujet", "Sujet mis au vote")
                .required(true),
        )
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    let Some(guild_id) = require_guild_id(ctx, command).await else {
        return;
    };
    let org = option_str(&command.data.options, "org").unwrap_or("");
    let sujet = option_str(&command.data.options, "sujet").unwrap_or("");

    let api = {
        let data = ctx.data.read().await;
        match data.get::<ApiClientKey>() {
            Some(a) => a.clone(),
            None => return,
        }
    };

    let state = match api_client::create_motion(
        &api,
        &guild_id,
        org,
        &command.user.id.to_string(),
        &command.user.name,
        sujet,
    )
    .await
    {
        Ok(s) => s,
        Err(e) => {
            reply_ephemeral(ctx, command, &format!("Impossible d'ouvrir le vote : {e}")).await;
            return;
        }
    };

    // Message PUBLIC (les membres doivent pouvoir cliquer).
    if let Err(e) = command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .embed(build_embed(&state))
                    .components(vote_rows(&state.motion_id, false)),
            ),
        )
        .await
    {
        tracing::warn!(error = %e, "Echec post message de vote");
    }
}

/// Construit l'embed d'une motion (partage entre creation et mise a jour).
pub fn build_embed(s: &MotionState) -> CreateEmbed {
    let total = s.pour + s.contre + s.abstention;
    let closed = s.status != "ouverte";

    let color = match s.status.as_str() {
        "adoptee" => 0x2ECC71,
        "rejetee" => 0xE74C3C,
        _ => 0x8E44AD,
    };

    let mut embed = CreateEmbed::new()
        .title(format!("🗳️ {}", s.title))
        .color(color)
        .field("Organisation", s.org_name.clone(), true)
        .field("Statut", s.status_label.clone(), true)
        .field("✅ Pour", s.pour.to_string(), true)
        .field("❌ Contre", s.contre.to_string(), true)
        .field("⚪ Abstention", s.abstention.to_string(), true)
        .field("Votants", total.to_string(), true)
        .field(
            "⚖️ Poids du vote (influence + notoriété)",
            format!("Pour **{}** / Contre **{}**", s.pour_weight, s.contre_weight),
            false,
        );

    embed = embed.footer(CreateEmbedFooter::new(if closed {
        "Vote clos.".to_string()
    } else {
        "Seuls les membres de l'organisation peuvent voter.".to_string()
    }));
    embed
}

/// Boutons de vote. `closed = true` renvoie une liste vide (retire les boutons).
pub fn vote_rows(motion_id: &str, closed: bool) -> Vec<CreateActionRow> {
    if closed {
        return vec![];
    }
    vec![CreateActionRow::Buttons(vec![
        CreateButton::new(format!("{PREFIX}{motion_id}:pour"))
            .label("Pour")
            .emoji(ReactionType::Unicode("✅".into()))
            .style(ButtonStyle::Success),
        CreateButton::new(format!("{PREFIX}{motion_id}:contre"))
            .label("Contre")
            .emoji(ReactionType::Unicode("❌".into()))
            .style(ButtonStyle::Danger),
        CreateButton::new(format!("{PREFIX}{motion_id}:abstention"))
            .label("Abstention")
            .emoji(ReactionType::Unicode("⚪".into()))
            .style(ButtonStyle::Secondary),
        CreateButton::new(format!("{PREFIX}{motion_id}:close"))
            .label("Clôturer")
            .emoji(ReactionType::Unicode("🔒".into()))
            .style(ButtonStyle::Primary),
    ])]
}
