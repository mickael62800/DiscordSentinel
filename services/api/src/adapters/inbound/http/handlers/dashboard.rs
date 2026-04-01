use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use redis::AsyncCommands;
use uuid::Uuid;

use axum::extract::Query;

use crate::adapters::inbound::http::dto::dashboard::{
    CreateLogDto, DashboardInfractionDto, DashboardRuleDto, DashboardStatsDto, GuildDto,
    GuildFilterParams, LogEntryDto, RegisterGuildDto,
};
use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::state::AppState;
use crate::domain::entities::LogEntry;

/// GET /api/stats — stats globales pour le dashboard desktop
pub async fn get_dashboard_stats(
    State(state): State<AppState>,
) -> Result<Json<DashboardStatsDto>, ApiError> {
    let stats = state.stats_uc.get_dashboard_stats().await?;
    Ok(Json(DashboardStatsDto::from(stats)))
}

/// GET /api/logs — logs récents (filtrable par guild_id)
pub async fn get_logs(
    State(state): State<AppState>,
    Query(params): Query<GuildFilterParams>,
) -> Result<Json<Vec<LogEntryDto>>, ApiError> {
    let logs = state.log_repo.find_all(200).await?;
    let filtered: Vec<LogEntryDto> = logs
        .into_iter()
        .filter(|l| params.guild_id.as_ref().map_or(true, |gid| l.server == *gid))
        .map(LogEntryDto::from)
        .collect();
    Ok(Json(filtered))
}

/// DELETE /api/logs/{category} — supprimer tous les logs d'une categorie
pub async fn delete_logs_by_category(
    State(state): State<AppState>,
    Path(category): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    if category == "discord" {
        return Err(ApiError(crate::domain::errors::DomainError::ValidationError(
            "Impossible de supprimer les journaux Discord".into(),
        )));
    }
    let count = state.log_repo.delete_by_category(&category).await?;
    Ok(Json(serde_json::json!({ "deleted": count })))
}

/// POST /api/logs — écrire un log (utilisé par les bots)
pub async fn create_log(
    State(state): State<AppState>,
    Json(dto): Json<CreateLogDto>,
) -> Result<StatusCode, ApiError> {
    let bot_name = dto.bot.unwrap_or_default();
    let category = dto.category.unwrap_or_else(|| {
        if bot_name.contains("worker") { "worker".to_string() }
        else if bot_name.contains("-bot") { "bot".to_string() }
        else { "discord".to_string() }
    });
    let entry = LogEntry {
        id: Uuid::new_v4(),
        timestamp: chrono::Utc::now(),
        level: dto.level.unwrap_or_else(|| "info".to_string()),
        bot: bot_name,
        server: dto.server.unwrap_or_default(),
        message: dto.message,
        category,
        details: dto.details.unwrap_or(serde_json::json!({})),
    };
    state.log_repo.save(&entry).await?;
    Ok(StatusCode::CREATED)
}

/// GET /api/infractions — infractions (filtrable par guild_id)
pub async fn get_all_infractions(
    State(state): State<AppState>,
    Query(params): Query<GuildFilterParams>,
) -> Result<Json<Vec<DashboardInfractionDto>>, ApiError> {
    let infractions = match &params.guild_id {
        Some(gid) => {
            let filters = crate::ports::inbound::InfractionFilters {
                user_id: None,
                action: None,
                limit: 200,
                offset: 0,
            };
            state.infractions_uc.list_infractions(gid, filters).await?
        }
        None => state.infractions_uc.list_all_infractions(200, 0).await?,
    };
    Ok(Json(
        infractions
            .into_iter()
            .map(DashboardInfractionDto::from)
            .collect(),
    ))
}

/// GET /api/rules — règles (filtrable par guild_id)
pub async fn get_all_rules(
    State(state): State<AppState>,
    Query(params): Query<GuildFilterParams>,
) -> Result<Json<Vec<DashboardRuleDto>>, ApiError> {
    let rules = match &params.guild_id {
        Some(gid) => state.rules_uc.get_rules(gid).await?,
        None => state.rules_uc.get_all_rules().await?,
    };
    Ok(Json(rules.into_iter().map(DashboardRuleDto::from).collect()))
}

/// PATCH /api/rules/{id} — toggle enabled/disabled
pub async fn toggle_rule(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<TogglePayload>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let enabled = state.rules_uc.toggle_rule(id, payload.enabled).await?;
    Ok(Json(serde_json::json!({ "enabled": enabled })))
}

#[derive(serde::Deserialize)]
pub struct TogglePayload {
    pub enabled: bool,
}

/// POST /api/bots/heartbeat — un bot signale qu'il est en ligne
pub async fn bot_heartbeat(
    State(state): State<AppState>,
    Json(payload): Json<HeartbeatPayload>,
) -> Result<axum::http::StatusCode, ApiError> {
    // Stocker dans Redis avec TTL de 90 secondes
    if let Ok(mut conn) = state.redis_client.get_multiplexed_async_connection().await {
        use redis::AsyncCommands;
        let key = format!("bot:online:{}", payload.name);
        let _: () = conn.set_ex(&key, "1", 90).await.unwrap_or(());
        // Enregistrer aussi dans l'ensemble des bots connus
        let _: () = conn.sadd("bots:known", &payload.name).await.unwrap_or(());
    }

    state.broadcaster.broadcast(
        "bot_heartbeat",
        serde_json::json!({ "name": &payload.name }),
    );

    Ok(axum::http::StatusCode::NO_CONTENT)
}

#[derive(serde::Deserialize)]
pub struct HeartbeatPayload {
    pub name: String,
}

/// GET /api/guilds — liste des serveurs connus (cache 5min)
pub async fn list_guilds(
    State(state): State<AppState>,
) -> Result<Json<Vec<GuildDto>>, ApiError> {
    // Cache-first
    if let Ok(mut conn) = state.redis_client.get_multiplexed_async_connection().await {
        if let Ok(Some(json)) = conn.get::<_, Option<String>>("guilds:all").await {
            if let Ok(dtos) = serde_json::from_str::<Vec<GuildDto>>(&json) {
                return Ok(Json(dtos));
            }
        }
    }

    let guilds = state.guild_repo.find_all().await?;
    let dtos: Vec<GuildDto> = guilds.into_iter().map(GuildDto::from).collect();

    if let Ok(mut conn) = state.redis_client.get_multiplexed_async_connection().await {
        if let Ok(json) = serde_json::to_string(&dtos) {
            let _: Result<(), _> = conn.set_ex("guilds:all", json, 300u64).await;
        }
    }

    Ok(Json(dtos))
}

/// POST /api/guilds/register — un bot enregistre/met à jour un serveur
pub async fn register_guild(
    State(state): State<AppState>,
    Json(dto): Json<RegisterGuildDto>,
) -> Result<StatusCode, ApiError> {
    let guild = crate::domain::entities::Guild {
        guild_id: dto.guild_id,
        name: dto.name,
        icon: dto.icon,
        member_count: dto.member_count.unwrap_or(0),
        registered_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    state.guild_repo.upsert(&guild).await?;

    // Invalider le cache guilds
    if let Ok(mut conn) = state.redis_client.get_multiplexed_async_connection().await {
        let _: Result<(), _> = conn.del("guilds:all").await;
    }

    Ok(StatusCode::NO_CONTENT)
}
