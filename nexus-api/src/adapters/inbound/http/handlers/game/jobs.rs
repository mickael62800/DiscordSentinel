//! Endpoints INTERNES utilises par le worker game-portal.
//!
//! Securite : seuls le Bearer global NEXUS_API_KEY protege ces endpoints
//! (le worker est un processus de confiance qui partage la meme cle).

use axum::extract::State;
use axum::Json;

use crate::adapters::inbound::http::handlers::ApiError;
use crate::bootstrap::AppState;

use nexus_core::application::game::worker_jobs::{
    run_daily_ping, run_health_check, run_idle_shutdown, run_image_cleanup, run_reconciler,
    run_reveal_ip, JobContext, JobReport,
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
        events: state.events.clone(),
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

pub async fn job_reveal_ip(State(state): State<AppState>) -> Result<Json<JobReport>, ApiError> {
    Ok(Json(run_reveal_ip(&ctx(&state)).await?))
}

pub async fn job_daily_ping(State(state): State<AppState>) -> Result<Json<JobReport>, ApiError> {
    Ok(Json(run_daily_ping(&ctx(&state)).await?))
}
