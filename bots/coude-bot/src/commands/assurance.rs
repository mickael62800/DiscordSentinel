use rand::Rng;
use serenity::all::{
    CommandInteraction, Context, CreateCommand, CreateEmbed, CreateEmbedFooter,
    CreateInteractionResponse, CreateInteractionResponseMessage,
};

use crate::handler::{GameDbKey, load_guild_config};

pub fn register() -> CreateCommand {
    CreateCommand::new("assurance")
        .description("Achete une assurance temporaire contre les pertes de combat !")
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

    let config = load_guild_config(ctx, &guild_id).await;
    if !crate::channel_check::check_channel(ctx, command, config.channel_profil()).await {
        return;
    }
    let insurance_cost = config.insurance_cost();

    let data = ctx.data.read().await;
    let db = data.get::<GameDbKey>().unwrap();

    let player = match db
        .get_or_create_player(&guild_id, &user_id, &command.user.name)
        .await
    {
        Ok(p) => p,
        Err(e) => {
            reply_ephemeral(ctx, command, &format!("Erreur DB : {e}")).await;
            return;
        }
    };

    if player.coins < insurance_cost {
        reply_ephemeral(
            ctx,
            command,
            &format!(
                "Pas assez de coins ! L'assurance coute {} coins, tu en as {}.",
                insurance_cost, player.coins
            ),
        )
        .await;
        return;
    }

    // Verifier si deja assure
    match db.get_active_insurance(&guild_id, &user_id).await {
        Ok(Some(_)) => {
            reply_ephemeral(
                ctx,
                command,
                "Tu as deja une assurance active ! Une seule a la fois.",
            )
            .await;
            return;
        }
        Ok(None) => {}
        Err(e) => {
            reply_ephemeral(ctx, command, &format!("Erreur DB : {e}")).await;
            return;
        }
    }

    // Deduire le cout
    if let Err(e) = db
        .update_player_coins(&guild_id, &user_id, -insurance_cost)
        .await
    {
        reply_ephemeral(ctx, command, &format!("Erreur DB : {e}")).await;
        return;
    }

    // Chance d'arnaque depuis la config
    let is_scam = {
        let mut rng = rand::thread_rng();
        rng.gen_range(1..=100) <= config.insurance_scam_rate()
    };

    if let Err(e) = db.buy_insurance(&guild_id, &user_id, is_scam).await {
        // Rembourser
        let _ = db
            .update_player_coins(&guild_id, &user_id, insurance_cost)
            .await;
        reply_ephemeral(ctx, command, &format!("Erreur DB : {e}")).await;
        return;
    }

    let description = if is_scam {
        format!(
            "\u{1f6e1}\u{fe0f} <@{}> a souscrit une **Assurance Coup de Coude** pour 1 heure !\n\n\
             Les pertes de combat seront reduites de 50%.\n\n\
             \u{1f6e1}\u{fe0f} Assurance activee... (mais est-elle fiable ? \u{1f60f})",
            command.user.id
        )
    } else {
        format!(
            "\u{1f6e1}\u{fe0f} <@{}> a souscrit une **Assurance Coup de Coude** pour 1 heure !\n\n\
             Les pertes de combat seront reduites de 50%.",
            command.user.id
        )
    };

    let embed = CreateEmbed::new()
        .title("\u{1f6e1}\u{fe0f} Assurance activee !")
        .description(description)
        .color(0x3498DB)
        .field("Cout", format!("{} coins", insurance_cost), true)
        .field("Duree", "1 heure", true)
        .field("Protection", "50% des pertes de combat", true)
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
