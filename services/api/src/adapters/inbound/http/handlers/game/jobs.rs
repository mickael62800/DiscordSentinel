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
    run_health_check, run_idle_shutdown, run_image_cleanup, run_reconciler, JobContext,
    JobReport,
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

pub async fn job_health_check(
    State(state): State<AppState>,
) -> Result<Json<JobReport>, ApiError> {
    Ok(Json(run_health_check(&ctx(&state)).await?))
}

pub async fn job_idle_shutdown(
    State(state): State<AppState>,
) -> Result<Json<JobReport>, ApiError> {
    Ok(Json(run_idle_shutdown(&ctx(&state)).await?))
}

pub async fn job_reconcile(
    State(state): State<AppState>,
) -> Result<Json<JobReport>, ApiError> {
    Ok(Json(run_reconciler(&ctx(&state)).await?))
}

pub async fn job_image_cleanup(
    State(state): State<AppState>,
) -> Result<Json<JobReport>, ApiError> {
    Ok(Json(run_image_cleanup(&ctx(&state)).await?))
}

#[allow(dead_code)]
fn _force_arc<T>(_a: Arc<T>) {}
