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

    // Ticket dashboard (best-effort, pour le suivi cote web).
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
    if let Err(e) = base.auth(req).send().await {
        warn!(error = %e, "Ticket appel (dashboard) non cree — on continue");
    }

    record_appeal(&guild_id.to_string(), &user_id);
    finalize_appeal(
        ctx,
        &guild_id.to_string(),
        command.user.id.get(),
        &command.user.name,
        None,
        |content| {
            let ctx = ctx.clone();
            async move {
                reply_ephemeral(&ctx, command, &content).await;
            }
        },
    )
    .await;
    info!(user = %command.user.name, "Appel de sanction traite via /appeal");
}

/// Cree le salon d'appel (si categorie configuree) + notifie ; puis renvoie le
/// message a afficher a l'appelant via `reply`.
async fn finalize_appeal<F, Fut>(
    ctx: &Context,
    guild_id: &str,
    appellant_id: u64,
    appellant_name: &str,
    action_id: Option<&str>,
    reply: F,
) where
    F: FnOnce(String) -> Fut,
    Fut: std::future::Future<Output = ()>,
{
    let mut desc = format!("<@{appellant_id}> conteste une sanction et demande un réexamen.");
    if let Some(a) = action_id {
        desc.push_str(&format!("\n**Action :** `{}`", &a[..16.min(a.len())]));
    }
    let intro = crate::shared::embeds::info_embed("📨 Appel de sanction")
        .description(desc)
        .timestamp(serenity::model::Timestamp::now());

    // 1) Salon dedie sous la categorie (si configuree).
    if let Some(channel) = crate::modules::moderation::create_appeal_channel(
        ctx,
        guild_id,
        appellant_id,
        appellant_name,
        intro,
    )
    .await
    {
        reply(format!(
            "✅ Ton appel est ouvert : <#{channel}>. Un modérateur va l'examiner."
        ))
        .await;
        return;
    }

    // 2) Fallback : notification dans le salon d'appels configure.
    let notif = crate::shared::embeds::info_embed("📨 Nouvel appel de sanction")
        .description(format!("<@{appellant_id}> conteste sa sanction."))
        .timestamp(serenity::model::Timestamp::now());
    crate::modules::moderation::post_to_appeal_channel(ctx, guild_id, notif).await;
    reply(
        "✅ Ton appel a été enregistré. Un modérateur senior va l'examiner.".to_string(),
    )
    .await;
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

    // Ticket dashboard (best-effort). On lit le client puis on relache le lock.
    {
        let data = ctx.data.read().await;
        if let Some(base) = data.get::<ApiClientKey>() {
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
            if let Err(e) = base.auth(req).send().await {
                warn!(error = %e, "Ticket appel (dashboard) non cree — on continue");
            }
        }
    }

    // Repond a l'interaction (differe : la creation du salon peut prendre du temps).
    let _ = component
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Defer(
                CreateInteractionResponseMessage::new().ephemeral(true),
            ),
        )
        .await;

    finalize_appeal(
        ctx,
        &found_guild,
        component.user.id.get(),
        &component.user.name,
        Some(action_id),
        |content| {
            let ctx = ctx.clone();
            async move {
                let _ = component
                    .create_followup(
                        &ctx.http,
                        serenity::builder::CreateInteractionResponseFollowup::new()
                            .content(content)
                            .ephemeral(true),
                    )
                    .await;
            }
        },
    )
    .await;
    info!(user = %component.user.name, action_id = action_id, "Appel de sanction traite via bouton DM");
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
