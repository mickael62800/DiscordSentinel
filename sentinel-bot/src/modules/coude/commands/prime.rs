use serenity::all::{
    CommandDataOptionValue, CommandInteraction, CommandOptionType, Context, CreateCommand,
    CreateCommandOption, CreateEmbed, CreateEmbedFooter,
};

use crate::shared::discord_helpers::{reply_ephemeral, require_guild_id};

use crate::modules::coude::GameApiKey;
use crate::modules::coude::load_guild_config;

pub fn register() -> CreateCommand {
    CreateCommand::new("prime")
        .description("Place une prime sur la tete d'un joueur")
        .add_option(
            CreateCommandOption::new(CommandOptionType::User, "cible", "Le joueur a cibler")
                .required(true),
        )
        .add_option(
            CreateCommandOption::new(CommandOptionType::Integer, "montant", "Montant de la prime")
                .required(true)
                .min_int_value(1),
        )
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    let Some(guild_id) = require_guild_id(ctx, command).await else { return; };

    let config = load_guild_config(ctx, &guild_id).await;
    if !crate::modules::coude::channel_check::check_channel(ctx, command, config.channel_activites()).await {
        return;
    }
    if !config.enabled() {
        reply_ephemeral(ctx, command, "Le jeu Coup de Coude est desactive sur ce serveur.").await;
        return;
    }

    let target_id = command
        .data
        .options
        .iter()
        .find(|o| o.name == "cible")
        .and_then(|o| match &o.value {
            CommandDataOptionValue::User(id) => Some(*id),
            _ => None,
        })
        .unwrap();

    let amount = command
        .data
        .options
        .iter()
        .find(|o| o.name == "montant")
        .and_then(|o| match &o.value {
            CommandDataOptionValue::Integer(v) => Some(*v),
            _ => None,
        })
        .unwrap_or(10);

    if target_id == command.user.id {
        reply_ephemeral(ctx, command, "Tu ne peux pas mettre une prime sur ta propre tete !").await;
        return;
    }

    let target = match target_id.to_user(&ctx.http).await {
        Ok(u) => u,
        Err(_) => {
            reply_ephemeral(ctx, command, "Utilisateur introuvable.").await;
            return;
        }
    };

    // Defer public : 5 appels API (get_player x2, update_coins,
    // create_prime, get_active_primes).
    if !crate::modules::coude::interaction_helper::defer_response(ctx, command).await {
        return;
    }

    let data = ctx.data.read().await;
    let api = data.get::<GameApiKey>().unwrap();

    let player = match api
        .get_or_create_player(&guild_id, &command.user.id.to_string(), &command.user.name)
        .await
    {
        Ok(p) => p,
        Err(e) => {
            crate::modules::coude::interaction_helper::followup_text(ctx, command, &format!("Erreur API : {e}")).await;
            return;
        }
    };

    if player.coins < amount {
        crate::modules::coude::interaction_helper::followup_text(
            ctx,
            command,
            &format!("Pas assez de coins ! Tu as {} coins.", player.coins),
        )
        .await;
        return;
    }

    // Creer le joueur cible s'il n'existe pas
    if let Err(e) = api
        .get_or_create_player(&guild_id, &target.id.to_string(), &target.name)
        .await
    {
        tracing::warn!(error = %e, "Echec API get_or_create_player cible prime");
    }

    // Deduire les coins
    if let Err(e) = api
        .update_player_coins(&guild_id, &command.user.id.to_string(), -amount)
        .await
    {
        crate::modules::coude::interaction_helper::followup_text(ctx, command, &format!("Erreur API : {e}")).await;
        return;
    }

    // Creer la prime
    if let Err(e) = api
        .create_prime(
            &guild_id,
            &target.id.to_string(),
            &target.name,
            &command.user.id.to_string(),
            &command.user.name,
            amount,
        )
        .await
    {
        crate::modules::coude::interaction_helper::followup_text(ctx, command, &format!("Erreur API : {e}")).await;
        return;
    }

    // Verifier le total des primes actives sur la cible
    let active_primes = api
        .get_active_primes(&guild_id, &target.id.to_string())
        .await
        .unwrap_or_default();
    let total: i64 = active_primes.iter().map(|p| p.amount).sum();

    let embed = CreateEmbed::new()
        .title("\u{1f3af} Nouvelle prime !")
        .description(format!(
            "<@{}> a mis une prime de **{} coins** sur la tete de <@{}> !\n\nQuiconque bat <@{}> empochera la prime !",
            command.user.id, amount, target.id, target.id
        ))
        .color(0xE67E22)
        .field(
            "\u{1f4b0} Total des primes actives",
            format!("{} coins", total),
            false,
        )
        .footer(CreateEmbedFooter::new(crate::shared::branding::COUDE_TAGLINE_SHORT))
        .timestamp(serenity::model::Timestamp::now());

    crate::modules::coude::channel_check::post_activity_followup(ctx, command, config.channel_activites(), embed).await;
}

