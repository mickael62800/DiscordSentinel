//! GET /api/ai-dataset/messages â€” liste paginee des messages utilisateurs
//! pour construction d'un dataset d'entrainement IA.
//! DELETE /api/ai-dataset/messages â€” suppression en masse des messages exportes.
//!
//! Gate :
//!   - GET : admin+ (lecture du contenu de chat)
//!   - DELETE : owner+ (action destructive)

use crate::adapters::inbound::http::errors_helpers::sqlx_internal;
use crate::adapters::inbound::http::extractors::ValidatedGuild;
use axum::extract::Query;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Extension;
use axum::Json;
use serde::Deserialize;
use serde::Serialize;

use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::middleware::rbac::require_role;
use crate::adapters::inbound::http::middleware::rbac::RoleContext;
use crate::adapters::inbound::http::state::AppState;
use sentinel_core::domain::enums::system::role::Role;
use sentinel_core::domain::errors::DomainError;

#[derive(Debug, Deserialize)]
pub struct ListMessagesQuery {
    pub channel_id: Option<String>,
    pub from: Option<String>, // ISO8601
    pub to: Option<String>,
    pub min_length: Option<i32>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct DatasetMessageDto {
    pub id: String,
    pub user_id: String,
    pub channel_id: Option<String>,
    pub channel_name: Option<String>,
    pub content: String,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct ListMessagesResponse {
    pub items: Vec<DatasetMessageDto>,
    pub total: i64,
}

fn forbid(s: StatusCode, msg: &str) -> ApiError {
    ApiError(if s == StatusCode::FORBIDDEN {
        DomainError::Forbidden(msg.into())
    } else {
        DomainError::Internal(msg.into())
    })
}

/// GET /api/ai-dataset/messages/{guild_id}
pub async fn list_messages(
    State(state): State<AppState>,
    Extension(ctx): Extension<RoleContext>,
    ValidatedGuild { guild_id }: ValidatedGuild,
    Query(q): Query<ListMessagesQuery>,
) -> Result<Json<ListMessagesResponse>, ApiError> {
    require_role(&ctx, Role::Admin).map_err(|s| forbid(s, "admin+ requis"))?;

    let limit = q.limit.unwrap_or(200).clamp(1, 1000);
    let offset = q.offset.unwrap_or(0).max(0);
    let min_len = q.min_length.unwrap_or(1).max(0) as i64;

    // Construction dynamique securisee (params bindes via $N)
    let mut sql = String::from(
        "SELECT id::text, user_id, channel_id, channel_name, content, \
                to_char(created_at, 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') \
         FROM ai_dataset_messages \
         WHERE guild_id = $1 \
           AND length(content) >= $2",
    );
    let mut count_sql = String::from(
        "SELECT COUNT(*)::bigint FROM ai_dataset_messages \
         WHERE guild_id = $1 \
           AND length(content) >= $2",
    );
    let mut idx = 3;
    if q.channel_id.is_some() {
        let f = format!(" AND channel_id = ${}", idx);
        sql.push_str(&f);
        count_sql.push_str(&f);
        idx += 1;
    }
    if q.from.is_some() {
        let f = format!(" AND created_at >= ${}::timestamptz", idx);
        sql.push_str(&f);
        count_sql.push_str(&f);
        idx += 1;
    }
    if q.to.is_some() {
        let f = format!(" AND created_at <= ${}::timestamptz", idx);
        sql.push_str(&f);
        count_sql.push_str(&f);
        idx += 1;
    }
    sql.push_str(&format!(
        " ORDER BY created_at DESC LIMIT ${} OFFSET ${}",
        idx,
        idx + 1
    ));

    // Bind helper macro-like
    let mut q_items = sqlx::query_as::<
        _,
        (
            String,
            String,
            Option<String>,
            Option<String>,
            String,
            String,
        ),
    >(&sql)
    .bind(&guild_id)
    .bind(min_len);
    let mut q_count = sqlx::query_scalar::<_, i64>(&count_sql)
        .bind(&guild_id)
        .bind(min_len);
    if let Some(c) = &q.channel_id {
        q_items = q_items.bind(c);
        q_count = q_count.bind(c);
    }
    if let Some(f) = &q.from {
        q_items = q_items.bind(f);
        q_count = q_count.bind(f);
    }
    if let Some(t) = &q.to {
        q_items = q_items.bind(t);
        q_count = q_count.bind(t);
    }
    q_items = q_items.bind(limit).bind(offset);

    let rows = q_items
        .fetch_all(&state.pg_pool)
        .await
        .map_err(sqlx_internal("query"))?;
    let total = q_count.fetch_one(&state.pg_pool).await.unwrap_or(0);

    let items: Vec<DatasetMessageDto> = rows
        .into_iter()
        .map(
            |(id, user_id, channel_id, channel_name, content, created_at)| DatasetMessageDto {
                id,
                user_id,
                channel_id,
                channel_name,
                content,
                created_at,
            },
        )
        .collect();

    Ok(Json(ListMessagesResponse { items, total }))
}

#[derive(Debug, Deserialize)]
pub struct BulkDeleteDto {
    pub ids: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct BulkDeleteResponse {
    pub deleted: i64,
}

/// DELETE /api/ai-dataset/messages/{guild_id}
pub async fn bulk_delete(
    State(state): State<AppState>,
    Extension(ctx): Extension<RoleContext>,
    ValidatedGuild { guild_id }: ValidatedGuild,
    Json(body): Json<BulkDeleteDto>,
) -> Result<Json<BulkDeleteResponse>, ApiError> {
    require_role(&ctx, Role::Owner).map_err(|s| forbid(s, "owner+ requis"))?;

    if body.ids.is_empty() {
        return Ok(Json(BulkDeleteResponse { deleted: 0 }));
    }
    if body.ids.len() > 5000 {
        return Err(ApiError(DomainError::ValidationError(
            "Max 5000 IDs par requete".into(),
        )));
    }

    // Validation : chaque id doit etre un UUID parsable
    let uuids: Vec<uuid::Uuid> = body
        .ids
        .iter()
        .map(|s| uuid::Uuid::parse_str(s))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| {
            ApiError(DomainError::ValidationError(format!(
                "uuid invalide: {}",
                e
            )))
        })?;

    let res = sqlx::query(
        "DELETE FROM ai_dataset_messages \
         WHERE guild_id = $1 \
           AND id = ANY($2)",
    )
    .bind(&guild_id)
    .bind(&uuids)
    .execute(&state.pg_pool)
    .await
    .map_err(sqlx_internal("delete"))?;

    Ok(Json(BulkDeleteResponse {
        deleted: res.rows_affected() as i64,
    }))
}
