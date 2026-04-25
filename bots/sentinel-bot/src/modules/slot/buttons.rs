//! Handlers des clics sur le panel slot (Tirer / Daily).

use std::sync::Arc;

use serenity::all::{
    ComponentInteraction, Context, CreateInteractionResponse, CreateInteractionResponseMessage,
};
use tracing::warn;

use sentinel_shared::api_client::BaseApiClient;
use sentinel_shared::heartbeat::ApiClientKey;

use super::api_client::{self, DailyRequest, SpinRequest};
use super::embeds;

/// Mise par defaut quand le user clique "Tirer" sans choisir de mise.
/// Lue depuis la config (cle "default_bet"), fallback 50.
const FALLBACK_DEFAULT_BET: i64 = 50;

pub async fn handle_spin(ctx: &Context, component: &ComponentInteraction) {
    let guild_id = match component.guild_id {
        Some(id) => id.to_string(),
        None => return,
    };
    let user_id = component.user.id.to_string();
    let username = component.user.name.clone();

    // Defer ephemeral pour pouvoir prendre du temps (round-trip API).
    if let Err(e) = component
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Defer(
                CreateInteractionResponseMessage::new().ephemeral(true),
            ),
        )
        .await
    {
        warn!(error = %e, "Echec defer slot_spin");
        return;
    }

    let base = match get_api_client(ctx).await {
        Some(c) => c,
        None => return,
    };

    let mise = read_default_bet(&base, &guild_id).await;

    let req = SpinRequest { user_id, username: username.clone(), mise };

    let response = match api_client::spin(&base, &guild_id, &req).await {
        Ok(r) => r,
        Err(e) => {
            let msg = humanize_api_error(&e);
            edit_with_error(ctx, component, &msg).await;
            return;
        }
    };

    let embed = embeds::build_spin_result_embed(&response, &username);
    let edit = serenity::builder::EditInteractionResponse::new().embed(embed);
    if let Err(e) = component.edit_response(&ctx.http, edit).await {
        warn!(error = %e, "Echec edit reponse spin");
    }
}

pub async fn handle_daily(ctx: &Context, component: &ComponentInteraction) {
    let guild_id = match component.guild_id {
        Some(id) => id.to_string(),
        None => return,
    };
    let user_id = component.user.id.to_string();
    let username = component.user.name.clone();

    if let Err(e) = component
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Defer(
                CreateInteractionResponseMessage::new().ephemeral(true),
            ),
        )
        .await
    {
        warn!(error = %e, "Echec defer slot_daily");
        return;
    }

    let base = match get_api_client(ctx).await {
        Some(c) => c,
        None => return,
    };

    let req = DailyRequest { user_id, username: username.clone() };

    let response = match api_client::daily(&base, &guild_id, &req).await {
        Ok(r) => r,
        Err(e) => {
            let msg = humanize_api_error(&e);
            edit_with_error(ctx, component, &msg).await;
            return;
        }
    };

    let embed = embeds::build_spin_result_embed(&response, &username);
    let edit = serenity::builder::EditInteractionResponse::new().embed(embed);
    if let Err(e) = component.edit_response(&ctx.http, edit).await {
        warn!(error = %e, "Echec edit reponse daily");
    }
}

async fn get_api_client(ctx: &Context) -> Option<Arc<BaseApiClient>> {
    let data = ctx.data.read().await;
    data.get::<ApiClientKey>().map(Arc::clone)
}

async fn read_default_bet(base: &BaseApiClient, guild_id: &str) -> i64 {
    base.get_guild_config_for(guild_id, super::MODULE_BOT_NAME)
        .await
        .ok()
        .and_then(|cfg| cfg.get("default_bet").and_then(|v| v.parse::<i64>().ok()))
        .unwrap_or(FALLBACK_DEFAULT_BET)
}

async fn edit_with_error(ctx: &Context, component: &ComponentInteraction, message: &str) {
    let embed = embeds::build_error_embed(message);
    let edit = serenity::builder::EditInteractionResponse::new().embed(embed);
    if let Err(e) = component.edit_response(&ctx.http, edit).await {
        warn!(error = %e, "Echec edit error response slot");
    }
}

/// Convertit une erreur de l API en message lisible pour l utilisateur.
/// Les ValidationError remontent avec un "ValidationError(...)" qu on nettoie.
pub(crate) fn humanize_api_error(raw: &str) -> String {
    // L API serialise DomainError::ValidationError(msg) en HTTP 400 avec body
    // contenant le msg. BaseApiClient le rebalance dans la chaine d erreur.
    if let Some(start) = raw.find("Cooldown") {
        return raw[start..].split('"').next().unwrap_or(raw).to_string();
    }
    if let Some(start) = raw.find("Mise hors borne") {
        return raw[start..].split('"').next().unwrap_or(raw).to_string();
    }
    if raw.contains("Solde insuffisant") {
        return "Solde insuffisant pour cette mise.".to_string();
    }
    if raw.contains("desactive") {
        return "Daily bonus desactive sur ce serveur.".to_string();
    }
    if raw.contains("deja reclame") {
        return "Tu as deja reclame ton daily bonus aujourd hui.".to_string();
    }
    if raw.contains("Config slot-bot invalide") {
        return "Configuration slot invalide. Contacte un admin.".to_string();
    }
    "Erreur lors du spin. Reessaie dans quelques instants.".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn humanizes_cooldown_error() {
        let raw = r#"Erreur API 400 POST /api/slot/g/spin: {"error":"Cooldown actif : encore 3 secondes"}"#;
        let msg = humanize_api_error(raw);
        assert!(msg.contains("Cooldown"));
        assert!(msg.contains("3 secondes"));
    }

    #[test]
    fn humanizes_mise_out_of_range() {
        let raw = r#"Erreur API 400: {"error":"Mise hors borne (autorise : 10 - 1000)"}"#;
        let msg = humanize_api_error(raw);
        assert!(msg.contains("Mise hors borne"));
        assert!(msg.contains("10"));
        assert!(msg.contains("1000"));
    }

    #[test]
    fn humanizes_insufficient_balance() {
        let raw = r#"Erreur API 400: {"error":"ValidationError(\"Solde insuffisant: tu as 50 coins\")"}"#;
        let msg = humanize_api_error(raw);
        assert!(msg.contains("Solde insuffisant"));
    }

    #[test]
    fn humanizes_daily_disabled() {
        let raw = r#"Erreur API 400: {"error":"Daily bonus desactive sur ce serveur"}"#;
        let msg = humanize_api_error(raw);
        assert!(msg.contains("Daily bonus") && msg.contains("desactive"));
    }

    #[test]
    fn humanizes_daily_already_claimed() {
        let raw = r#"Erreur API 400: {"error":"Daily bonus deja reclame aujourd hui"}"#;
        let msg = humanize_api_error(raw);
        assert!(msg.contains("deja reclame"));
    }

    #[test]
    fn humanizes_unknown_error_to_generic() {
        let raw = "Erreur API 500: timeout";
        let msg = humanize_api_error(raw);
        assert!(msg.contains("Erreur") && msg.contains("Reessaie"));
    }
}
