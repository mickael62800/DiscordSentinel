use axum::Json;
use serde::Serialize;

/// Convertit un Vec<T> en Json<Vec<D>> via From<T> pour D.
/// Remplace le pattern repete : `items.into_iter().map(Dto::from).collect()`
pub fn map_to_dtos<T, D: From<T>>(items: Vec<T>) -> Json<Vec<D>> {
    Json(items.into_iter().map(D::from).collect())
}

/// Normalise un parametre limit optionnel avec une valeur par defaut et un maximum.
/// Remplace le pattern repete : `params.limit.unwrap_or(default).min(max)`
pub fn normalize_limit(limit: Option<i64>, default: i64, max: i64) -> i64 {
    limit.unwrap_or(default).min(max)
}

/// Normalise un parametre days optionnel (i32).
pub fn normalize_days(days: Option<i32>, default: i32, max: i32) -> i32 {
    days.unwrap_or(default).min(max)
}

/// Reponse JSON generique pour les operations reussies.
/// Remplace le pattern repete : `Ok(Json(serde_json::json!({ "ok": true })))`
pub fn ok_response() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "ok": true }))
}

/// Reponse JSON pour une entite unique convertie en DTO.
pub fn single_dto<T, D: From<T> + Serialize>(entity: T) -> Json<D> {
    Json(D::from(entity))
}
