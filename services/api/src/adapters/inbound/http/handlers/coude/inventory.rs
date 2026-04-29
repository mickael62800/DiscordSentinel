//! Handlers inventaire/primes/assurances. Délèguent à `state.coude_inventory_uc`.

use axum::extract::Path;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use uuid::Uuid;

use super::dto::AddItemDto;
use super::dto::BuyInsuranceDto;
use super::dto::ClaimPrimesDto;
use super::dto::CreatePrimeDto;
use super::dto::InsuranceDto;
use super::dto::InventoryItemDto;
use super::dto::PrimeDto;
use super::dto::UseItemDto;
use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::state::AppState;
use crate::domain::entities::coude::inventory::NewCoudePrime;
use crate::domain::errors::DomainError;
use crate::domain::entities::system::discord_ids::UserId;

// ── Items ──

/// GET /api/coude/{guild_id}/inventory/{user_id}
pub async fn get_inventory(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
) -> Result<Json<Vec<InventoryItemDto>>, ApiError> {
    let items = state
        .coude_inventory_uc
        .list_inventory(&guild_id, &user_id)
        .await?;
    Ok(Json(items.into_iter().map(InventoryItemDto::from).collect()))
}

/// POST /api/coude/{guild_id}/inventory/{user_id}/add
pub async fn add_item(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
    Json(dto): Json<AddItemDto>,
) -> Result<StatusCode, ApiError> {
    state
        .coude_inventory_uc
        .add_item(&guild_id, &user_id, &dto.item_key)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/coude/{guild_id}/inventory/{user_id}/use
pub async fn use_item(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
    Json(dto): Json<UseItemDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let success = state
        .coude_inventory_uc
        .use_item(&guild_id, &user_id, &dto.item_key)
        .await?;
    Ok(Json(serde_json::json!({ "success": success })))
}

/// GET /api/coude/{guild_id}/inventory/{user_id}/has/{item_key}
pub async fn has_item(
    State(state): State<AppState>,
    Path((guild_id, user_id, item_key)): Path<(String, String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let has = state
        .coude_inventory_uc
        .has_item(&guild_id, &user_id, &item_key)
        .await?;
    Ok(Json(serde_json::json!({ "has_item": has })))
}

// ── Primes (bounties) ──

/// POST /api/coude/{guild_id}/primes
pub async fn create_prime(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
    Json(dto): Json<CreatePrimeDto>,
) -> Result<Json<PrimeDto>, ApiError> {
    let prime = state
        .coude_inventory_uc
        .create_prime(NewCoudePrime {
            guild_id: guild_id.into(),
            target_id: dto.target_id,
            target_name: dto.target_name,
            placed_by_id: dto.placed_by_id,
            placed_by_name: dto.placed_by_name,
            amount: dto.amount,
        })
        .await?;
    Ok(Json(prime.into()))
}

/// GET /api/coude/{guild_id}/primes/{target_id}/active
pub async fn get_active_primes(
    State(state): State<AppState>,
    Path((guild_id, target_id)): Path<(String, String)>,
) -> Result<Json<Vec<PrimeDto>>, ApiError> {
    let primes = state
        .coude_inventory_uc
        .list_active_primes(&guild_id, &target_id)
        .await?;
    Ok(Json(primes.into_iter().map(PrimeDto::from).collect()))
}

/// POST /api/coude/{guild_id}/primes/claim
pub async fn claim_primes(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
    Json(dto): Json<ClaimPrimesDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let total = state
        .coude_inventory_uc
        .claim_primes(&guild_id, &dto.target_id, &dto.claimer_id, &dto.claimer_name)
        .await?;
    Ok(Json(serde_json::json!({ "total_claimed": total })))
}

// ── Assurances ──

/// POST /api/coude/{guild_id}/insurance/buy
pub async fn buy_insurance(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
    Json(dto): Json<BuyInsuranceDto>,
) -> Result<StatusCode, ApiError> {
    let inserted = state
        .coude_inventory_uc
        .buy_insurance(&guild_id, &dto.user_id, dto.is_scam, dto.duration_seconds)
        .await?;
    if !inserted {
        return Err(ApiError(crate::domain::errors::DomainError::Conflict(
            "Une assurance active existe deja pour ce joueur".into(),
        )));
    }
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, serde::Deserialize)]
pub struct BuyInsuranceWithRollDto {
    pub user_id: UserId,
    pub scam_rate_pct: u32,
    pub duration_seconds: i64,
    pub level: i32,
}

#[derive(Debug, serde::Serialize)]
pub struct BuyInsuranceResolvedDto {
    pub created: bool,
    pub is_scam: bool,
}

/// POST /api/coude/{guild_id}/insurance/buy-with-roll
///
/// Phase 2 #3 audit : RNG `scam` migre cote API. Le bot envoie le taux
/// de scam (config guild) et le niveau, l'API roule + persiste + retourne
/// le verdict.
pub async fn buy_insurance_with_roll(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
    Json(dto): Json<BuyInsuranceWithRollDto>,
) -> Result<Json<BuyInsuranceResolvedDto>, ApiError> {
    let (created, is_scam) = state
        .coude_inventory_uc
        .buy_insurance_with_scam_roll(
            &guild_id,
            &dto.user_id,
            dto.scam_rate_pct,
            dto.duration_seconds,
            dto.level,
        )
        .await?;
    if !created {
        return Err(ApiError(crate::domain::errors::DomainError::Conflict(
            "Une assurance active existe deja pour ce joueur".into(),
        )));
    }
    Ok(Json(BuyInsuranceResolvedDto { created, is_scam }))
}

/// GET /api/coude/{guild_id}/insurance/{user_id}
pub async fn get_active_insurance(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
) -> Result<Json<Option<InsuranceDto>>, ApiError> {
    let insurance = state
        .coude_inventory_uc
        .get_active_insurance(&guild_id, &user_id)
        .await?;
    Ok(Json(insurance.map(InsuranceDto::from)))
}

/// POST /api/coude/insurance/{insurance_id}/expire
pub async fn expire_insurance(
    State(state): State<AppState>,
    Path(insurance_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let id = Uuid::parse_str(&insurance_id).map_err(|_| {
        ApiError::from(DomainError::ValidationError(
            "ID d'assurance invalide (UUID attendu)".into(),
        ))
    })?;
    state.coude_inventory_uc.expire_insurance(id).await?;
    Ok(StatusCode::NO_CONTENT)
}
