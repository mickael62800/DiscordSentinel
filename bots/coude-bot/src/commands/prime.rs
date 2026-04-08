use serenity::all::{
    CommandDataOptionValue, CommandInteraction, CommandOptionType, Context, CreateCommand,
    CreateCommandOption, CreateEmbed, CreateEmbedFooter, CreateInteractionResponse,
    CreateInteractionResponseMessage,
};

use crate::handler::{GameDbKey, load_guild_config};

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
    let guild_id = match command.guild_id {
        Some(id) => id.to_string(),
        None => {
            reply_ephemeral(ctx, command, "Commande serveur uniquement.").await;
            return;
        }
    };

    let config = load_guild_config(ctx, &guild_id).await;
    if !crate::channel_check::check_channel(ctx, command, config.channel_activites()).await {
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

    let data = ctx.data.read().await;
    let db = data.get::<GameDbKey>().unwrap();

    let player = match db
        .get_or_create_player(&guild_id, &command.user.id.to_string(), &command.user.name)
        .await
    {
        Ok(p) => p,
        Err(e) => {
            reply_ephemeral(ctx, command, &format!("Erreur DB : {e}")).await;
            return;
        }
    };

    if player.coins < amount {
        reply_ephemeral(
            ctx,
            command,
            &format!("Pas assez de coins ! Tu as {} coins.", player.coins),
        )
        .await;
        return;
    }

    // Creer le joueur cible s'il n'existe pas
    if let Err(e) = db
        .get_or_create_player(&guild_id, &target.id.to_string(), &target.name)
        .await
    {
        tracing::warn!(error = %e, "Echec DB get_or_create_player cible prime");
    }

    // Deduire les coins
    if let Err(e) = db
        .update_player_coins(&guild_id, &command.user.id.to_string(), -amount)
        .await
    {
        reply_ephemeral(ctx, command, &format!("Erreur DB : {e}")).await;
        return;
    }

    // Creer la prime
    if let Err(e) = db
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
        reply_ephemeral(ctx, command, &format!("Erreur DB : {e}")).await;
        return;
    }

    // Verifier le total des primes actives sur la cible
    let active_primes = db
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
        .footer(CreateEmbedFooter::new("Coup de Coude | Sentinel"))
        .timestamp(serenity::model::Timestamp::now());

    crate::channel_check::post_activity(ctx, command, config.channel_activites(), embed).await;
}

async fn reply_ephemeral(ctx: &Context, command: &CommandInteraction, content: &str) {
    if let Err(e) = command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content(content)
                    .ephemeral(true),
            ),
        )
        .await
    {
        tracing::warn!(error = %e, "Echec response Discord");
    }
}
