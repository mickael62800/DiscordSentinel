use rand::Rng;
use serenity::all::{
    CommandDataOptionValue, CommandInteraction, CommandOptionType, Context, CreateCommand,
    CreateCommandOption, CreateEmbed, CreateEmbedFooter, CreateInteractionResponse,
    CreateInteractionResponseMessage,
};

use crate::game::progression;
use crate::handler::{GameDbKey, load_guild_config};

pub fn register() -> CreateCommand {
    CreateCommand::new("casino")
        .description("Tente ta chance au casino !")
        .add_option(
            CreateCommandOption::new(CommandOptionType::Integer, "mise", "Montant a miser")
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
    if !config.casino_enabled() {
        reply_ephemeral(ctx, command, "Le casino est desactive sur ce serveur.").await;
        return;
    }

    let mise = command
        .data
        .options
        .iter()
        .find(|o| o.name == "mise")
        .and_then(|o| match &o.value {
            CommandDataOptionValue::Integer(v) => Some(*v),
            _ => None,
        })
        .unwrap_or(config.default_bet());

    if mise > config.casino_max_bet() {
        reply_ephemeral(ctx, command, &format!("La mise max au casino est de {} coins.", config.casino_max_bet())).await;
        return;
    }

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

    if player.coins < mise {
        reply_ephemeral(
            ctx,
            command,
            &format!("Pas assez de coins ! Tu as {} coins.", player.coins),
        )
        .await;
        return;
    }

    // Tirage
    let roll: u32 = {
        let mut rng = rand::thread_rng();
        rng.gen_range(1..=100)
    };

    let (title, description, gain, color, is_faillite) = if roll <= 2 {
        // 2% faillite_totale
        (
            "\u{1f4a3} FAILLITE TOTALE !",
            format!(
                "<@{}> a TOUT perdu ! **{} coins** partis en fumee !\n\n\u{1f480} La roue ne pardonne pas...",
                command.user.id, player.coins
            ),
            0i64,
            0xED4245u32,
            true,
        )
    } else if roll <= 2 + 50 {
        // 50% lose all bet
        (
            "\u{1f3b0} Perdu !",
            format!(
                "<@{}> perd **{} coins** au casino !\n\n\u{1f622} La prochaine sera la bonne...",
                command.user.id, mise
            ),
            -mise,
            0xED4245,
            false,
        )
    } else if roll <= 2 + 50 + 25 {
        // 25% win x2
        let win = mise * 2;
        (
            "\u{1f389} x2 !",
            format!(
                "<@{}> remporte **{} coins** ! (x2)\n\n\u{2728} Bien joue !",
                command.user.id, win
            ),
            win - mise, // net gain
            0xF1C40F,
            false,
        )
    } else if roll <= 2 + 50 + 25 + 15 {
        // 15% win x5
        let win = mise * 5;
        (
            "\u{1f525} x5 !",
            format!(
                "<@{}> remporte **{} coins** ! (x5)\n\n\u{1f4ab} Incroyable !",
                command.user.id, win
            ),
            win - mise,
            0xF1C40F,
            false,
        )
    } else {
        // 8% jackpot x10
        let win = mise * 10;
        (
            "\u{1f451} JACKPOT x10 !!!",
            format!(
                "<@{}> decroche le JACKPOT et remporte **{} coins** ! (x10)\n\n\u{1f4b0}\u{1f4b0}\u{1f4b0} LA LEGENDE !",
                command.user.id, win
            ),
            win - mise,
            0xF1C40F,
            false,
        )
    };

    // Appliquer le resultat
    if is_faillite {
        let _ = db
            .record_casino_faillite(&guild_id, &command.user.id.to_string())
            .await;
    } else if gain >= 0 {
        let _ = db
            .record_casino_win(&guild_id, &command.user.id.to_string(), gain)
            .await;
    } else {
        let _ = db
            .record_casino_loss(&guild_id, &command.user.id.to_string(), -gain)
            .await;
    }

    // XP pour jackpot x10 (roll > 92)
    let mut xp_line = String::new();
    if roll > 92 {
        if let Ok((_new_xp, new_level, leveled_up, stat_points)) =
            db.add_xp(&guild_id, &command.user.id.to_string(), 10).await
        {
            xp_line.push_str(&format!("\n\n\u{2b06}\u{fe0f} +10 XP (Jackpot bonus !)"));
            if leveled_up {
                let new_title = progression::title_for_level(new_level);
                xp_line.push_str(&format!(
                    "\n\u{1f31f} **LEVEL UP !** Niveau **{}** \u{300c}{}\u{300d} ! (+{} points de stats)",
                    new_level, new_title, stat_points
                ));
            }
        }
    }

    let description = format!("{}{}", description, xp_line);

    let embed = CreateEmbed::new()
        .title(title)
        .description(&description)
        .color(color)
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
