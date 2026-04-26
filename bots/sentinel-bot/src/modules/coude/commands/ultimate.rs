//! Commande `/ultimate` — affiche / utilise l ultimate de classe
//! (cf. COUPE_AMELIORATIONS 3.1).
//!
//! Premiere passe : affichage uniquement (declaratif). L activation
//! mecanique des effets (HP swap, coin flip, vol pre-combat, statue)
//! sera branchee dans des commits suivants.

use serenity::all::{
    CommandInteraction, Context, CreateCommand, CreateEmbed, CreateEmbedFooter,
    CreateInteractionResponse, CreateInteractionResponseMessage,
};

use sentinel_shared::discord_helpers::reply_ephemeral;

use crate::modules::coude::ultimates::{
    format_ultimate_for_class, ultimate_for_class, ULTIMATE_UNLOCK_LEVEL,
};
use crate::modules::coude::GameApiKey;

pub fn register() -> CreateCommand {
    CreateCommand::new("ultimate")
        .description("Affiche ton ultimate de classe (debloque au niveau 10)")
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    let guild_id = match command.guild_id {
        Some(id) => id.to_string(),
        None => {
            reply_ephemeral(ctx, command, "Commande serveur uniquement.").await;
            return;
        }
    };
    let user_id = command.user.id.to_string();
    let data = ctx.data.read().await;
    let api = data.get::<GameApiKey>().unwrap();
    let player = match api
        .get_or_create_player(&guild_id, &user_id, &command.user.name)
        .await
    {
        Ok(p) => p,
        Err(e) => {
            reply_ephemeral(ctx, command, &format!("Erreur API : {e}")).await;
            return;
        }
    };

    let class_key = player.class.as_deref().unwrap_or("bourrin");
    let ult = ultimate_for_class(class_key);

    let embed = if let Some(u) = ult {
        let unlock_status = if player.level >= ULTIMATE_UNLOCK_LEVEL {
            format!("\u{2705} **Debloque** (niveau {})", player.level)
        } else {
            format!(
                "\u{1f512} Verrouille — debloque au niveau {} (tu es niveau {})",
                ULTIMATE_UNLOCK_LEVEL, player.level
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
                format_ultimate_for_class(class_key, player.level)
            ))
            .color(0x9B59B6)
            .footer(CreateEmbedFooter::new(
                sentinel_shared::branding::COUDE_TAGLINE_SHORT,
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
