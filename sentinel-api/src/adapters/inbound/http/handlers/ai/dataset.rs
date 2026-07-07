//! GET /api/ai-dataset/messages — liste paginee des messages utilisateurs
//! pour construction d'un dataset d'entrainement IA.
//! DELETE /api/ai-dataset/messages — suppression en masse des messages exportes.
//!
//! Adaptateur ENTRANT mince : RBAC + parse/map. Le bornage des filtres et la
//! validation des ids vivent dans `ManageDatasetUseCase` ; le SQL dans
//! `DatasetRepository`.
//!
//! Gate :
//!   - GET : admin+ (lecture du contenu de chat)
//!   - DELETE : owner+ (action destructive)

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
use crate::ports::inbound::ai::manage_dataset::{BulkDeleteCommand, ListDatasetQuery};
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

fn forbid(msg: &str) -> ApiError {
    ApiError(DomainError::Forbidden(msg.into()))
}

/// GET /api/ai-dataset/messages/{guild_id}
pub async fn list_messages(
    State(state): State<AppState>,
    Extension(ctx): Extension<RoleContext>,
    ValidatedGuild { guild_id }: ValidatedGuild,
    Query(q): Query<ListMessagesQuery>,
) -> Result<Json<ListMessagesResponse>, ApiError> {
    require_role(&ctx, Role::Admin).map_err(|_: StatusCode| forbid("admin+ requis"))?;

    let page = state
        .dataset_uc
        .list_messages(ListDatasetQuery {
            guild_id,
            channel_id: q.channel_id,
            from: q.from,
            to: q.to,
            min_length: q.min_length,
            limit: q.limit,
            offset: q.offset,
        })
        .await?;

    let items = page
        .items
        .into_iter()
        .map(|m| DatasetMessageDto {
            id: m.id,
            user_id: m.user_id,
            channel_id: m.channel_id,
            channel_name: m.channel_name,
            content: m.content,
            created_at: m.created_at,
        })
        .collect();

    Ok(Json(ListMessagesResponse {
        items,
        total: page.total,
    }))
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
    require_role(&ctx, Role::Owner).map_err(|_: StatusCode| forbid("owner+ requis"))?;

    let deleted = state
        .dataset_uc
        .bulk_delete(BulkDeleteCommand {
            guild_id,
            ids: body.ids,
        })
        .await?;

    Ok(Json(BulkDeleteResponse { deleted }))
}
