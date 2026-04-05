use serenity::all::{
    CommandDataOptionValue, CommandInteraction, CommandOptionType, Context, CreateCommand,
    CreateCommandOption, CreateEmbed, CreateEmbedFooter, CreateInteractionResponse,
    CreateInteractionResponseMessage,
};

use crate::handler::{GameDbKey, load_guild_config};

pub fn register() -> CreateCommand {
    CreateCommand::new("pari")
        .description("Parie sur l'issue du combat d'un joueur !")
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::User,
                "combattant",
                "Le joueur sur lequel tu paries (doit avoir un combat en attente)",
            )
            .required(true),
        )
        .add_option(
            CreateCommandOption::new(CommandOptionType::Integer, "mise", "Montant du pari")
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
    if !config.enabled() {
        reply_ephemeral(ctx, command, "Le jeu Coup de Coude est desactive sur ce serveur.").await;
        return;
    }

    let target_id = command
        .data
        .options
        .iter()
        .find(|o| o.name == "combattant")
        .and_then(|o| match &o.value {
            CommandDataOptionValue::User(id) => Some(*id),
            _ => None,
        })
        .unwrap();

    let mise = command
        .data
        .options
        .iter()
        .find(|o| o.name == "mise")
        .and_then(|o| match &o.value {
            CommandDataOptionValue::Integer(v) => Some(*v),
            _ => None,
        })
        .unwrap_or(10);

    let bettor_id = command.user.id.to_string();
    let target_id_str = target_id.to_string();

    // On ne peut pas parier sur soi-meme
    if bettor_id == target_id_str {
        reply_ephemeral(ctx, command, "Tu ne peux pas parier sur toi-meme !").await;
        return;
    }

    let data = ctx.data.read().await;
    let db = data.get::<GameDbKey>().unwrap();

    // Creer/recuperer le parieur
    let bettor = match db
        .get_or_create_player(&guild_id, &bettor_id, &command.user.name)
        .await
    {
        Ok(p) => p,
        Err(e) => {
            reply_ephemeral(ctx, command, &format!("Erreur DB : {e}")).await;
            return;
        }
    };

    if bettor.coins < mise {
        reply_ephemeral(
            ctx,
            command,
            &format!(
                "Pas assez de coins ! Tu as {} coins, mise demandee : {}.",
                bettor.coins, mise
            ),
        )
        .await;
        return;
    }

    // Chercher un combat pending pour le combattant (comme attaquant ou defenseur)
    let combat = match db
        .get_pending_combat_for_player(&guild_id, &target_id_str)
        .await
    {
        Ok(Some(c)) => c,
        Ok(None) => {
            reply_ephemeral(
                ctx,
                command,
                &format!("<@{}> n'a aucun combat en attente !", target_id),
            )
            .await;
            return;
        }
        Err(e) => {
            reply_ephemeral(ctx, command, &format!("Erreur DB : {e}")).await;
            return;
        }
    };

    // Verifier que le parieur n'est ni attaquant ni defenseur
    if bettor_id == combat.attacker_id || bettor_id == combat.defender_id {
        reply_ephemeral(
            ctx,
            command,
            "Tu ne peux pas parier sur un combat dans lequel tu participes !",
        )
        .await;
        return;
    }

    // Deduire la mise du parieur
    if let Err(e) = db.update_player_coins(&guild_id, &bettor_id, -mise).await {
        reply_ephemeral(ctx, command, &format!("Erreur DB : {e}")).await;
        return;
    }

    // Inserer le pari
    if let Err(e) = db
        .place_bet(
            &guild_id,
            combat.id,
            &bettor_id,
            &command.user.name,
            &target_id_str,
            mise,
        )
        .await
    {
        // Rembourser en cas d'erreur
        let _ = db.update_player_coins(&guild_id, &bettor_id, mise).await;
        reply_ephemeral(ctx, command, &format!("Erreur pari : {e}")).await;
        return;
    }

    let embed = CreateEmbed::new()
        .title("\u{1f3b2} Pari enregistre !")
        .description(format!(
            "<@{}> parie **{} coins** sur la victoire de <@{}> !\n\nCombat : <@{}> vs <@{}>",
            command.user.id, mise, target_id, combat.attacker_id, combat.defender_id
        ))
        .color(0xF1C40F)
        .footer(CreateEmbedFooter::new("Coup de Coude | Sentinel"))
        .timestamp(serenity::model::Timestamp::now());

    command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new().embed(embed),
            ),
        )
        .await
        .ok();
}

async fn reply_ephemeral(ctx: &Context, command: &CommandInteraction, content: &str) {
    command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content(content)
                    .ephemeral(true),
            ),
        )
        .await
        .ok();
}
