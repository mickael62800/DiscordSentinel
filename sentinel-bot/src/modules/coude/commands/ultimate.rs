//! Commande `/ultimate` — affiche / utilise l ultimate de classe
//! (cf. COUPE_AMELIORATIONS 3.1).
//!
//! Premiere passe : affichage uniquement (declaratif). L activation
//! mecanique des effets (HP swap, coin flip, vol pre-combat, statue)
//! sera branchee dans des commits suivants.

use serenity::all::{
    CommandDataOptionValue, CommandInteraction, CommandOptionType, Context, CreateCommand,
    CreateCommandOption, CreateEmbed, CreateEmbedFooter, CreateInteractionResponse,
    CreateInteractionResponseMessage,
};

use crate::shared::discord_helpers::{reply_ephemeral, require_guild_id, reply_api_err};

use crate::modules::coude::ultimates::{format_ultimate_for_class, ultimate_for_class};
use crate::modules::coude::{load_guild_config, GameApiKey};

pub fn register() -> CreateCommand {
    CreateCommand::new("ultimate")
        .description("Affiche ou active ton ultimate (debloque au niveau 10)")
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::Boolean,
                "activer",
                "true = active sur ton prochain combat (sinon affichage seul)",
            )
            .required(false),
        )
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    let Some(guild_id) = require_guild_id(ctx, command).await else { return; };
    let user_id = command.user.id.to_string();
    let config = load_guild_config(ctx, &guild_id).await;
    let unlock_level = config.ultimate_unlock_level();
    let data = ctx.data.read().await;
    let api = data.get::<GameApiKey>().unwrap();
    let player = match api
        .get_or_create_player(&guild_id, &user_id, &command.user.name)
        .await
    {
        Ok(p) => p,
        Err(e) => {
            reply_api_err(ctx, command, e).await;
            return;
        }
    };

    let class_key = player.class.as_deref().unwrap_or("bourrin");
    let ult = ultimate_for_class(class_key);

    let activate = command
        .data
        .options
        .iter()
        .find(|o| o.name == "activer")
        .and_then(|o| match &o.value {
            CommandDataOptionValue::Boolean(b) => Some(*b),
            _ => None,
        })
        .unwrap_or(false);

    if activate {
        let Some(u) = ult else {
            reply_ephemeral(ctx, command, "Aucun ultimate pour ta classe.").await;
            return;
        };
        if !u.mechanical_implemented && u.class_key != "bourrin" {
            reply_ephemeral(
                ctx,
                command,
                "Cet ultimate n a pas encore d effet mecanique branche — l activation arrive bientot.",
            )
            .await;
            return;
        }
        match api
            .activate_ultimate(&guild_id, &user_id, u.class_key)
            .await
        {
            Ok(_) => {
                let embed = CreateEmbed::new()
                    .title(format!("{} {} — ACTIVE !", u.emoji, u.label))
                    .description(format!(
                        "<@{}> a active **{}** ! Ton prochain combat declenche l effet.\n\n_{}_",
                        command.user.id, u.label, u.description
                    ))
                    .color(0x9B59B6)
                    .footer(CreateEmbedFooter::new(
                        crate::shared::branding::COUDE_TAGLINE_SHORT,
                    ))
                    .timestamp(serenity::model::Timestamp::now());
                if let Err(e) = command
                    .create_response(
                        &ctx.http,
                        CreateInteractionResponse::Message(
                            CreateInteractionResponseMessage::new().embed(embed),
                        ),
                    )
                    .await
                {
                    tracing::warn!(error = %e, "Echec response /ultimate activate");
                }
                return;
            }
            Err(e) => {
                let msg = if e.contains("Cooldown") || e.contains("non disponible") {
                    "Cooldown encore en cours ou level insuffisant.".to_string()
                } else if e.contains("reserve a la classe") {
                    "Cet ultimate ne correspond pas a ta classe.".to_string()
                } else {
                    e.clone()
                };
                reply_ephemeral(ctx, command, &msg).await;
                return;
            }
        }
    }

    let embed = if let Some(u) = ult {
        let unlock_status = if player.level >= unlock_level {
            format!("\u{2705} **Debloque** (niveau {})", player.level)
        } else {
            format!(
                "\u{1f512} Verrouille — debloque au niveau {} (tu es niveau {})",
                unlock_level, player.level
            )
        };
        let mech_note = if u.mechanical_implemented {
            "\n_Effet : actif au prochain combat apres activation._"
        } else {
            "\n_\u{2728} L effet mecanique sera disponible bientot — pour l instant, declaratif uniquement._"
        };
        CreateEmbed::new()
            .title(format!("{} {} — {}", u.emoji, u.name, u.label))
            .description(format!(
                "{}\n\n{}\n\n\u{23f0} Cooldown : **{} jours** entre 2 utilisations.{}\n\n_Resume : {}_",
                u.description,
                unlock_status,
                u.cooldown_days,
                mech_note,
                format_ultimate_for_class(class_key, player.level, unlock_level)
            ))
            .color(0x9B59B6)
            .footer(CreateEmbedFooter::new(
                crate::shared::branding::COUDE_TAGLINE_SHORT,
            ))
            .timestamp(serenity::model::Timestamp::now())
    } else {
        CreateEmbed::new()
            .title("\u{1f512} Aucun ultimate")
            .description(format!(
                "Ta classe ({}) n a pas encore d ultimate dedie.",
                class_key
            ))
            .color(0x95A5A6)
            .timestamp(serenity::model::Timestamp::now())
    };

    if let Err(e) = command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .embed(embed)
                    .ephemeral(true),
            ),
        )
        .await
    {
        tracing::warn!(error = %e, "Echec response /ultimate");
    }
}
