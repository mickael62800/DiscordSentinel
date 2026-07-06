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
                )
                .add_sub_option(
                    CreateCommandOption::new(
                        CommandOptionType::String,
                        "parametre",
                        "Réglage modifié si la loi est adoptée (optionnel)",
                    )
                    .add_string_choice("Coût d'une enquête", "cout_enquete")
                    .add_string_choice("Coût de création d'organisation", "cout_creation_org")
                    .add_string_choice("Coût du rôle d'organisation", "cout_role_org")
                    .add_string_choice("Réputation perdue par scandale", "perte_reputation_scandale")
                    .add_string_choice("Proba de réussite d'enquête (%)", "proba_enquete")
                    .add_string_choice("Durée de débat d'une loi (h)", "duree_debat_loi")
                    .required(false),
                )
                .add_sub_option(
                    CreateCommandOption::new(
                        CommandOptionType::Integer,
                        "valeur",
                        "Nouvelle valeur du réglage (si un paramètre est choisi)",
                    )
                    .required(false),
                ),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "financer",
                "Finance une loi depuis la trésorerie de ton organisation (lobbying)",
            )
            .add_sub_option(
                CreateCommandOption::new(CommandOptionType::String, "loi", "Identifiant de la loi")
                    .required(true),
            )
            .add_sub_option(
                CreateCommandOption::new(CommandOptionType::String, "org", "Ton organisation")
                    .required(true),
            )
            .add_sub_option(
                CreateCommandOption::new(CommandOptionType::Integer, "montant", "Montant à dépenser")
                    .required(true),
            )
            .add_sub_option(
                CreateCommandOption::new(CommandOptionType::String, "camp", "Camp à soutenir")
                    .required(true)
                    .add_string_choice("Pour", "pour")
                    .add_string_choice("Contre", "contre"),
            ),
        )
        .add_option(CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "liste",
            "Liste les lois actuellement en vote (avec leur ID)",
        ))
}

async fn handle_list(ctx: &Context, command: &CommandInteraction, guild_id: &str) {
    let api = {
        let data = ctx.data.read().await;
        match data.get::<ApiClientKey>() {
            Some(a) => a.clone(),
            None => return,
        }
    };
    let embed = match api_client::list_laws(&api, guild_id).await {
        Ok(laws) if laws.is_empty() => CreateEmbed::new()
            .title("📜 Lois en vote")
            .color(0x3498DB)
            .description("*Aucune loi en vote actuellement.*"),
        Ok(laws) => {
            let desc = laws
                .iter()
                .map(|l| {
                    let funding = if l.funding_pour > 0 || l.funding_contre > 0 {
                        format!(" · 🏛️ {}/{}", l.funding_pour, l.funding_contre)
                    } else {
                        String::new()
                    };
                    format!(
                        "**{}**\n`{}` · ✅ {} / ❌ {}{}",
                        l.title, l.law_id, l.pour_weight, l.contre_weight, funding
                    )
                })
                .collect::<Vec<_>>()
                .join("\n\n");
            CreateEmbed::new()
                .title("📜 Lois en vote")
                .color(0x3498DB)
                .description(desc)
                .footer(CreateEmbedFooter::new(
                    "Utilise l'ID pour /loi financer (lobbying) ou pour voter.",
                ))
        }
        Err(e) => {
            let _ = command
                .create_response(
                    &ctx.http,
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new()
                            .content(format!("Erreur : {e}"))
                            .ephemeral(true),
                    ),
                )
                .await;
            return;
        }
    };
    let _ = command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new().embed(embed).ephemeral(true),
            ),
        )
        .await;
}

async fn handle_fund(
    ctx: &Context,
    command: &CommandInteraction,
    guild_id: &str,
    opts: &[serenity::all::CommandDataOption],
) {
    let law_id = option_str(opts, "loi").unwrap_or("");
    let org = option_str(opts, "org").unwrap_or("");
    let camp_pour = option_str(opts, "camp").unwrap_or("pour") != "contre";
    let montant = opts
        .iter()
        .find(|o| o.name == "montant")
        .and_then(|o| match &o.value {
            serenity::all::CommandDataOptionValue::Integer(i) => Some(*i),
            _ => None,
        })
        .unwrap_or(0);

    let api = {
        let data = ctx.data.read().await;
        match data.get::<ApiClientKey>() {
            Some(a) => a.clone(),
            None => return,
        }
    };
    let user_id = command.user.id.to_string();
    match api_client::fund_law(
        &api,
        guild_id,
        org,
        law_id,
        &user_id,
        &command.user.name,
        montant,
        camp_pour,
    )
    .await
    {
        Ok(r) => {
            let camp = if r.camp_pour { "Pour" } else { "Contre" };
            let msg = format!(
                "🏛️ **{}** a financé **{}** pour le camp **{}** de « {} ».\nFinancement total : Pour **{}** / Contre **{}** · Trésorerie restante : **{}** 💰",
                org, r.amount, camp, r.law_title, r.funding_pour, r.funding_contre, r.treasury_left
            );
            let _ = command
                .create_response(
                    &ctx.http,
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new().content(msg).ephemeral(true),
                    ),
                )
                .await;
        }
        Err(e) => {
            let _ = command
                .create_response(
                    &ctx.http,
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new()
                            .content(format!("Impossible de financer : {e}"))
                            .ephemeral(true),
                    ),
                )
                .await;
        }
    }
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    let Some(guild_id) = require_guild_id(ctx, command).await else {
        return;
    };
    let Some(sub) = command.data.options.first() else {
        return;
    };
    let sub_name = sub.name.clone();
    let opts = match &sub.value {
        serenity::all::CommandDataOptionValue::SubCommand(o) => o.clone(),
        _ => return,
    };
    if sub_name == "financer" {
        handle_fund(ctx, command, &guild_id, &opts).await;
        return;
    }
    if sub_name == "liste" {
        handle_list(ctx, command, &guild_id).await;
        return;
    }
    // Sous-commande propose (par defaut).
    let titre = option_str(&opts, "titre").unwrap_or("");
    let texte = option_str(&opts, "texte").unwrap_or("");
    // Effet optionnel : parametre (choix) + valeur (entier).
    let parametre = option_str(&opts, "parametre");
    let valeur = opts.iter().find(|o| o.name == "valeur").and_then(|o| {
        match &o.value {
            serenity::all::CommandDataOptionValue::Integer(i) => Some(*i),
            _ => None,
        }
    });

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
        parametre,
        valeur,
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
    let mut embed = CreateEmbed::new()
        .title(format!("📜 Loi : {}", s.title))
        .color(color)
        .description(body)
        .field("Statut", s.status_label.clone(), true)
        .field("Votants", total.to_string(), true)
        .field("✅ Pour", s.pour.to_string(), true)
        .field("❌ Contre", s.contre.to_string(), true)
        .field("⚪ Abstention", s.abstention.to_string(), true)
        .field(
            "⚖️ Poids (influence)",
            format!("Pour **{}** / Contre **{}**", s.pour_weight, s.contre_weight),
            false,
        );
    if let (Some(label), Some(val)) = (&s.effect_label, s.effect_value) {
        embed = embed.field(
            "⚙️ Effet si adoptée",
            format!("**{label}** → **{val}**"),
            false,
        );
    }
    if s.funding_pour > 0 || s.funding_contre > 0 {
        embed = embed.field(
            "🏛️ Financement (lobbying)",
            format!("Pour **{}** / Contre **{}**", s.funding_pour, s.funding_contre),
            false,
        );
    }
    embed
        .footer(CreateEmbedFooter::new(if s.status == "vote" {
            "Le résultat est pondéré par l'influence des votants. Clôture automatique à l'échéance."
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
