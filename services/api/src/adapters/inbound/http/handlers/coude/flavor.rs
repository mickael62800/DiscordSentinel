//! Handler HTTP catalogue de templates flavor (Phase 3 #9 audit).
//!
//! Endpoint read-only : le bot tire un template au hasard pour une cle
//! donnee (`steal_success_afk`, `heist_fail`, etc.) et `locale` (default `fr`).

use axum::extract::{Path, Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::state::AppState;

#[derive(Debug, Deserialize, Default)]
pub struct FlavorQuery {
    /// Locale, default "fr". Le client peut envoyer "en" plus tard.
    #[serde(default = "default_locale")]
    pub locale: String,
}

fn default_locale() -> String {
    "fr".into()
}

#[derive(Debug, Serialize)]
pub struct FlavorTemplateDto {
    pub content: String,
}

/// GET /api/coude/flavor/{key}/random?locale=fr
///
/// Retourne 404 si aucun template ne matche (le bot fallback alors sur
/// ses arrays locales pour preserver le comportement legacy).
pub async fn get_random_flavor(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Query(q): Query<FlavorQuery>,
) -> Result<Json<FlavorTemplateDto>, ApiError> {
    match state
        .coude_flavor_templates_repo
        .random_by_key(&key, &q.locale)
        .await?
    {
        Some(content) => Ok(Json(FlavorTemplateDto { content })),
        None => Err(ApiError(crate::domain::errors::DomainError::NotFound(
            format!("Aucun template pour key={key}, locale={}", q.locale),
        ))),
    }
}

