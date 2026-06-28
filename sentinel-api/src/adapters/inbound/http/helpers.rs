use axum::Json;
use serde::Serialize;

/// Capture (clone) des champs d'un DTO AVANT de le consommer par `.into()`,
/// pour pouvoir les reutiliser ensuite (typiquement dans un broadcast).
/// Retourne `(command, (champ1, champ2, ...))`.
///
/// Avant :
/// ```ignore
/// let guild_id = dto.guild_id.clone();
/// let user_id = dto.user_id.clone();
/// let command = dto.into();
/// ```
/// Apres :
/// ```ignore
/// let (command, (guild_id, user_id)) = capture_and_into!(dto, guild_id, user_id);
/// ```
#[macro_export]
macro_rules! capture_and_into {
    ($dto:expr, $($field:ident),+ $(,)?) => {{
        let captured = ( $($dto.$field.clone(),)+ );
        let command = $dto.into();
        (command, captured)
    }};
}

/// Convertit un Vec<T> en Json<Vec<D>> via From<T> pour D.
/// Remplace le pattern repete : `items.into_iter().map(Dto::from).collect()`
pub fn map_to_dtos<T, D: From<T>>(items: Vec<T>) -> Json<Vec<D>> {
    Json(items.into_iter().map(D::from).collect())
}

/// Normalise un parametre limit optionnel avec une valeur par defaut et un maximum.
/// Garantit que la valeur est >= 0.
pub fn normalize_limit(limit: Option<i64>, default: i64, max: i64) -> i64 {
    limit.unwrap_or(default).max(0).min(max)
}

/// Normalise un parametre days optionnel (i32). Garantit >= 1.
pub fn normalize_days(days: Option<i32>, default: i32, max: i32) -> i32 {
    days.unwrap_or(default).max(1).min(max)
}

/// Normalise un parametre offset optionnel. Garantit >= 0.
pub fn normalize_offset(offset: Option<i64>) -> i64 {
    offset.unwrap_or(0).max(0)
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

#[cfg(test)]
#[path = "tests/helpers.rs"]
mod tests;
