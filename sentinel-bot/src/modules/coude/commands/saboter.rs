//! Commande `/saboter` — sabotages cibles (cf. COUPE_AMELIORATIONS 5.2).
//!
//! Premier sabotage implemente : "Coller une pancarte" (150c, 7 jours).
//! Marque la cible avec un panneau "Rival officiel de @toi" visible dans
//! son `/profil`. Pure cosmetique : aucune mecanique gameplay derriere.
//!
//! Reutilise l infrastructure curses (table coude_curses, kind=pancarte) —
//! les sabotages partagent le meme contrat "1 effet actif par cible" que
//! les maledictions.

use serenity::all::{
    CommandDataOptionValue, CommandInteraction, CommandOptionType, Context, CreateCommand,
    CreateCommandOption, CreateEmbed, CreateEmbedFooter,
};

use crate::shared::discord_helpers::{reply_ephemeral, require_guild_id, reply_api_err};

use crate::modules::coude::load_guild_config;
use crate::modules::coude::GameApiKey;

pub fn register() -> CreateCommand {
    CreateCommand::new("saboter")
        .description("Sabotages cibles contre un autre joueur (cf. roadmap 5.2)")
        .add_option(
            CreateCommandOption::new(CommandOptionType::String, "type", "Type de sabotage")
                .required(true)
                .add_string_choice("Coller une pancarte (150c, 7 jours)", "pancarte")
                .add_string_choice("Graisser les armes (200c, prochaine attaque speciale)", "graisser")
                .add_string_choice("Empoisonner le wallet (400c, 10% des 3 prochains gains)", "empoisonner")
                .add_string_choice("Fausse assurance (500c, scam quand la cible perd avec une assurance)", "fausse_assurance"),
        )
        .add_option(
            CreateCommandOption::new(CommandOptionType::User, "cible", "Cible du sabotage")
                .required(true),
        )
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    let Some(guild_id) = require_guild_id(ctx, command).await else { return; };

    let config = load_guild_config(ctx, &guild_id).await;
    if !crate::modules::coude::channel_check::check_channel(ctx, command, config.channel_activites()).await {
        return;
    }

    let sabotage_type = command
        .data
        .options
        .iter()
        .find(|o| o.name == "type")
        .and_then(|o| match &o.value {
            CommandDataOptionValue::String(s) => Some(s.clone()),
            _ => None,
        })
        .unwrap_or_default();

    let target_id = match command
        .data
        .options
        .iter()
        .find(|o| o.name == "cible")
        .and_then(|o| match &o.value {
            CommandDataOptionValue::User(id) => Some(*id),
            _ => None,
        }) {
        Some(id) => id,
        None => {
            reply_ephemeral(ctx, command, "Cible manquante.").await;
            return;
        }
    };

    let source_id = command.user.id.to_string();
    let target_id_str = target_id.to_string();

    if source_id == target_id_str {
        reply_ephemeral(ctx, command, "Tu ne peux pas te saboter toi-meme !").await;
        return;
    }

    let target_user = match target_id.to_user(&ctx.http).await {
        Ok(u) => u,
        Err(_) => {
            reply_ephemeral(ctx, command, "Utilisateur introuvable.").await;
            return;
        }
    };
    if target_user.bot {
        reply_ephemeral(ctx, command, "Pas de sabotage contre un bot !").await;
        return;
    }

    let data = ctx.data.read().await;
    let api = data.get::<GameApiKey>().unwrap();

    if let Err(e) = api
        .get_or_create_player(&guild_id, &source_id, &command.user.name)
        .await
    {
        reply_api_err(ctx, command, e).await;
        return;
    }
    if let Err(e) = api
        .get_or_create_player(&guild_id, &target_id_str, &target_user.name)
        .await
    {
        reply_api_err(ctx, command, e).await;
        return;
    }

    let result = api
        .cast_curse(
            &guild_id,
            &source_id,
            &command.user.name,
            &target_id_str,
            Some(&sabotage_type),
        )
        .await;

    match result {
        Ok(out) => {
            let effect_text = match out.kind.as_str() {
                "pancarte" => format!(
                    "<@{}> est marque « Rival officiel de <@{}> » pendant 7 jours, visible dans son `/profil`.",
                    target_id, command.user.id
                ),
                "graisser" => format!(
                    "La **prochaine attaque speciale** de <@{}> en combat foire automatiquement. Effet consume au 1er combat (sinon expire en 24h).",
                    target_id
                ),
                "empoisonner" => format!(
                    "Sur les **3 prochains gains de combat** de <@{}>, **10%** sont redirige vers ton wallet. Expire en 7 jours si non epuise.",
                    target_id
                ),
                "fausse_assurance" => format!(
                    "La prochaine fois que <@{}> perd un combat avec une assurance active, elle est silencieusement annulee + **200c** preleves vers ton wallet. One-shot, expire en 7 jours si non declenche.",
                    target_id
                ),
                _ => format!("Effet inconnu sur <@{}>.", target_id),
            };
            let embed = CreateEmbed::new()
                .title(format!("{} Sabotage execute !", out.kind_emoji))
                .description(format!(
                    "<@{}> vient de poser **{}** sur <@{}> !\n\n\
                     Effet : {}\n\
                     Cout : {}c.",
                    command.user.id,
                    out.kind_label,
                    target_id,
                    effect_text,
                    out.cost_paid,
                ))
                .color(0xE67E22)
                .footer(CreateEmbedFooter::new(
                    crate::shared::branding::COUDE_TAGLINE_SHORT,
                ))
                .timestamp(serenity::model::Timestamp::now());

            crate::modules::coude::channel_check::post_activity(
                ctx,
                command,
                config.channel_activites(),
                embed,
            )
            .await;
        }
        Err(e) => {
            let msg = if e.contains("deja active") {
                "Cette cible a deja un effet actif (malediction ou sabotage). Attends qu il expire.".to_string()
            } else if e.to_lowercase().contains("solde") || e.to_lowercase().contains("insufficient") {
                "Pas assez de coins (150c minimum).".to_string()
            } else {
                format!("Erreur API : {e}")
            };
            reply_ephemeral(ctx, command, &msg).await;
        }
    }
}
