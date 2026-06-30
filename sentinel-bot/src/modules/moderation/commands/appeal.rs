use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use serenity::all::{
    CommandInteraction, ComponentInteraction, Context, CreateCommand, CreateInteractionResponse,
    CreateInteractionResponseMessage,
};
use tracing::{error, info, warn};

use crate::shared::discord_helpers::reply_ephemeral;
use crate::shared::heartbeat::ApiClientKey;

pub const APPEAL_PREFIX: &str = "sentinel_mod_appeal_";

/// MOD #9 — fenetre anti-spam de `/appeal` par (guild, user).
const APPEAL_COOLDOWN: Duration = Duration::from_secs(5 * 60);

/// Garde in-process (guild_id, user_id) -> dernier appel accepte.
fn appeal_cooldowns() -> &'static Mutex<HashMap<(String, String), Instant>> {
    static MAP: OnceLock<Mutex<HashMap<(String, String), Instant>>> = OnceLock::new();
    MAP.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Renvoie `true` si l'utilisateur est encore en cooldown (appel a ignorer).
/// Purge au passage les entrees expirees pour borner la memoire.
fn appeal_on_cooldown(guild_id: &str, user_id: &str) -> bool {
    let mut map = appeal_cooldowns().lock().unwrap();
    let now = Instant::now();
    map.retain(|_, last| now.duration_since(*last) < APPEAL_COOLDOWN);
    map.contains_key(&(guild_id.to_string(), user_id.to_string()))
}

/// Enregistre un appel accepte pour demarrer le cooldown.
fn record_appeal(guild_id: &str, user_id: &str) {
    appeal_cooldowns()
        .lock()
        .unwrap()
        .insert((guild_id.to_string(), user_id.to_string()), Instant::now());
}

pub fn register() -> CreateCommand {
    CreateCommand::new("appeal")
        .description("Contester une sanction recue (cree un ticket automatiquement)")
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    let guild_id = match command.guild_id {
        Some(id) => id,
        None => {
            reply_ephemeral(ctx, command, "Commande serveur uniquement.").await;
            return;
        }
    };

    let user_id = command.user.id.to_string();

    // MOD #9 (b) — anti-spam : cooldown in-process par (guild, user).
    if appeal_on_cooldown(&guild_id.to_string(), &user_id) {
        reply_ephemeral(
            ctx,
            command,
            "Vous avez deja soumis un appel recemment. Patientez quelques minutes avant de reessayer.",
        )
        .await;
        return;
    }

    let data = ctx.data.read().await;
    let base = match data.get::<ApiClientKey>() {
        Some(b) => b,
        None => {
            reply_ephemeral(ctx, command, "Erreur interne.").await;
            return;
        }
    };

    // MOD #9 (a) — verifier que l'appelant a bien une sanction a contester.
    // En cas d'erreur reseau on reste permissif (on n'empeche pas un appel
    // legitime), mais une absence confirmee de sanction stoppe la creation.
    match base
        .get_json::<serde_json::Value>(&format!("/api/moderation/history/{}/{}", guild_id, user_id))
        .await
    {
        Ok(history) => {
            let has_sanction = history
                .get("actions")
                .and_then(|a| a.as_array())
                .map(|a| !a.is_empty())
                .unwrap_or(false);
            if !has_sanction {
                reply_ephemeral(
                    ctx,
                    command,
                    "Aucune sanction a contester n'a ete trouvee a votre encontre sur ce serveur.",
                )
                .await;
                return;
            }
        }
        Err(e) => {
            warn!(error = %e, "Verification sanction /appeal echouee, on laisse passer");
        }
    }

    let req = base
        .client()
        .post(format!("{}/api/tickets", base.base_url()))
        .json(&serde_json::json!({
            "title": format!("Appel de sanction — {}", command.user.name),
            "priority": "medium",
            "author_id": command.user.id.to_string(),
            "author_name": command.user.name,
            "server": guild_id.to_string(),
            "category": "appel_sanction",
            "ticket_type": "appel_sanction",
        }));

    match base.auth(req).send().await {
        Ok(resp) if resp.status().is_success() => {
            record_appeal(&guild_id.to_string(), &user_id);
            reply_ephemeral(
                ctx,
                command,
                "Votre appel de sanction a ete enregistre. Un ticket a ete cree et un moderateur senior va l'examiner.",
            ).await;
            info!(user = %command.user.name, "Appel de sanction cree via /appeal");
        }
        Ok(resp) => {
            let status = resp.status();
            error!(status = %status, "Erreur creation ticket appel");
            reply_ephemeral(
                ctx,
                command,
                "Erreur lors de la creation de l'appel. Reessayez plus tard.",
            )
            .await;
        }
        Err(e) => {
            error!(error = %e, "Erreur reseau creation ticket appel");
            reply_ephemeral(ctx, command, "Erreur reseau. Reessayez plus tard.").await;
        }
    }
}

pub async fn handle_appeal_button(ctx: &Context, component: &ComponentInteraction) {
    // custom_id format : `sentinel_mod_appeal_{guild_id}_{action_id}`.
    // Le guild_id est numerique (pas d'underscore) et l'action_id est un UUID
    // (tirets, pas d'underscore) -> split_once('_') est sans ambiguite.
    let payload = match component.data.custom_id.strip_prefix(APPEAL_PREFIX) {
        Some(p) => p,
        None => return,
    };
    let (found_guild, action_id) = match payload.split_once('_') {
        Some((g, a)) => (g.to_string(), a),
        // Compat : ancien format sans guild_id embarque -> on ne devine plus le
        // serveur (source du bug multi-guild), on demande /appeal explicite.
        None => {
            let response = CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content(
                        "Bouton d'appel obsolete. Utilisez `/appeal` dans le serveur concerne.",
                    )
                    .ephemeral(true),
            );
            if let Err(e) = component.create_response(&ctx.http, response).await {
                warn!(error = %e, "Failed to send appeal legacy-button response");
            }
            return;
        }
    };

    let data = ctx.data.read().await;
    let base = match data.get::<ApiClientKey>() {
        Some(b) => b,
        None => return,
    };

    let req = base
        .client()
        .post(format!("{}/api/tickets", base.base_url()))
        .json(&serde_json::json!({
            "title": format!("Appel de sanction — {} (action: {})", component.user.name, &action_id[..8.min(action_id.len())]),
            "priority": "medium",
            "author_id": component.user.id.to_string(),
            "author_name": component.user.name,
            "server": found_guild,
            "category": "appel_sanction",
            "ticket_type": "appel_sanction",
        }));

    match base.auth(req).send().await {
        Ok(resp) if resp.status().is_success() => {
            let response = CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content("Votre appel a ete enregistre. Un ticket a ete cree et un moderateur senior va l'examiner.")
                    .ephemeral(true),
            );
            if let Err(e) = component.create_response(&ctx.http, response).await {
                warn!(error = %e, "Failed to send appeal success response");
            }
            info!(user = %component.user.name, action_id = action_id, "Appel de sanction cree via bouton DM");

            // Notifie le salon d'appels (si configure) que le ticket a ete
            // cree -- permet aux mods de voir l'appel directement dans
            // Discord sans surveiller la dashboard tickets.
            let notif_embed = crate::shared::embeds::info_embed("Nouvel appel de sanction")
                .description(format!(
                    "<@{}> conteste sa sanction.\n**Action ID :** `{}`\nUn ticket a ete cree (categorie `appel_sanction`).",
                    component.user.id,
                    &action_id[..16.min(action_id.len())],
                ))
                .timestamp(serenity::model::Timestamp::now());
            crate::modules::moderation::post_to_appeal_channel(ctx, &found_guild, notif_embed)
                .await;
        }
        _ => {
            let response = CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content("Erreur lors de la creation de l'appel. Utilisez `/appeal` dans le serveur.")
                    .ephemeral(true),
            );
            if let Err(e) = component.create_response(&ctx.http, response).await {
                warn!(error = %e, "Failed to send appeal error response");
            }
        }
    }
}

/// Construit la ligne de bouton "Contester" attachee aux DM de sanction.
///
/// Le `guild_id` est embarque dans le custom_id pour router l'appel vers le BON
/// serveur (fix multi-guild : on ne devine plus via le cache). Format :
/// `sentinel_mod_appeal_{guild_id}_{action_id}`.
pub fn build_appeal_button(guild_id: &str, action_id: &str) -> serenity::builder::CreateActionRow {
    let button = serenity::builder::CreateButton::new(format!(
        "{}{}_{}",
        APPEAL_PREFIX, guild_id, action_id
    ))
    .label("Contester cette sanction")
    .style(serenity::all::ButtonStyle::Secondary);

    serenity::builder::CreateActionRow::Buttons(vec![button])
}
