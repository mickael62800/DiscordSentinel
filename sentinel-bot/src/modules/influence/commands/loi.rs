//! Commande `/loi propose` — depose une loi soumise au vote de tous les
//! citoyens (Phase 3). Le vote se cloture automatiquement a l'echeance (worker).

use serenity::all::{
    ButtonStyle, CommandInteraction, CommandOptionType, Context, CreateActionRow, CreateButton,
    CreateCommand, CreateCommandOption, CreateEmbed, CreateEmbedFooter, CreateInteractionResponse,
    CreateInteractionResponseMessage, ReactionType,
};

use crate::modules::influence::api_client::{self, LawState};
use crate::shared::discord_helpers::{option_str, require_guild_id};
use crate::shared::heartbeat::ApiClientKey;

/// Prefixe des boutons de vote de loi : `inf_law:<law_id>:<choix>`.
pub const PREFIX: &str = "inf_law:";

pub fn register() -> CreateCommand {
    CreateCommand::new("loi")
        .description("Propose une loi au vote des citoyens")
        .add_option(
            CreateCommandOption::new(CommandOptionType::SubCommand, "propose", "Depose une loi")
                .add_sub_option(
                    CreateCommandOption::new(CommandOptionType::String, "titre", "Titre de la loi")
                        .required(true),
                )
                .add_sub_option(
                    CreateCommandOption::new(CommandOptionType::String, "texte", "Contenu de la loi")
                        .required(true),
                ),
        )
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    let Some(guild_id) = require_guild_id(ctx, command).await else {
        return;
    };
    // Sous-commande propose.
    let opts = match command.data.options.first().map(|s| &s.value) {
        Some(serenity::all::CommandDataOptionValue::SubCommand(o)) => o.clone(),
        _ => return,
    };
    let titre = option_str(&opts, "titre").unwrap_or("");
    let texte = option_str(&opts, "texte").unwrap_or("");

    let api = {
        let data = ctx.data.read().await;
        match data.get::<ApiClientKey>() {
            Some(a) => a.clone(),
            None => return,
        }
    };

    let state = match api_client::propose_law(
        &api,
        &guild_id,
        &command.user.id.to_string(),
        &command.user.name,
        titre,
        texte,
    )
    .await
    {
        Ok(s) => s,
        Err(e) => {
            let _ = command
                .create_response(
                    &ctx.http,
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new()
                            .content(format!("Impossible de deposer la loi : {e}"))
                            .ephemeral(true),
                    ),
                )
                .await;
            return;
        }
    };

    // Message PUBLIC avec boutons de vote.
    if let Err(e) = command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .embed(build_embed(&state))
                    .components(vote_rows(&state.law_id, false)),
            ),
        )
        .await
    {
        tracing::warn!(error = %e, "Echec post message de loi");
        return;
    }

    // Recupere le message poste pour memoriser channel/message (edition a la
    // cloture par le worker).
    if let Ok(msg) = command.get_response(&ctx.http).await {
        let _ = api_client::set_law_message(
            &api,
            &guild_id,
            &state.law_id,
            &msg.channel_id.to_string(),
            &msg.id.to_string(),
        )
        .await;
    }
}

/// Embed d'une loi (partage creation / mise a jour / cloture).
pub fn build_embed(s: &LawState) -> CreateEmbed {
    let total = s.pour + s.contre + s.abstention;
    let color = match s.status.as_str() {
        "adoptee" => 0x2ECC71,
        "rejetee" => 0xE74C3C,
        _ => 0x3498DB,
    };
    let body: String = s.body.chars().take(1000).collect();
    CreateEmbed::new()
        .title(format!("📜 Loi : {}", s.title))
        .color(color)
        .description(body)
        .field("Statut", s.status_label.clone(), true)
        .field("Votants", total.to_string(), true)
        .field("✅ Pour", s.pour.to_string(), true)
        .field("❌ Contre", s.contre.to_string(), true)
        .field("⚪ Abstention", s.abstention.to_string(), true)
        .footer(CreateEmbedFooter::new(if s.status == "vote" {
            "Tous les citoyens peuvent voter. Cloture automatique a l'echeance."
        } else {
            "Vote clos."
        }))
}

/// Boutons de vote. Liste vide si la loi n'est plus en vote.
pub fn vote_rows(law_id: &str, closed: bool) -> Vec<CreateActionRow> {
    if closed {
        return vec![];
    }
    vec![CreateActionRow::Buttons(vec![
        CreateButton::new(format!("{PREFIX}{law_id}:pour"))
            .label("Pour")
            .emoji(ReactionType::Unicode("✅".into()))
            .style(ButtonStyle::Success),
        CreateButton::new(format!("{PREFIX}{law_id}:contre"))
            .label("Contre")
            .emoji(ReactionType::Unicode("❌".into()))
            .style(ButtonStyle::Danger),
        CreateButton::new(format!("{PREFIX}{law_id}:abstention"))
            .label("Abstention")
            .emoji(ReactionType::Unicode("⚪".into()))
            .style(ButtonStyle::Secondary),
    ])]
}
