use axum::extract::Path;
use axum::extract::Query;
use axum::extract::State;
use axum::Extension;
use axum::Json;
use serde::Deserialize;
use tracing::info;

use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::helpers::normalize_limit;
use crate::adapters::inbound::http::helpers::ok_response;
use crate::adapters::inbound::http::middleware::rbac::check_role_for_guild;
use crate::adapters::inbound::http::middleware::rbac::RoleContext;
use crate::adapters::inbound::http::state::AppState;
use crate::adapters::inbound::http::validation;
use crate::domain::enums::system::role::Role;
use crate::domain::entities::casino::wallet::validate_positive_amount;
use crate::domain::entities::casino::wallet::validate_transfer_distinct_users;
use crate::domain::entities::casino::wallet::Wallet;
use crate::domain::entities::casino::wallet::WalletTransaction;
use crate::domain::errors::DomainError;
use crate::domain::entities::system::discord_ids::GuildId;

// ── DTOs ──

#[derive(Debug, Deserialize)]
pub struct CreditDebitDto {
    pub amount: i64,
    pub source: String,
    pub description: String,
}

#[derive(Debug, Deserialize)]
pub struct TransferDto {
    pub guild_id: GuildId,
    pub from_user_id: String,
    pub to_user_id: String,
    pub amount: i64,
    pub source: String,
    pub description: String,
}

#[derive(Debug, Deserialize)]
pub struct LimitQuery {
    pub limit: Option<i64>,
}

// ── Handlers ──

/// GET /api/wallet/{guild_id}/{user_id}
pub async fn get_wallet(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
) -> Result<Json<Wallet>, ApiError> {
    validation::validate_guild_user_path(&guild_id, &user_id).map_err(ApiError)?;

    let wallet = state.wallet_uc.get_or_create(&guild_id, &user_id).await?;
    Ok(Json(wallet))
}

/// POST /api/wallet/{guild_id}/{user_id}/credit
pub async fn credit(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
    Json(dto): Json<CreditDebitDto>,
) -> Result<Json<Wallet>, ApiError> {
    validation::validate_guild_user_path(&guild_id, &user_id).map_err(ApiError)?;
    validate_positive_amount(dto.amount)
        .map_err(|m| ApiError(DomainError::ValidationError(m.into())))?;

    let mutation = state
        .wallet_uc
        .credit(&guild_id, &user_id, dto.amount, &dto.source, &dto.description)
        .await?;

    state.broadcaster.broadcast(
        "wallet_credit",
        serde_json::json!({
            "guild_id": guild_id,
            "user_id": user_id,
            "amount": dto.amount,
            "balance": mutation.new_balance,
            "source": dto.source,
        }),
    );

    let wallet = state.wallet_uc.get_or_create(&guild_id, &user_id).await?;
    Ok(Json(wallet))
}

/// POST /api/wallet/{guild_id}/{user_id}/debit
pub async fn debit(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
    Json(dto): Json<CreditDebitDto>,
) -> Result<Json<Wallet>, ApiError> {
    validation::validate_guild_user_path(&guild_id, &user_id).map_err(ApiError)?;
    validate_positive_amount(dto.amount)
        .map_err(|m| ApiError(DomainError::ValidationError(m.into())))?;

    let mutation = state
        .wallet_uc
        .debit(&guild_id, &user_id, dto.amount, &dto.source, &dto.description)
        .await?;

    state.broadcaster.broadcast(
        "wallet_debit",
        serde_json::json!({
            "guild_id": guild_id,
            "user_id": user_id,
            "amount": dto.amount,
            "balance": mutation.new_balance,
            "source": dto.source,
        }),
    );

    let wallet = state.wallet_uc.get_or_create(&guild_id, &user_id).await?;
    Ok(Json(wallet))
}

/// POST /api/wallet/transfer
pub async fn transfer(
    State(state): State<AppState>,
    Json(dto): Json<TransferDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    validation::validate_discord_id("guild_id", &dto.guild_id).map_err(ApiError)?;
    validation::validate_discord_id("from_user_id", &dto.from_user_id).map_err(ApiError)?;
    validation::validate_discord_id("to_user_id", &dto.to_user_id).map_err(ApiError)?;
    validate_positive_amount(dto.amount)
        .map_err(|m| ApiError(DomainError::ValidationError(m.into())))?;
    validate_transfer_distinct_users(&dto.from_user_id, &dto.to_user_id)
        .map_err(|m| ApiError(DomainError::ValidationError(m.into())))?;

    state
        .wallet_uc
        .transfer(
            &dto.guild_id,
            &dto.from_user_id,
            &dto.to_user_id,
            dto.amount,
            &dto.source,
            &dto.description,
        )
        .await?;

    state.broadcaster.broadcast(
        "wallet_transfer",
        serde_json::json!({
            "guild_id": dto.guild_id,
            "from_user_id": dto.from_user_id,
            "to_user_id": dto.to_user_id,
            "amount": dto.amount,
            "source": dto.source,
        }),
    );

    info!(
        guild_id = %dto.guild_id,
        from = %dto.from_user_id,
        to = %dto.to_user_id,
        amount = dto.amount,
        "Transfert wallet effectue"
    );

    Ok(ok_response())
}

/// GET /api/wallet/{guild_id}/leaderboard?limit=20
pub async fn leaderboard(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
    Query(params): Query<LimitQuery>,
) -> Result<Json<Vec<Wallet>>, ApiError> {
    validation::validate_guild_id_path(&guild_id).map_err(ApiError)?;

    let limit = normalize_limit(params.limit, 20, 100);
    let wallets = state.wallet_uc.leaderboard(&guild_id, limit).await?;
    Ok(Json(wallets))
}

/// GET /api/wallet/{guild_id}/{user_id}/transactions?limit=20
pub async fn transactions(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
    Query(params): Query<LimitQuery>,
) -> Result<Json<Vec<WalletTransaction>>, ApiError> {
    validation::validate_guild_user_path(&guild_id, &user_id).map_err(ApiError)?;

    let limit = normalize_limit(params.limit, 20, 100);
    let txs = state.wallet_uc.get_transactions(&guild_id, &user_id, limit).await?;
    Ok(Json(txs))
}

/// GET /api/wallet/{guild_id}/all — liste tous les wallets d'un serveur.
/// Utilise par la page Wallet du desktop.
pub async fn list_wallets(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
) -> Result<Json<Vec<Wallet>>, ApiError> {
    validation::validate_guild_id_path(&guild_id).map_err(ApiError)?;
    let wallets = state.wallet_uc.list_by_guild(&guild_id).await?;
    Ok(Json(wallets))
}

#[derive(Debug, Deserialize)]
pub struct ResetWalletDto {
    /// Nouveau solde de depart (optionnel, defaut 100).
    pub new_balance: Option<i64>,
}

/// POST /api/wallet/{guild_id}/{user_id}/reset — reset individuel.
pub async fn reset_wallet(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
    Json(dto): Json<ResetWalletDto>,
) -> Result<Json<Wallet>, ApiError> {
    validation::validate_guild_user_path(&guild_id, &user_id).map_err(ApiError)?;

    let (wallet, new_balance) = state
        .wallet_uc
        .reset_wallet(&guild_id, &user_id, dto.new_balance)
        .await?;

    state.broadcaster.broadcast(
        "wallet_reset",
        serde_json::json!({
            "guild_id": guild_id,
            "user_id": user_id,
            "new_balance": new_balance,
        }),
    );

    Ok(Json(wallet))
}

/// POST /api/wallet/{guild_id}/reset-all — reset bulk de tous les wallets.
pub async fn reset_all_wallets(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Path(guild_id): Path<String>,
    Json(dto): Json<ResetWalletDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    validation::validate_guild_id_path(&guild_id).map_err(ApiError)?;
    check_role_for_guild(
        &state, &rbac, &guild_id, Role::Owner,
        "owner uniquement pour reset bulk des wallets",
    )
    .await?;

    let (affected, new_balance) = state
        .wallet_uc
        .reset_all_wallets(&guild_id, dto.new_balance)
        .await?;

    state.broadcaster.broadcast(
        "wallet_reset_all",
        serde_json::json!({
            "guild_id": guild_id,
            "affected": affected,
            "new_balance": new_balance,
        }),
    );

    info!(guild_id = %guild_id, affected, new_balance, "Bulk wallet reset");
    Ok(Json(serde_json::json!({ "affected": affected, "new_balance": new_balance })))
}

#[cfg(test)]
#[path = "tests/wallet.rs"]
mod tests;
