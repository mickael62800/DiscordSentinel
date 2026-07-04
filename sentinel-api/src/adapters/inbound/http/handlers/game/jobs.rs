//! Endpoints INTERNES utilisees par game-portal-worker.
//!
//! Securite : pas de RBAC user (le worker n'a pas de Discord token), seul
//! le auth_middleware (X-API-Key) protege ces endpoints. Le worker est un
//! processus de confiance qui partage la meme cle API.

use axum::extract::State;
use axum::Json;
use std::sync::Arc;

use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::state::AppState;
use crate::application::game::worker_jobs::{
    run_health_check, run_idle_shutdown, run_image_cleanup, run_reconciler, JobContext, JobReport,
};

fn ctx(state: &AppState) -> JobContext {
    JobContext {
        server_repo: state.game_server_repo.clone(),
        template_repo: state.game_template_repo.clone(),
        audit_repo: state.game_audit_repo.clone(),
        session_repo: state.game_session_repo.clone(),
        container_runtime: state.game_container_runtime.clone(),
        rcon_client: state.game_rcon_client.clone(),
        port_allocator: state.game_port_allocator.clone(),
        bot_config: state.bot_config_repo.clone(),
    }
}

pub async fn job_health_check(State(state): State<AppState>) -> Result<Json<JobReport>, ApiError> {
    Ok(Json(run_health_check(&ctx(&state)).await?))
}

pub async fn job_idle_shutdown(State(state): State<AppState>) -> Result<Json<JobReport>, ApiError> {
    Ok(Json(run_idle_shutdown(&ctx(&state)).await?))
}

pub async fn job_reconcile(State(state): State<AppState>) -> Result<Json<JobReport>, ApiError> {
    Ok(Json(run_reconciler(&ctx(&state)).await?))
}

pub async fn job_image_cleanup(State(state): State<AppState>) -> Result<Json<JobReport>, ApiError> {
    Ok(Json(run_image_cleanup(&ctx(&state)).await?))
}

/// Revelation d'IP : pour chaque session dont `ip_reveal_at` est atteint,
/// publie un event `game_ip_reveal` (le bot poste l'IP dans le salon) et
/// marque l'IP comme revelee (fire-once).
pub async fn job_reveal_ip(State(state): State<AppState>) -> Result<Json<JobReport>, ApiError> {
    let due = state.game_server_repo.list_ip_reveal_due().await?;
    let mut processed = 0usize;
    for s in &due {
        state.broadcaster.broadcast(
            "game_ip_reveal",
            serde_json::json!({ "server_id": s.id.to_string(), "guild_id": s.guild_id }),
        );
        state.game_server_repo.mark_ip_revealed(s.id).await?;
        processed += 1;
    }
    Ok(Json(JobReport {
        job: "reveal_ip",
        processed,
        errors: 0,
        details: serde_json::json!({}),
    }))
}

/// Ping quotidien : a l'heure configuree (session_daily_ping_hour, UTC), pour
/// chaque session en attente de revelation, publie `game_daily_ping` (le bot
/// ping le role dans le salon). Fire-once par jour via last_daily_ping_at.
pub async fn job_daily_ping(State(state): State<AppState>) -> Result<Json<JobReport>, ApiError> {
    use chrono::Timelike;
    let now_hour = chrono::Utc::now().hour() as i64;
    let awaiting = state
        .game_server_repo
        .list_awaiting_reveal_no_ping_today()
        .await?;
    let mut processed = 0usize;
    for s in &awaiting {
        let cfg = state
            .bot_config_repo
            .get_config(&s.guild_id, "game-portal")
            .await
            .unwrap_or_default();
        let get = |k: &str| cfg.iter().find(|c| c.config_key == k).map(|c| c.config_value.clone());
        let enabled = matches!(get("session_daily_ping_enabled").as_deref(), Some("true") | Some("1"));
        let hour = get("session_daily_ping_hour")
            .and_then(|v| v.parse::<i64>().ok())
            .unwrap_or(18);
        if enabled && hour == now_hour {
            state.broadcaster.broadcast(
                "game_daily_ping",
                serde_json::json!({ "server_id": s.id.to_string(), "guild_id": s.guild_id }),
            );
            state.game_server_repo.mark_daily_ping(s.id).await?;
            processed += 1;
        }
    }
    Ok(Json(JobReport {
        job: "daily_ping",
        processed,
        errors: 0,
        details: serde_json::json!({}),
    }))
}

#[allow(dead_code)]
fn _force_arc<T>(_a: Arc<T>) {}
