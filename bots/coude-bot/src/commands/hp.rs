use serenity::all::{
    CommandInteraction, Context, CreateCommand, CreateEmbed, CreateEmbedFooter,
    CreateInteractionResponse, CreateInteractionResponseMessage,
};

use crate::handler::load_guild_config;
use crate::GameApiKey;

/// Paliers de regen (HP/h par bande de % HP courant).
/// Doivent rester en sync avec services/workers/coude-worker/src/jobs/hp_regen.rs.
const RATE_0_25: f64 = 100.0;
const RATE_25_50: f64 = 50.0;
const RATE_50_75: f64 = 30.0;
const RATE_75_100: f64 = 10.0;

/// Calcule le temps (en minutes) necessaire pour passer de `hp_current` a
/// `hp_max` avec la regen degressive.
fn estimate_minutes_to_full(mut hp_current: i32, hp_max: i32) -> i32 {
    if hp_current >= hp_max || hp_max <= 0 {
        return 0;
    }
    let mut total_minutes = 0.0_f64;
    // On consomme palier par palier ; chaque palier est defini par un
    // plafond en HP absolu et un taux horaire.
    let thresholds: [(i32, f64); 4] = [
        (hp_max / 4, RATE_0_25),
        (hp_max / 2, RATE_25_50),
        ((hp_max * 3) / 4, RATE_50_75),
        (hp_max, RATE_75_100),
    ];
    for (ceiling, rate) in thresholds {
        if hp_current >= ceiling {
            continue;
        }
        let delta = (ceiling - hp_current) as f64;
        total_minutes += (delta / rate) * 60.0;
        hp_current = ceiling;
    }
    total_minutes.ceil() as i32
}

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

    // hp_max est maintenu par l'API (recalcule a chaque combat + sur
    // spend_stat_point). Fallback 100 si jamais Option::None.
    let hp_max = player.hp_max.unwrap_or(100);
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
    let status = if hp_pct < 10 {
        "\u{1f480} **KO** \u{2014} Tu ne peux pas combattre ! (seuil : 10 %)"
    } else if hp_pct <= 25 {
        "\u{1fa78} **Critique** \u{2014} Pense a te soigner !"
    } else if hp_pct <= 50 {
        "\u{1f915} **Blesse**"
    } else if hp_pct < 100 {
        "\u{1f44d} **En forme**"
    } else {
        "\u{2764}\u{fe0f} **Pleine sante !**"
    };

    // Regen timer : on affiche le taux du palier courant + estimation
    // du temps total pour un full heal.
    let regen_msg = if hp_current >= hp_max {
        "\u{2705} HP au maximum !".to_string()
    } else {
        let current_rate = if hp_pct < 25 {
            RATE_0_25
        } else if hp_pct < 50 {
            RATE_25_50
        } else if hp_pct < 75 {
            RATE_50_75
        } else {
            RATE_75_100
        };
        let minutes_to_full = estimate_minutes_to_full(hp_current, hp_max);
        let full_heal_str = if minutes_to_full < 60 {
            format!("~{}min", minutes_to_full)
        } else {
            format!("~{}h{:02}", minutes_to_full / 60, minutes_to_full % 60)
        };
        format!(
            "\u{1f504} Palier actuel : **+{} HP/h** (regen degressive)\n\
             \u{23f0} Full heal dans environ **{}**\n\
             \u{1f4a4} `/repos` = heal total instantane (cooldown 12h)",
            current_rate as i32, full_heal_str
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
