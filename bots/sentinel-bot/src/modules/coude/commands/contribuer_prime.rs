//! Commande `/contribuer-prime` — pile sur la prime collective d une cible
//! (cf. COUPE_AMELIORATIONS 5.3).

use serenity::all::{
    CommandDataOptionValue, CommandInteraction, CommandOptionType, Context, CreateCommand,
    CreateCommandOption, CreateEmbed, CreateEmbedFooter,
};

use sentinel_shared::discord_helpers::reply_ephemeral;

use crate::modules::coude::load_guild_config;
use crate::modules::coude::GameApiKey;

const MIN_CONTRIBUTION: i64 = 50;

pub fn register() -> CreateCommand {
    CreateCommand::new("contribuer-prime")
        .description("Ajoute des coins a la prime collective d un joueur a haute serie")
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::User,
                "cible",
                "Le joueur sur qui une prime est ouverte",
            )
            .required(true),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::Integer,
                "montant",
                "Coins a ajouter (50c minimum)",
            )
            .required(true)
            .min_int_value(MIN_CONTRIBUTION as u64),
        )
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    let guild_id = match command.guild_id {
        Some(id) => id.to_string(),
        None => {
            reply_ephemeral(ctx, command, "Commande serveur uniquement.").await;
            return;
        }
    };

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
    let amount = command
        .data
        .options
        .iter()
        .find(|o| o.name == "montant")
        .and_then(|o| match &o.value {
            CommandDataOptionValue::Integer(v) => Some(*v),
            _ => None,
        })
        .unwrap_or(0);

    if amount < MIN_CONTRIBUTION {
        reply_ephemeral(
            ctx,
            command,
            &format!("Contribution minimum : {}c.", MIN_CONTRIBUTION),
        )
        .await;
        return;
    }

    let contributor_id = command.user.id.to_string();
    let target_id_str = target_id.to_string();

    if contributor_id == target_id_str {
        reply_ephemeral(ctx, command, "Tu ne peux pas piler sur ta propre tete !").await;
        return;
    }

    let data = ctx.data.read().await;
    let api = data.get::<GameApiKey>().unwrap();

    // Verif solde du contributeur.
    let player = match api
        .get_or_create_player(&guild_id, &contributor_id, &command.user.name)
        .await
    {
        Ok(p) => p,
        Err(e) => {
            reply_ephemeral(ctx, command, &format!("Erreur API : {e}")).await;
            return;
        }
    };
    if player.coins < amount {
        reply_ephemeral(
            ctx,
            command,
            &format!("Solde insuffisant ({}c).", player.coins),
        )
        .await;
        return;
    }

    // Debit du wallet, puis contribution. En 2 temps : si la contribution
    // echoue (pas de prime ouverte par exemple), on rollback le debit.
    if let Err(e) = api
        .update_player_coins(&guild_id, &contributor_id, -amount)
        .await
    {
        reply_ephemeral(ctx, command, &format!("Erreur API : {e}")).await;
        return;
    }

    match api
        .contribute_to_bounty(
            &guild_id,
            &target_id_str,
            &contributor_id,
            &command.user.name,
            amount,
        )
        .await
    {
        Ok(resp) => {
            let embed = CreateEmbed::new()
                .title("\u{1f48e} CONTRIBUTION A LA PRIME")
                .description(format!(
                    "<@{}> ajoute **{}c** sur la tete de <@{}> !\n\n\
                     \u{1f4b0} Pot total : **{}c**.\n\
                     Le prochain joueur a battre <@{}> empoche tout.",
                    command.user.id, amount, target_id, resp.new_total, target_id
                ))
                .color(0xFFD700)
                .footer(CreateEmbedFooter::new(
                    sentinel_shared::branding::COUDE_TAGLINE_SHORT,
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
            // Rollback : on rend les coins au contributeur.
            let _ = api
                .update_player_coins(&guild_id, &contributor_id, amount)
                .await;
            let msg = if e.contains("Aucune prime") || e.to_lowercase().contains("not found") {
                format!(
                    "Pas de prime ouverte sur <@{}>. Une prime s ouvre automatiquement quand un joueur atteint 5 victoires consecutives.",
                    target_id
                )
            } else {
                format!("Erreur API : {e}")
            };
            reply_ephemeral(ctx, command, &msg).await;
        }
    }
}
