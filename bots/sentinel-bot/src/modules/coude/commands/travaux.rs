//! Commande `/travaux` — taches communautaires en prison
//! (cf. COUPE_AMELIORATIONS 4.3).
//!
//! Disponible uniquement quand le joueur est en prison. Tirage aleatoire
//! d une tache (50/50 succes) avec cooldown 2h. Total max 24h : ~500c.

use rand::seq::SliceRandom;
use rand::Rng;
use serenity::all::{
    CommandInteraction, Context, CreateCommand, CreateEmbed, CreateEmbedFooter,
};

use sentinel_shared::discord_helpers::reply_ephemeral;

use crate::modules::coude::load_guild_config;
use crate::modules::coude::GameApiKey;

const TRAVAUX_COOLDOWN_SECS: i64 = 2 * 3600;
const TRAVAUX_COOLDOWN_KEY: &str = "travaux_prison";
const SUCCESS_PCT: f64 = 0.5;
const COINS_MIN: i64 = 50;
const COINS_MAX: i64 = 100;
const XP_PER_TASK: i64 = 5;

const TASKS: &[(&str, &str, &str)] = &[
    (
        "clean",
        "\u{1f9f9} Nettoyer les cellules",
        "Tu prends une vadrouille et tu nettoies les vomissures des dernieres bagarres. Pas glorieux, mais ca paie.",
    ),
    (
        "cook",
        "\u{1f373} Cuisiner pour les gardes",
        "Tu rejoins la cuisine, prepares des œufs au plat trop cuits, sers les gardes en silence.",
    ),
    (
        "inform",
        "\u{1f5e3}\u{fe0f} Informer la police",
        "Tu balances quelques rumeurs douteuses sur tes copegars. La police te paie en sourires gras.",
    ),
];

const SUCCESS_FLAVORS: &[&str] = &[
    "Les gardes te tapent sur l epaule. \"T es un peu moins nul que prevu.\"",
    "Personne ne t a vu glander. Bravo.",
    "Le systeme penitentiaire te remercie pour ta contribution citoyenne.",
    "Tu as evite de te faire poignarder. C est deja une victoire.",
];

const FAIL_FLAVORS: &[&str] = &[
    "Tu glisses sur une serpilliere. Les gardes rient. Tu n es pas paye.",
    "Tu rates ta tache. Personne n est etonne.",
    "Un detenu t a vu et te chambre depuis. Pas de coins aujourd hui.",
    "Tu t es endormi a moitie. Reveille-toi quand tu veux travailler vraiment.",
];

pub fn register() -> CreateCommand {
    CreateCommand::new("travaux")
        .description("Effectue une tache de prison (disponible uniquement en cellule)")
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

    let user_id = command.user.id.to_string();
    let data = ctx.data.read().await;
    let api = data.get::<GameApiKey>().unwrap();

    // 1. Verif prison status.
    let status = match api.get_prison_status(&guild_id, &user_id).await {
        Ok(s) => s,
        Err(e) => {
            reply_ephemeral(ctx, command, &format!("Erreur API : {e}")).await;
            return;
        }
    };
    if !status.in_prison {
        reply_ephemeral(
            ctx,
            command,
            "Tu n es pas en prison. /travaux est reserve aux detenus apres un braquage rate.",
        )
        .await;
        return;
    }

    // 2. Verif cooldown 2h.
    match api.check_cooldown(&guild_id, &user_id, TRAVAUX_COOLDOWN_KEY).await {
        Ok(Some(expires_at_str)) => {
            let expires = chrono::DateTime::parse_from_rfc3339(&expires_at_str)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now());
            let remaining = expires.signed_duration_since(chrono::Utc::now()).num_seconds();
            if remaining > 0 {
                let mins = remaining / 60;
                reply_ephemeral(
                    ctx,
                    command,
                    &format!("Tu dois encore te reposer **{}m** avant la prochaine tache.", mins),
                )
                .await;
                return;
            }
        }
        Ok(None) => {}
        Err(e) => {
            reply_ephemeral(ctx, command, &format!("Erreur API : {e}")).await;
            return;
        }
    }

    // 3. Tirage tache + outcome (RNG scope ferme avant await).
    let (task_key, task_label, task_desc, success, coins_gain, flavor) = {
        let mut rng = rand::thread_rng();
        let task = TASKS.choose(&mut rng).copied().unwrap_or(TASKS[0]);
        let success = rng.gen_bool(SUCCESS_PCT);
        let coins = if success {
            rng.gen_range(COINS_MIN..=COINS_MAX)
        } else {
            0
        };
        let flavor = if success {
            SUCCESS_FLAVORS.choose(&mut rng).copied().unwrap_or("")
        } else {
            FAIL_FLAVORS.choose(&mut rng).copied().unwrap_or("")
        };
        (task.0, task.1, task.2, success, coins, flavor)
    };

    // 4. Credit + XP si succes.
    if success && coins_gain > 0 {
        if let Err(e) = api.update_player_coins(&guild_id, &user_id, coins_gain).await {
            tracing::warn!(error = %e, "Echec update_player_coins travaux");
        }
        if let Err(e) = api.add_xp(&guild_id, &user_id, XP_PER_TASK).await {
            tracing::warn!(error = %e, "Echec add_xp travaux");
        }
    }

    // 5. Pose le cooldown 2h (meme en cas d echec — pas de spam).
    if let Err(e) = api
        .set_cooldown(&guild_id, &user_id, TRAVAUX_COOLDOWN_KEY, TRAVAUX_COOLDOWN_SECS)
        .await
    {
        tracing::warn!(error = %e, "Echec set_cooldown travaux");
    }

    // 6. Embed.
    let title = if success {
        format!("\u{2705} {} — Reussi !", task_label)
    } else {
        format!("\u{274c} {} — Echec.", task_label)
    };
    let body = if success {
        format!(
            "_{}_\n\n{}\n\n\u{1f4b0} **+{}c** + **{} XP**.\nProchaine tache dans 2h.",
            task_desc, flavor, coins_gain, XP_PER_TASK
        )
    } else {
        format!(
            "_{}_\n\n{}\n\nProchaine tache dans 2h.",
            task_desc, flavor
        )
    };
    let _ = task_key;
    let embed = CreateEmbed::new()
        .title(title)
        .description(body)
        .color(if success { 0x2ECC71 } else { 0x95A5A6 })
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
