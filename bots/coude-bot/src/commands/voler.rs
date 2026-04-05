use rand::Rng;
use serenity::all::{
    CommandDataOptionValue, CommandInteraction, CommandOptionType, Context, CreateCommand,
    CreateCommandOption, CreateEmbed, CreateEmbedFooter, CreateInteractionResponse,
    CreateInteractionResponseMessage,
};

use crate::game::progression;
use crate::handler::{GameDbKey, load_guild_config};

const FAIL_MESSAGES: &[&str] = &[
    "\u{1f921} {user} a essaye de voler {target} mais s'est pris les pieds dans le tapis !",
    "\u{1f6a8} {user} s'est fait choper la main dans le sac par {target} !",
    "\u{1f480} {user} a glisse en essayant de pickpocket {target}. Honteux.",
    "\u{1f414} {user} a panique et a lache ses propres coins en fuyant !",
];

pub fn register() -> CreateCommand {
    CreateCommand::new("voler")
        .description("Tente de pickpocket un joueur !")
        .add_option(
            CreateCommandOption::new(CommandOptionType::User, "cible", "Le joueur a voler")
                .required(true),
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

    let thief_id = command.user.id.to_string();
    let target_id_str = target_id.to_string();

    if thief_id == target_id_str {
        reply_ephemeral(ctx, command, "Tu ne peux pas te voler toi-meme !").await;
        return;
    }

    let config = load_guild_config(ctx, &guild_id).await;
    if !config.steal_enabled() {
        reply_ephemeral(ctx, command, "Le vol est desactive sur ce serveur.").await;
        return;
    }

    let data = ctx.data.read().await;
    let db = data.get::<GameDbKey>().unwrap();

    // Verifier le cooldown
    match db.check_cooldown(&guild_id, &thief_id, "voler").await {
        Ok(Some(expires_at)) => {
            let remaining = expires_at
                .signed_duration_since(chrono::Utc::now())
                .num_seconds();
            if remaining > 0 {
                let mins = remaining / 60;
                let secs = remaining % 60;
                reply_ephemeral(
                    ctx,
                    command,
                    &format!(
                        "Tu dois attendre encore {}m{}s avant de pouvoir voler quelqu'un !",
                        mins, secs
                    ),
                )
                .await;
                return;
            }
        }
        Ok(None) => {}
        Err(e) => {
            reply_ephemeral(ctx, command, &format!("Erreur DB : {e}")).await;
            return;
        }
    }

    // Creer/recuperer les joueurs
    let thief = match db
        .get_or_create_player(&guild_id, &thief_id, &command.user.name)
        .await
    {
        Ok(p) => p,
        Err(e) => {
            reply_ephemeral(ctx, command, &format!("Erreur DB : {e}")).await;
            return;
        }
    };

    let target_user = match target_id.to_user(&ctx.http).await {
        Ok(u) => u,
        Err(_) => {
            reply_ephemeral(ctx, command, "Utilisateur introuvable.").await;
            return;
        }
    };

    if target_user.bot {
        reply_ephemeral(ctx, command, "Tu ne peux pas voler un bot !").await;
        return;
    }

    let target_player = match db
        .get_or_create_player(&guild_id, &target_id_str, &target_user.name)
        .await
    {
        Ok(p) => p,
        Err(e) => {
            reply_ephemeral(ctx, command, &format!("Erreur DB : {e}")).await;
            return;
        }
    };

    if target_player.coins < 10 {
        reply_ephemeral(
            ctx,
            command,
            &format!(
                "<@{}> n'a que {} coins... Meme les voleurs ont des principes !",
                target_id, target_player.coins
            ),
        )
        .await;
        return;
    }

    // Poser le cooldown
    if let Err(e) = db.set_cooldown(&guild_id, &thief_id, "voler", 1800).await {
        reply_ephemeral(ctx, command, &format!("Erreur DB : {e}")).await;
        return;
    }

    // Determiner le succes : 30% de base, 50% pour les fourbes
    let success_rate = if thief.class == "fourbe" { 50 } else { 30 };
    let roll: u32 = {
        let mut rng = rand::thread_rng();
        rng.gen_range(1..=100)
    };

    let success = roll <= success_rate;

    let embed = if success {
        // Voler entre 10-25% des coins de la cible
        let steal_pct: f64 = {
            let mut rng = rand::thread_rng();
            rng.gen_range(10.0..=25.0) / 100.0
        };
        let stolen = (target_player.coins as f64 * steal_pct) as i64;
        let stolen = stolen.max(1);

        // Transferer les coins
        if let Err(e) = db
            .transfer_coins(&guild_id, &target_id_str, &thief_id, stolen)
            .await
        {
            reply_ephemeral(ctx, command, &format!("Erreur DB : {e}")).await;
            return;
        }

        // XP pour vol reussi
        let mut xp_line = String::new();
        if let Ok((_new_xp, new_level, leveled_up, stat_points)) =
            db.add_xp(&guild_id, &thief_id, 5).await
        {
            xp_line.push_str(&format!("\n\u{2b06}\u{fe0f} +5 XP"));
            if leveled_up {
                let title = progression::title_for_level(new_level);
                xp_line.push_str(&format!(
                    "\n\u{1f31f} **LEVEL UP !** Niveau **{}** \u{300c}{}\u{300d} ! (+{} points de stats)",
                    new_level, title, stat_points
                ));
            }
        }

        CreateEmbed::new()
            .title("\u{1f4b0} Vol reussi !")
            .description(format!(
                "<@{}> a subtilise **{} coins** a <@{}> !\n\n\u{1f575}\u{fe0f} Discretion exemplaire.{}",
                command.user.id, stolen, target_id, xp_line
            ))
            .color(0x57F287)
            .footer(CreateEmbedFooter::new("Coup de Coude | Sentinel"))
            .timestamp(serenity::model::Timestamp::now())
    } else {
        // Perdre 15% de ses propres coins
        let lost = (thief.coins as f64 * 0.15) as i64;
        let lost = lost.max(1);

        let _ = db
            .update_player_coins(&guild_id, &thief_id, -lost)
            .await;

        let fail_msg = {
            let mut rng = rand::thread_rng();
            let idx = rng.gen_range(0..FAIL_MESSAGES.len());
            FAIL_MESSAGES[idx]
                .replace("{user}", &format!("<@{}>", command.user.id))
                .replace("{target}", &format!("<@{}>", target_id))
        };

        CreateEmbed::new()
            .title("\u{1f6a8} Vol rate !")
            .description(format!(
                "{}\n\n<@{}> perd **{} coins** dans la tentative !",
                fail_msg, command.user.id, lost
            ))
            .color(0xED4245)
            .footer(CreateEmbedFooter::new("Coup de Coude | Sentinel"))
            .timestamp(serenity::model::Timestamp::now())
    };

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
