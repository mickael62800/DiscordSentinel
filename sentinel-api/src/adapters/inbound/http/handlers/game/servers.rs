//! Handlers HTTP Game Portal — gates RBAC via component_gates.

use crate::adapters::inbound::http::extractors::ValidatedGuild;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Extension;
use axum::Json;
use serde::Deserialize;
use uuid::Uuid;

use crate::adapters::inbound::http::dto::game::servers::{
    CreateGameServerDto, GameServerDetailDto, GameServerDto, GameServerStatsDto, RconCommandDto,
    RconCommandResponseDto, UpdateConfigDto,
};
use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::middleware::component_gates::check_component_role;
use crate::adapters::inbound::http::middleware::rbac::RoleContext;
use crate::adapters::inbound::http::state::AppState;
use sentinel_core::domain::entities::game::server::CreateGameServerCommand;

/// Helper : extrait le guild_id du serveur via DB et gate RBAC.
pub(super) async fn gate_server(
    state: &AppState,
    rbac: &Option<Extension<RoleContext>>,
    server_id: Uuid,
    component_key: &'static str,
    label: &'static str,
) -> Result<sentinel_core::domain::entities::game::server::GameServer, ApiError> {
    let server = state
        .game_servers_uc
        .get(server_id)
        .await
        .map_err(ApiError)?
        .server;
    check_component_role(state, rbac, &server.guild_id, component_key, label).await?;
    Ok(server)
}

/// POST /api/games/{guild_id}/servers
pub async fn create_server(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    ValidatedGuild { guild_id }: ValidatedGuild,
    Json(dto): Json<CreateGameServerDto>,
) -> Result<(StatusCode, Json<GameServerDto>), ApiError> {
    check_component_role(
        &state,
        &rbac,
        &guild_id,
        "game.server.create",
        "role insuffisant pour creer un serveur de jeu",
    )
    .await?;

    let cmd = CreateGameServerCommand {
        guild_id: guild_id.clone(),
        template_slug: dto.template_slug,
        name: dto.name,
        allocated_memory_mb: dto.memory_mb,
        owner_user_id: dto.owner_user_id,
        initial_config: dto.config,
    };
    let server = state.game_servers_uc.create(cmd).await?;

    // Programme la revelation d'IP : delai fourni, sinon defaut de la guild.
    // 0 jour = pas de revelation programmee.
    let default_days = state
        .bot_config_repo
        .get_config(&guild_id, "game-portal")
        .await
        .ok()
        .and_then(|cfg| {
            cfg.iter()
                .find(|c| c.config_key == "ip_reveal_default_days")
                .and_then(|c| c.config_value.parse::<i32>().ok())
        })
        .unwrap_or(7);
    let days = dto.ip_reveal_days.unwrap_or(default_days).max(0);
    if days > 0 {
        let at = chrono::Utc::now() + chrono::Duration::days(i64::from(days));
        let _ = state.game_server_repo.set_ip_reveal_at(server.id, Some(at)).await;
    }

    state.broadcaster.broadcast(
        "game_server_created",
        serde_json::json!({
            "guild_id": guild_id,
            "server_id": server.id,
        }),
    );

    Ok((StatusCode::CREATED, Json(server.into())))
}

/// GET /api/games/{guild_id}/servers
pub async fn list_servers(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    ValidatedGuild { guild_id }: ValidatedGuild,
) -> Result<Json<Vec<GameServerDto>>, ApiError> {
    check_component_role(
        &state,
        &rbac,
        &guild_id,
        "game.server.view",
        "role insuffisant pour lister les serveurs",
    )
    .await?;
    let list = state.game_servers_uc.list_for_guild(&guild_id).await?;
    Ok(Json(list.into_iter().map(GameServerDto::from).collect()))
}

/// GET /api/games/servers/{server_id}
pub async fn get_server(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Path(server_id): Path<Uuid>,
) -> Result<Json<GameServerDetailDto>, ApiError> {
    gate_server(
        &state,
        &rbac,
        server_id,
        "game.server.view",
        "role insuffisant pour consulter un serveur",
    )
    .await?;
    let detail = state.game_servers_uc.get(server_id).await?;
    Ok(Json(detail.into()))
}

#[derive(Debug, Deserialize)]
pub struct ActorQuery {
    /// Discord user id de l'acteur (audit). Si absent, fallback sur ctx RBAC.
    pub actor_id: Option<String>,
}

/// POST /api/games/servers/{server_id}/start
pub async fn start_server(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Path(server_id): Path<Uuid>,
    Query(q): Query<ActorQuery>,
) -> Result<StatusCode, ApiError> {
    let server = gate_server(
        &state,
        &rbac,
        server_id,
        "game.server.start_stop",
        "role insuffisant pour demarrer un serveur",
    )
    .await?;
    let actor = resolve_actor(&rbac, q.actor_id.as_deref(), &server.owner_user_id);
    state.game_servers_uc.start(server_id, &actor).await?;
    state.broadcaster.broadcast(
        "game_server_started",
        serde_json::json!({"server_id": server_id, "guild_id": server.guild_id}),
    );
    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/games/servers/{server_id}/stop
pub async fn stop_server(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Path(server_id): Path<Uuid>,
    Query(q): Query<ActorQuery>,
) -> Result<StatusCode, ApiError> {
    let server = gate_server(
        &state,
        &rbac,
        server_id,
        "game.server.start_stop",
        "role insuffisant pour arreter un serveur",
    )
    .await?;
    let actor = resolve_actor(&rbac, q.actor_id.as_deref(), &server.owner_user_id);
    state.game_servers_uc.stop(server_id, &actor).await?;
    state.broadcaster.broadcast(
        "game_server_stopped",
        serde_json::json!({"server_id": server_id, "guild_id": server.guild_id}),
    );
    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/games/servers/{server_id}/restart
pub async fn restart_server(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Path(server_id): Path<Uuid>,
    Query(q): Query<ActorQuery>,
) -> Result<StatusCode, ApiError> {
    let server = gate_server(
        &state,
        &rbac,
        server_id,
        "game.server.start_stop",
        "role insuffisant pour redemarrer un serveur",
    )
    .await?;
    let actor = resolve_actor(&rbac, q.actor_id.as_deref(), &server.owner_user_id);
    state.game_servers_uc.restart(server_id, &actor).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// DELETE /api/games/servers/{server_id}
pub async fn delete_server(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Path(server_id): Path<Uuid>,
    Query(q): Query<ActorQuery>,
) -> Result<StatusCode, ApiError> {
    let server = gate_server(
        &state,
        &rbac,
        server_id,
        "game.server.delete",
        "role insuffisant pour supprimer un serveur",
    )
    .await?;
    let actor = resolve_actor(&rbac, q.actor_id.as_deref(), &server.owner_user_id);
    state.game_servers_uc.delete(server_id, &actor).await?;
    state.broadcaster.broadcast(
        "game_server_deleted",
        serde_json::json!({"server_id": server_id, "guild_id": server.guild_id}),
    );
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
pub struct LogsQuery {
    pub lines: Option<u32>,
}

/// GET /api/games/servers/{server_id}/logs?lines=200
pub async fn get_logs(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Path(server_id): Path<Uuid>,
    Query(q): Query<LogsQuery>,
) -> Result<Json<Vec<String>>, ApiError> {
    // Les logs console peuvent contenir des secrets (RCON) et des IP joueurs.
    gate_server(
        &state,
        &rbac,
        server_id,
        "game.server.view",
        "role insuffisant pour consulter les logs",
    )
    .await?;
    let lines = q.lines.unwrap_or(200).min(1000);
    let logs = state.game_servers_uc.get_logs(server_id, lines).await?;
    Ok(Json(logs))
}

/// GET /api/games/servers/{server_id}/stats
pub async fn get_stats(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Path(server_id): Path<Uuid>,
) -> Result<Json<GameServerStatsDto>, ApiError> {
    gate_server(
        &state,
        &rbac,
        server_id,
        "game.server.view",
        "role insuffisant pour consulter les stats",
    )
    .await?;
    let stats = state.game_servers_uc.get_stats(server_id).await?;
    Ok(Json(stats.into()))
}

/// PUT /api/games/servers/{server_id}/config
pub async fn update_config(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Path(server_id): Path<Uuid>,
    Query(q): Query<ActorQuery>,
    Json(dto): Json<UpdateConfigDto>,
) -> Result<StatusCode, ApiError> {
    let server = gate_server(
        &state,
        &rbac,
        server_id,
        "game.server.config_edit",
        "role insuffisant pour editer la config",
    )
    .await?;
    let actor = resolve_actor(&rbac, q.actor_id.as_deref(), &server.owner_user_id);
    state
        .game_servers_uc
        .update_config(server_id, dto.config, &actor)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/games/servers/{server_id}/command
pub async fn execute_rcon(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Path(server_id): Path<Uuid>,
    Query(q): Query<ActorQuery>,
    Json(dto): Json<RconCommandDto>,
) -> Result<Json<RconCommandResponseDto>, ApiError> {
    let server = gate_server(
        &state,
        &rbac,
        server_id,
        "game.server.command_rcon",
        "role insuffisant pour la console RCON",
    )
    .await?;
    let actor = resolve_actor(&rbac, q.actor_id.as_deref(), &server.owner_user_id);
    let resp = state
        .game_servers_uc
        .execute_rcon(server_id, &dto.command, &actor)
        .await?;
    Ok(Json(RconCommandResponseDto { response: resp }))
}

fn resolve_actor(
    rbac: &Option<Extension<RoleContext>>,
    explicit: Option<&str>,
    fallback: &str,
) -> String {
    // L'identite authentifiee (RoleContext, token Discord verifie) PRIME : on
    // n'accepte pas un actor_id arbitraire du query pour un appelant web (sinon
    // usurpation de l'audit trail sur des actions sensibles : delete/RCON...).
    // L'actor_id explicite ne sert que pour les appels internes/bot (sans RBAC).
    if let Some(Extension(ctx)) = rbac {
        return ctx.discord_user_id.clone();
    }
    if let Some(s) = explicit {
        return s.to_string();
    }
    fallback.to_string()
}
