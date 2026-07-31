//! Handlers HTTP Game Portal — version nexus.
//!
//! Difference avec sentinel-api : pas de RBAC/component-gates ici, la seule
//! auth est le Bearer global NEXUS_API_KEY (middleware require_api_key).
//! L'identite de l'acteur (audit) vient du payload/query (`actor_id`),
//! comme pour les handlers wallet.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use uuid::Uuid;

use crate::adapters::inbound::http::dto::game::servers::{
    CreateGameServerDto, GameServerDetailDto, GameServerDto, GameServerStatsDto, RconCommandDto,
    RconCommandResponseDto, UpdateConfigDto,
};
use crate::adapters::inbound::http::handlers::ApiError;
use crate::bootstrap::AppState;
use nexus_core::domain::entities::game::server::CreateGameServerCommand;
use nexus_core::ports::outbound::events::game_events::{
    SERVER_DELETED, SERVER_STARTED, SERVER_STOPPED,
};

/// POST /api/games/{guild_id}/servers
pub async fn create_server(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
    Json(dto): Json<CreateGameServerDto>,
) -> Result<(StatusCode, Json<GameServerDto>), ApiError> {
    let cmd = CreateGameServerCommand {
        guild_id: guild_id.clone(),
        template_slug: dto.template_slug,
        name: dto.name,
        allocated_memory_mb: dto.memory_mb,
        cpu_limit: dto.cpu_limit,
        owner_user_id: dto.owner_user_id,
        initial_config: dto.config,
    };
    let server = state.game_servers_uc.create(cmd).await?;

    // Programme la revelation d'IP : delai fourni, sinon defaut de la guild.
    // 0 jour = pas de revelation programmee.
    let default_days = nexus_core::domain::entities::system::bot_config::cfg_i64(
        &state
            .bot_config_repo
            .get_config(&guild_id, super::GAME_PORTAL_BOT)
            .await
            .unwrap_or_default(),
        "ip_reveal_default_days",
        7,
    ) as i32;
    let days = dto.ip_reveal_days.unwrap_or(default_days).max(0);
    if days > 0 {
        let at = chrono::Utc::now() + chrono::Duration::days(i64::from(days));
        let _ = state
            .game_server_repo
            .set_ip_reveal_at(server.id, Some(at))
            .await;
    }

    Ok((StatusCode::CREATED, Json(server.into())))
}

/// GET /api/games/{guild_id}/servers
pub async fn list_servers(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
) -> Result<Json<Vec<GameServerDto>>, ApiError> {
    let list = state.game_servers_uc.list_for_guild(&guild_id).await?;
    Ok(Json(list.into_iter().map(GameServerDto::from).collect()))
}

/// GET /api/games/servers/{server_id}
pub async fn get_server(
    State(state): State<AppState>,
    Path(server_id): Path<Uuid>,
) -> Result<Json<GameServerDetailDto>, ApiError> {
    let detail = state.game_servers_uc.get(server_id).await?;
    Ok(Json(detail.into()))
}

#[derive(Debug, Deserialize)]
pub struct ActorQuery {
    /// Discord user id de l'acteur (audit). Si absent, fallback sur l'owner.
    pub actor_id: Option<String>,
}

/// Resout l'acteur pour l'audit : actor_id explicite sinon owner du serveur.
async fn resolve_actor(
    state: &AppState,
    server_id: Uuid,
    explicit: Option<&str>,
) -> Result<String, ApiError> {
    if let Some(s) = explicit {
        return Ok(s.to_string());
    }
    let detail = state.game_servers_uc.get(server_id).await?;
    Ok(detail.server.owner_user_id)
}

/// Publie un evenement de cycle de vie serveur a destination du bot.
/// `guild_id` est lu avant l'action pour rester disponible apres un delete.
async fn publish_lifecycle(state: &AppState, event: &str, server_id: Uuid, guild_id: &str) {
    state
        .events
        .publish(
            event,
            serde_json::json!({
                "server_id": server_id.to_string(),
                "guild_id": guild_id,
            }),
        )
        .await;
}

/// POST /api/games/servers/{server_id}/start
pub async fn start_server(
    State(state): State<AppState>,
    Path(server_id): Path<Uuid>,
    Query(q): Query<ActorQuery>,
) -> Result<StatusCode, ApiError> {
    let detail = state.game_servers_uc.get(server_id).await?;
    let actor = q
        .actor_id
        .clone()
        .unwrap_or_else(|| detail.server.owner_user_id.clone());
    state.game_servers_uc.start(server_id, &actor).await?;
    publish_lifecycle(&state, SERVER_STARTED, server_id, &detail.server.guild_id).await;
    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/games/servers/{server_id}/stop
pub async fn stop_server(
    State(state): State<AppState>,
    Path(server_id): Path<Uuid>,
    Query(q): Query<ActorQuery>,
) -> Result<StatusCode, ApiError> {
    let detail = state.game_servers_uc.get(server_id).await?;
    let actor = q
        .actor_id
        .clone()
        .unwrap_or_else(|| detail.server.owner_user_id.clone());
    state.game_servers_uc.stop(server_id, &actor).await?;
    publish_lifecycle(&state, SERVER_STOPPED, server_id, &detail.server.guild_id).await;
    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/games/servers/{server_id}/restart
pub async fn restart_server(
    State(state): State<AppState>,
    Path(server_id): Path<Uuid>,
    Query(q): Query<ActorQuery>,
) -> Result<StatusCode, ApiError> {
    let actor = resolve_actor(&state, server_id, q.actor_id.as_deref()).await?;
    state.game_servers_uc.restart(server_id, &actor).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// DELETE /api/games/servers/{server_id}
pub async fn delete_server(
    State(state): State<AppState>,
    Path(server_id): Path<Uuid>,
    Query(q): Query<ActorQuery>,
) -> Result<StatusCode, ApiError> {
    let detail = state.game_servers_uc.get(server_id).await?;
    let actor = q
        .actor_id
        .clone()
        .unwrap_or_else(|| detail.server.owner_user_id.clone());
    state.game_servers_uc.delete(server_id, &actor).await?;
    publish_lifecycle(&state, SERVER_DELETED, server_id, &detail.server.guild_id).await;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
pub struct LogsQuery {
    pub lines: Option<u32>,
}

/// GET /api/games/servers/{server_id}/logs?lines=200
pub async fn get_logs(
    State(state): State<AppState>,
    Path(server_id): Path<Uuid>,
    Query(q): Query<LogsQuery>,
) -> Result<Json<Vec<String>>, ApiError> {
    let lines = q.lines.unwrap_or(200).min(1000);
    let logs = state.game_servers_uc.get_logs(server_id, lines).await?;
    Ok(Json(logs))
}

/// GET /api/games/servers/{server_id}/stats
pub async fn get_stats(
    State(state): State<AppState>,
    Path(server_id): Path<Uuid>,
) -> Result<Json<GameServerStatsDto>, ApiError> {
    let stats = state.game_servers_uc.get_stats(server_id).await?;
    Ok(Json(stats.into()))
}

/// PUT /api/games/servers/{server_id}/config
pub async fn update_config(
    State(state): State<AppState>,
    Path(server_id): Path<Uuid>,
    Query(q): Query<ActorQuery>,
    Json(dto): Json<UpdateConfigDto>,
) -> Result<StatusCode, ApiError> {
    let actor = resolve_actor(&state, server_id, q.actor_id.as_deref()).await?;
    state
        .game_servers_uc
        .update_config(server_id, dto.config, &actor)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/games/servers/{server_id}/command
pub async fn execute_rcon(
    State(state): State<AppState>,
    Path(server_id): Path<Uuid>,
    Query(q): Query<ActorQuery>,
    Json(dto): Json<RconCommandDto>,
) -> Result<Json<RconCommandResponseDto>, ApiError> {
    let actor = resolve_actor(&state, server_id, q.actor_id.as_deref()).await?;
    let resp = state
        .game_servers_uc
        .execute_rcon(server_id, &dto.command, &actor)
        .await?;
    Ok(Json(RconCommandResponseDto { response: resp }))
}
