//! Commande `/coalition` — rejoint une coalition contre une cible
//! (cf. COUPE_AMELIORATIONS 5.3).

use serenity::all::{
    CommandDataOptionValue, CommandInteraction, CommandOptionType, Context, CreateCommand,
    CreateCommandOption, CreateEmbed, CreateEmbedFooter,
};

use crate::shared::discord_helpers::{reply_ephemeral, require_guild_id, reply_api_err};

use crate::modules::coude::load_guild_config;
use crate::modules::coude::GameApiKey;

// `cost_per_member` migre dans `Config::coalition_cost_per_member`
// (Phase 1 leftovers audit). Default preserve : 500c.
const MIN_MEMBERS: usize = 3;

pub fn register() -> CreateCommand {
    CreateCommand::new("coalition")
        .description("Rejoint la coalition contre un joueur (500c, devient active a 3 membres)")
        .add_option(
            CreateCommandOption::new(CommandOptionType::User, "cible", "La cible de la coalition")
                .required(true),
        )
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    let Some(guild_id) = require_guild_id(ctx, command).await else { return; };
    let config = load_guild_config(ctx, &guild_id).await;
    if !crate::modules::coude::channel_check::check_channel(ctx, command, config.channel_activites()).await {
        return;
    }

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
    let user_id = command.user.id.to_string();
    let target_id_str = target_id.to_string();
    let cost_per_member = config.coalition_cost_per_member();

    if user_id == target_id_str {
        reply_ephemeral(ctx, command, "Tu ne peux pas te coaliser contre toi-meme !").await;
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
        reply_ephemeral(ctx, command, "Pas de coalition contre un bot !").await;
        return;
    }

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
    if player.coins < cost_per_member {
        reply_ephemeral(
            ctx,
            command,
            &format!("Solde insuffisant ({}c). Cout : {}c.", player.coins, cost_per_member),
        )
        .await;
        return;
    }
    if let Err(e) = api
        .get_or_create_player(&guild_id, &target_id_str, &target_user.name)
        .await
    {
        reply_api_err(ctx, command, e).await;
        return;
    }

    // Debit + join. Rollback si echec.
    if let Err(e) = api
        .update_player_coins(&guild_id, &user_id, -cost_per_member)
        .await
    {
        reply_api_err(ctx, command, e).await;
        return;
    }

    match api
        .join_coalition(&guild_id, &target_id_str, &user_id, &command.user.name)
        .await
    {
        Ok(c) => {
            let n = c.members.len();
            let active = c.status == "active";
            let title = if active {
                "\u{1f5e1}\u{fe0f} COALITION ACTIVE !"
            } else {
                "\u{1f5e1}\u{fe0f} Coalition en formation"
            };
            let body = if active {
                format!(
                    "<@{}> rejoint la coalition contre <@{}> ! ({} membres au total)\n\n\
                     **La coalition est ACTIVE** : tous les gains de combat de <@{}> sont reduits a 80% pendant 48h, ou jusqu a ce qu il batte un membre en combat direct.",
                    command.user.id, target_id, n, target_id
                )
            } else {
                format!(
                    "<@{}> rejoint la coalition contre <@{}>. ({} membre{}, il faut {} pour activer la penalite)\n\n\
                     Continuez a recruter avec `/coalition cible:<@{}>` !",
                    command.user.id, target_id, n, if n > 1 { "s" } else { "" }, MIN_MEMBERS, target_id
                )
            };
            let embed = CreateEmbed::new()
                .title(title)
                .description(body)
                .color(0x6E2C00)
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
            // Rollback : on rend les coins.
            let _ = api
                .update_player_coins(&guild_id, &user_id, cost_per_member)
                .await;
            reply_api_err(ctx, command, e).await;
        }
    }
}
