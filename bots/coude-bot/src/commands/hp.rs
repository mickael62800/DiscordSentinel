use serenity::all::{
    CommandInteraction, Context, CreateCommand, CreateEmbed, CreateEmbedFooter,
    CreateInteractionResponse, CreateInteractionResponseMessage,
};

use crate::game::combat;
use crate::handler::load_guild_config;
use crate::GameApiKey;

/// HP regenerated per tick.
const REGEN_HP_PER_HOUR: i32 = 10;

pub fn register() -> CreateCommand {
    CreateCommand::new("hp")
        .description("Affiche tes points de vie actuels")
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
    if !crate::channel_check::check_channel(ctx, command, config.channel_profil()).await {
        return;
    }

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

    let hp_max = combat::calculate_hp_max(&player);
    let hp_current = player.hp_current.unwrap_or(hp_max).min(hp_max);

    // Barre de vie visuelle (20 segments for more granularity)
    let segments = 20;
    let filled = ((hp_current as f64 / hp_max as f64) * segments as f64).round() as i32;
    let filled = filled.clamp(0, segments);
    let empty = segments - filled;
    let bar_full = "\u{2588}".repeat(filled as usize);
    let bar_empty = "\u{2591}".repeat(empty as usize);

    let hp_pct = (hp_current as f64 / hp_max as f64 * 100.0).round() as i32;

    // Couleur selon le pourcentage de vie
    let color = if hp_pct > 60 {
        0x57F287 // vert
    } else if hp_pct > 30 {
        0xF1C40F // jaune
    } else {
        0xED4245 // rouge
    };

    // Status emoji/text
    let status = if hp_pct <= 20 {
        "\u{1f480} **CRITIQUE** \u{2014} Tu ne peux pas combattre !"
    } else if hp_pct <= 50 {
        "\u{1f915} **Blesse** \u{2014} Pense a te soigner !"
    } else if hp_pct < 100 {
        "\u{1f44d} **En forme**"
    } else {
        "\u{2764}\u{fe0f} **Pleine sante !**"
    };

    // Regen timer (based on hp_last_regen)
    let regen_msg = if hp_current >= hp_max {
        "\u{2705} HP au maximum !".to_string()
    } else {
        let hp_needed = hp_max - hp_current;
        let hours_needed = ((hp_needed as f64) / (REGEN_HP_PER_HOUR as f64)).ceil() as i32;

        let next_regen = if let Some(ref last_regen) = player.hp_last_regen {
            if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(last_regen) {
                let next = dt.with_timezone(&chrono::Utc)
                    + chrono::Duration::hours(1);
                let remaining = next
                    .signed_duration_since(chrono::Utc::now())
                    .num_minutes();
                if remaining > 0 {
                    format!("Prochaine regen dans ~{}min", remaining)
                } else {
                    "Regen imminente !".to_string()
                }
            } else {
                "Regen en cours...".to_string()
            }
        } else {
            "Regen en cours...".to_string()
        };

        format!(
            "\u{1f504} +{} HP/heure | ~{}h pour full HP\n\u{23f0} {}",
            REGEN_HP_PER_HOUR, hours_needed, next_regen
        )
    };

    let embed = CreateEmbed::new()
        .title(format!("\u{2764}\u{fe0f} HP de {}", command.user.name))
        .description(format!(
            "**{}/{}** HP ({}%)\n`{}{}`\n\n{}\n\n{}",
            hp_current, hp_max, hp_pct, bar_full, bar_empty, status, regen_msg
        ))
        .color(color)
        .footer(CreateEmbedFooter::new("Coup de Coude | Sentinel"))
        .timestamp(serenity::model::Timestamp::now());

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
        tracing::warn!(error = %e, "Echec response Discord");
    }
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
