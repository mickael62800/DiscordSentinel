use axum::extract::Path;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use redis::AsyncCommands;
use uuid::Uuid;

use axum::extract::Query;

use crate::adapters::inbound::http::dto::dashboard::CreateLogDto;
use crate::adapters::inbound::http::dto::dashboard::DashboardInfractionDto;
use crate::adapters::inbound::http::dto::dashboard::DashboardRuleDto;
use crate::adapters::inbound::http::dto::dashboard::DashboardStatsDto;
use crate::adapters::inbound::http::dto::dashboard::GuildDto;
use crate::adapters::inbound::http::dto::dashboard::GuildFilterParams;
use crate::adapters::inbound::http::dto::dashboard::LogEntryDto;
use crate::adapters::inbound::http::dto::dashboard::RegisterGuildDto;
use tracing::warn;

use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::state::AppState;
use crate::domain::entities::system::log_entry::LogEntry;

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
        .filter(|l| params.guild_id.as_ref().is_none_or(|gid| l.server == *gid))
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

    state.broadcaster.broadcast(
        "log_entry_created",
        serde_json::json!({
            "level": &entry.level,
            "bot": &entry.bot,
            "message": &entry.message,
            "category": &entry.category,
            "server": &entry.server,
        }),
    );

    Ok(StatusCode::CREATED)
}

/// GET /api/infractions — journal unifie (detections automod + actions moderees)
///
/// Depuis le refactor du panneau web, cet endpoint agrege :
/// - Table `infractions` : detections automatisees (automod texte/image/conduit).
///   Le champ `moderator` y est hardcode a "AutoMod" car la table ne stocke pas
///   l'identite du composant qui a detecte.
/// - Table `moderation_actions` : sanctions prises (warn/mute/ban/unban) avec
///   leur moderator_name reel (bot, worker ou utilisateur humain via le panneau).
///
/// Resultat : le journal affiche maintenant la vraie diversite de moderateurs.
pub async fn get_all_infractions(
    State(state): State<AppState>,
    Query(params): Query<GuildFilterParams>,
) -> Result<Json<Vec<DashboardInfractionDto>>, ApiError> {
    let infractions = match &params.guild_id {
        Some(gid) => {
            let filters = crate::ports::inbound::moderation::manage_infractions::InfractionFilters {
                user_id: None,
                action: None,
                limit: 200,
                offset: 0,
            };
            state.infractions_uc.list_infractions(gid, filters).await?
        }
        None => state.infractions_uc.list_all_infractions(200, 0).await?,
    };

    let actions = state
        .moderation_uc
        .list_actions(params.guild_id.as_deref(), 200)
        .await
        .unwrap_or_default();

    let mut merged: Vec<DashboardInfractionDto> = infractions
        .into_iter()
        .map(DashboardInfractionDto::from)
        .chain(actions.into_iter().map(DashboardInfractionDto::from))
        .collect();

    // Tri global par created_at DESC — les deux sources ont deja trie mais le
    // merge les melange.
    merged.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    Ok(Json(merged))
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
        if let Err(e) = conn.set_ex::<_, _, ()>(&key, "1", 90).await {
            warn!(error = %e, bot = %payload.name, "Echec Redis set_ex heartbeat");
        }
        // Enregistrer aussi dans l'ensemble des bots connus
        if let Err(e) = conn.sadd::<_, _, ()>("bots:known", &payload.name).await {
            warn!(error = %e, bot = %payload.name, "Echec Redis sadd bots:known");
        }
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
            if let Err(e) = conn.set_ex::<_, _, ()>("guilds:all", json, 300u64).await {
                warn!(error = %e, "Echec cache set guilds:all");
            }
        }
    }

    Ok(Json(dtos))
}

/// POST /api/guilds/register — un bot enregistre/met à jour un serveur
pub async fn register_guild(
    State(state): State<AppState>,
    Json(dto): Json<RegisterGuildDto>,
) -> Result<StatusCode, ApiError> {
    let guild_id = dto.guild_id.clone();
    let owner_id = dto.owner_id.clone();

    let guild = crate::domain::entities::system::guild::Guild {
        guild_id: dto.guild_id,
        name: dto.name,
        icon: dto.icon,
        member_count: dto.member_count.unwrap_or(0),
        registered_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    state.guild_repo.upsert(&guild).await?;

    // Auto-grant le proprietaire Discord comme `owner` RBAC au premier
    // enregistrement. ON CONFLICT DO NOTHING : si quelqu un est deja defini
    // (meme en viewer), on ne l ecrase pas.
    if let Some(owner) = owner_id {
        if let Err(e) = sqlx::query(
            "INSERT INTO api_user_guilds (discord_user_id, guild_id, role, granted_by) \
             VALUES ($1, $2, 'owner', $1) \
             ON CONFLICT (discord_user_id, guild_id) DO NOTHING",
        )
        .bind(&owner)
        .bind(&guild_id)
        .execute(&state.pg_pool)
        .await
        {
            warn!(error = %e, guild_id = %guild_id, owner_id = %owner, "Echec auto-grant owner RBAC");
        }
    }

    // Invalider le cache guilds
    if let Ok(mut conn) = state.redis_client.get_multiplexed_async_connection().await {
        if let Err(e) = conn.del::<_, ()>("guilds:all").await {
            warn!(error = %e, "Echec invalidation cache guilds:all");
        }
    }

    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
#[path = "tests/dashboard.rs"]
mod tests;
