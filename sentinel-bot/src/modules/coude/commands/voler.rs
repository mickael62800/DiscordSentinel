use serenity::all::{
    ButtonStyle, CommandDataOptionValue, CommandInteraction, CommandOptionType,
    ComponentInteraction, Context, CreateActionRow, CreateButton, CreateCommand,
    CreateCommandOption, CreateEmbed, CreateEmbedFooter, CreateInteractionResponse,
    CreateInteractionResponseMessage, CreateMessage,
};
use uuid::Uuid;

use crate::shared::discord_helpers::{reply_ephemeral, require_guild_id};

use crate::modules::coude::api_client::ApiClient;
use crate::modules::coude::load_guild_config;
use crate::modules::coude::GameApiKey;

pub const STEAL_DEFEND_PREFIX: &str = "steal_defend:";

pub fn register() -> CreateCommand {
    CreateCommand::new("voler")
        .description("Tente de pickpocket un joueur !")
        .add_option(
            CreateCommandOption::new(CommandOptionType::User, "cible", "Le joueur a voler")
                .required(true),
        )
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    let Some(guild_id) = require_guild_id(ctx, command).await else {
        return;
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
    if !crate::modules::coude::channel_check::check_channel(
        ctx,
        command,
        config.channel_activites(),
    )
    .await
    {
        return;
    }
    if !config.steal_enabled() {
        reply_ephemeral(ctx, command, "Le vol est desactive sur ce serveur.").await;
        return;
    }

    // Defer ephemeral : /voler enchaine 5 appels API (count_steal, cooldown,
    // get player x2, set_cooldown) avant de repondre. Sans defer, Discord
    // coupait l'interaction a 3s.
    if !crate::modules::coude::interaction_helper::defer_ephemeral(ctx, command).await {
        return;
    }

    let data = ctx.data.read().await;
    let api = data.get::<GameApiKey>().unwrap();

    // Verifier la limite quotidienne de vols
    let max_daily = config.steal_max_daily();
    if max_daily > 0 {
        let today_count = api
            .count_steal_today(&guild_id, &thief_id)
            .await
            .unwrap_or(0);
        if today_count >= max_daily {
            crate::modules::coude::interaction_helper::followup_text(
                ctx,
                command,
                &format!("Tu as atteint la limite de {} vols par jour !", max_daily),
            )
            .await;
            return;
        }
    }

    // Verifier le cooldown (30 min)
    match api.check_cooldown(&guild_id, &thief_id, "voler").await {
        Ok(Some(expires_at_str)) => {
            let expires = chrono::DateTime::parse_from_rfc3339(&expires_at_str)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now());
            let remaining = expires
                .signed_duration_since(chrono::Utc::now())
                .num_seconds();
            if remaining > 0 {
                let mins = remaining / 60;
                let secs = remaining % 60;
                crate::modules::coude::interaction_helper::followup_text(
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
            crate::modules::coude::interaction_helper::followup_text(ctx, command, &e).await;
            return;
        }
    }

    // Creer/recuperer les joueurs
    let _thief_player = match api
        .get_or_create_player(&guild_id, &thief_id, &command.user.name)
        .await
    {
        Ok(p) => p,
        Err(e) => {
            crate::modules::coude::interaction_helper::followup_text(ctx, command, &e).await;
            return;
        }
    };

    let target_user = match target_id.to_user(&ctx.http).await {
        Ok(u) => u,
        Err(_) => {
            crate::modules::coude::interaction_helper::followup_text(
                ctx,
                command,
                "Utilisateur introuvable.",
            )
            .await;
            return;
        }
    };

    if target_user.bot {
        crate::modules::coude::interaction_helper::followup_text(
            ctx,
            command,
            "Tu ne peux pas voler un bot !",
        )
        .await;
        return;
    }

    let target_player = match api
        .get_or_create_player(&guild_id, &target_id_str, &target_user.name)
        .await
    {
        Ok(p) => p,
        Err(e) => {
            crate::modules::coude::interaction_helper::followup_text(ctx, command, &e).await;
            return;
        }
    };

    let min_target_coins = config.voler_min_target_coins();
    if target_player.coins < min_target_coins {
        crate::modules::coude::interaction_helper::followup_text(
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

    // Poser le cooldown (30 min = 1800s)
    if let Err(e) = api
        .set_cooldown(&guild_id, &thief_id, "voler", config.steal_cooldown_secs())
        .await
    {
        crate::modules::coude::interaction_helper::followup_text(ctx, command, &e).await;
        return;
    }

    // Envoyer l'alerte publique (voleur anonyme) avec bouton de defense.
    // Phase 5 : on genere un attempt_id UUID client-side qu'on inclut
    // dans le custom_id (5 segments au lieu de 4) pour pouvoir PATCH
    // /defend facilement quand le bouton est clique.
    let attempt_id = Uuid::new_v4();
    // Discord limite custom_id a 100 chars. Format Phase 5 (UUID dashes
    // + 3 snowflakes 19c + 4 ":") = ~110c -> overflow. Solution : on
    // strip les dashes du UUID (32c au lieu de 36) et on omet guild_id
    // (component.guild_id est dispo dans handle_defend). Format reduit :
    //   steal_defend:{uuid_no_dash}:{thief_id}:{target_id}
    // = 13 + 32 + 19 + 19 + 3 = ~86c, sous la limite.
    let attempt_id_compact = attempt_id.simple().to_string();
    let custom_id = format!(
        "steal_defend:{}:{}:{}",
        attempt_id_compact, thief_id, target_id_str
    );

    let embed = CreateEmbed::new()
        .title("\u{26a0}\u{fe0f} Tentative de vol !")
        .description(format!(
            "\u{26a0}\u{fe0f} Quelqu'un tente de voler <@{}> !\n\n\
             <@{}>, tu as **60 secondes** pour te defendre !",
            target_id, target_id
        ))
        .color(0xFFA500)
        .footer(CreateEmbedFooter::new(
            "Coup de Coude | Sentinel — 60s pour reagir",
        ))
        .timestamp(serenity::model::Timestamp::now());

    let defend_btn = CreateButton::new(&custom_id)
        .label("Se defendre !")
        .emoji(serenity::model::channel::ReactionType::Unicode(
            "\u{1f6e1}\u{fe0f}".to_string(),
        ))
        .style(ButtonStyle::Primary);

    let row = CreateActionRow::Buttons(vec![defend_btn]);

    // Confirme au voleur via followup ephemeral (on a defer plus haut).
    crate::modules::coude::interaction_helper::followup_text(
        ctx,
        command,
        "\u{1f575}\u{fe0f} Tentative de vol lancee... Reste discret !",
    )
    .await;

    // Poster le message public avec le bouton dans le salon d'activites
    let activity_channel = config.channel_activites();
    let channel_id = match activity_channel.and_then(|id| id.parse::<u64>().ok()) {
        Some(ch_id) => serenity::model::id::ChannelId::new(ch_id),
        None => command.channel_id,
    };

    let alert_msg = match channel_id
        .send_message(
            &ctx.http,
            CreateMessage::new().embed(embed).components(vec![row]),
        )
        .await
    {
        Ok(msg) => msg,
        Err(e) => {
            tracing::warn!(error = %e, "Echec send_message alerte vol");
            return;
        }
    };

    // Phase 5 — persistance de l'attempt en DB. Le worker `expire_steals`
    // (sentinel-worker) scanne les pending expires et publie un event
    // Redis `coude:steal_expired` que le consumer
    // `steal_expired_events.rs` consomme pour declencher la resolution
    // AFK. Plus de tokio::spawn fragile qui meurt avec le bot.
    if let Err(e) = api
        .create_steal_attempt(
            attempt_id,
            &guild_id,
            &thief_id,
            &target_id_str,
            &alert_msg.id.to_string(),
            &channel_id.to_string(),
            60,
        )
        .await
    {
        tracing::warn!(error = %e, "Echec create_steal_attempt API — la resolution AFK ne se declenchera pas");
    }
}

/// Resout une tentative de vol. Adaptateur MINCE : l'issue (gagne/perdu/
/// bloque), le butin et la penalite sont desormais decides SERVEUR-SIDE
/// (`ResolveStealUseCase`, cf. `POST /api/coude/{guild}/steal/resolve`).
/// Le bot ne fait plus AUCUN calcul (bonus classe/DEF/boost, totals,
/// stolen, lost) : il rend l'embed pret a poster et dispatche les
/// railleries.
///
/// - `afk = true` → cible n'a pas clique "Se defendre" (malus defenseur +
///   plage de butin plus faible, applique cote serveur).
/// - `afk = false` → defense active.
pub(crate) async fn resolve_steal_attempt(
    api: &ApiClient,
    guild_id: &str,
    thief_id: &str,
    target_id: &str,
    afk: bool,
) -> (
    CreateEmbed,
    Vec<crate::modules::coude::api_client::TauntEvent>,
) {
    let resolved = match api.resolve_steal(guild_id, thief_id, target_id, afk).await {
        Ok(r) => r,
        Err(_) => {
            let embed = CreateEmbed::new()
                .title("\u{26a0}\u{fe0f} API indisponible")
                .description("Veuillez reessayer plus tard.")
                .color(0x95A5A6);
            return (embed, Vec::new());
        }
    };

    let embed = CreateEmbed::new()
        .title(resolved.title)
        .description(resolved.description)
        .color(resolved.color)
        .footer(CreateEmbedFooter::new(
            crate::shared::branding::COUDE_TAGLINE_SHORT,
        ))
        .timestamp(serenity::model::Timestamp::now());
    (embed, resolved.taunt_events)
}

/// Handle the defend button click from a steal attempt.
pub async fn handle_defend(ctx: &Context, component: &ComponentInteraction) {
    let parts: Vec<&str> = component.data.custom_id.split(':').collect();
    // Format Phase 5b (compact, custom_id < 100c) :
    //   steal_defend:{uuid_no_dash}:{thief_id}:{target_id}
    //   guild_id lu via component.guild_id.
    // Format Phase 5 (deprecated, > 100c) :
    //   steal_defend:{attempt_id}:{thief_id}:{target_id}:{guild_id}
    // Format legacy (avant Phase 5) :
    //   steal_defend:{thief_id}:{target_id}:{guild_id}
    let component_guild_id_str = component
        .guild_id
        .map(|g| g.to_string())
        .unwrap_or_default();
    // Detection format :
    //   - 4 parts + guild context + parts[1] est un UUID -> Phase 5b compact
    //   - 4 parts sans UUID -> legacy (sans attempt_id)
    //   - 5 parts -> Phase 5 deprecated (guild_id en dernier)
    let (attempt_id_opt, thief_id, target_id, guild_id): (Option<Uuid>, String, String, String) =
        match parts.len() {
            4 => {
                if let Ok(aid) = Uuid::parse_str(parts[1]) {
                    // Phase 5b compact : steal_defend:{uuid}:{thief}:{target}
                    if component_guild_id_str.is_empty() {
                        return;
                    }
                    (
                        Some(aid),
                        parts[2].to_string(),
                        parts[3].to_string(),
                        component_guild_id_str,
                    )
                } else {
                    // Legacy : steal_defend:{thief}:{target}:{guild}
                    (
                        None,
                        parts[1].to_string(),
                        parts[2].to_string(),
                        parts[3].to_string(),
                    )
                }
            }
            5 => {
                // Phase 5 deprecated : steal_defend:{uuid}:{thief}:{target}:{guild}
                let aid = Uuid::parse_str(parts[1]).ok();
                (
                    aid,
                    parts[2].to_string(),
                    parts[3].to_string(),
                    parts[4].to_string(),
                )
            }
            _ => return,
        };
    let thief_id = thief_id.as_str();
    let target_id = target_id.as_str();
    let guild_id = guild_id.as_str();

    // Only the target can click the defend button
    if component.user.id.to_string() != target_id {
        if let Err(e) = component
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content("Seule la victime peut se defendre !")
                        .ephemeral(true),
                ),
            )
            .await
        {
            tracing::warn!(error = %e, "Echec response Discord");
        }
        return;
    }

    // Defer en mode UPDATE_MESSAGE : on acquitte le bouton avant les 3s sans
    // afficher de loader au user ; on editera le message original a la fin.
    if let Err(e) = component
        .create_response(&ctx.http, CreateInteractionResponse::Acknowledge)
        .await
    {
        tracing::warn!(error = %e, "Echec defer handle_defend");
    }

    let data = ctx.data.read().await;
    let api = data.get::<GameApiKey>().unwrap();

    // Phase 5 — marque l'attempt 'defended' cote DB pour que le worker
    // ne le passe pas en 'expired' apres coup. Fire-and-forget : meme
    // si l'API rate, on continue la resolution active.
    if let Some(attempt_id) = attempt_id_opt {
        api.mark_steal_defended(attempt_id).await;
    }

    // Garantit l'existence des deux joueurs cote DB (la resolution
    // serveur relit leur solde/DEF/classe via `players_uc.get`). On ne se
    // sert plus des donnees ici : le calcul est 100% serveur-side.
    if let Err(e) = api.get_or_create_player(guild_id, thief_id, "").await {
        let _ = component
            .create_followup(
                &ctx.http,
                serenity::all::CreateInteractionResponseFollowup::new()
                    .content(e)
                    .ephemeral(true),
            )
            .await;
        return;
    }

    let _ = match api
        .get_or_create_player(guild_id, target_id, &component.user.name)
        .await
    {
        Ok(p) => p,
        Err(e) => {
            let _ = component
                .create_followup(
                    &ctx.http,
                    serenity::all::CreateInteractionResponseFollowup::new()
                        .content(e)
                        .ephemeral(true),
                )
                .await;
            return;
        }
    };

    // CLAIM atomique de la resolution : on ne transfere les coins QUE si CE clic
    // a gagne la course (contre le worker AFK ou un double-clic). Sinon le vol a
    // deja ete resolu -> on sort sans re-transferer (anti double-vol).
    if let Some(attempt_id) = attempt_id_opt {
        if !api.mark_steal_resolved(attempt_id).await {
            let _ = component
                .create_followup(
                    &ctx.http,
                    serenity::all::CreateInteractionResponseFollowup::new()
                        .content("Ce vol a déjà été résolu.")
                        .ephemeral(true),
                )
                .await;
            return;
        }
    }

    let (embed, taunt_events) = resolve_steal_attempt(
        api, guild_id, thief_id, target_id, false, // defense active
    )
    .await;

    // Apres Acknowledge (DEFERRED_UPDATE_MESSAGE), edit_response edite le
    // message d'origine (le challenge de vol) pour afficher le resultat et
    // retirer les boutons.
    let edit_result = component
        .edit_response(
            &ctx.http,
            serenity::all::EditInteractionResponse::new()
                .embed(embed.clone())
                .components(vec![]),
        )
        .await;

    if let Err(e) = edit_result {
        tracing::warn!(error = %e, thief_id, target_id, guild_id,
            "Echec edit_response Discord (defend vol) — fallback en followup");
        // Fallback : poste le resultat en followup public dans le salon
        // pour que le user voie quand meme le verdict de son roll, meme si
        // le message original n'est plus editable (supprime, expire, etc.).
        if let Err(e2) = component
            .create_followup(
                &ctx.http,
                serenity::all::CreateInteractionResponseFollowup::new().embed(embed),
            )
            .await
        {
            tracing::error!(error = %e2, thief_id, target_id, guild_id,
                "Echec followup defend vol — l utilisateur ne verra rien");
        }
    }

    // Drop le data guard avant le dispatch async (il lock TypeMap).
    drop(data);

    // Phase 9 Part D : dispatch IO pur.
    if !taunt_events.is_empty() {
        if let Ok(guild_id_u64) = guild_id.parse::<u64>() {
            let gid = serenity::all::GuildId::new(guild_id_u64);
            crate::modules::coude::taunts_dispatch::dispatch_all(ctx, gid, &taunt_events).await;
        }
    }
}
