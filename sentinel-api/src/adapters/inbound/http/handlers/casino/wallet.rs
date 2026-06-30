use crate::adapters::inbound::http::extractors::{ValidatedGuild, ValidatedGuildUser};
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
use sentinel_core::domain::entities::casino::wallet::validate_positive_amount;
use sentinel_core::domain::entities::casino::wallet::validate_transfer_distinct_users;
use sentinel_core::domain::entities::casino::wallet::Wallet;
use sentinel_core::domain::entities::casino::wallet::WalletTransaction;
use sentinel_core::domain::entities::system::discord_ids::GuildId;
use sentinel_core::domain::enums::system::role::Role;
use sentinel_core::domain::errors::DomainError;

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

// ── RBAC ──

/// Gate les endpoints "raw-mint" du wallet (credit/debit/transfer/reset).
///
/// Ces endpoints mutent directement `user_wallets` en faisant confiance au
/// `user_id` fourni dans le path/body — sans gate ils permettent a n'importe
/// quel caller desktop de frapper ou detruire des coins arbitrairement.
///
/// Semantique (mirror `gate_coude_mutation`) :
/// - rbac absent → pass-through (appel bot/internal qui credite/debite
///   legitimement via le gameplay, pas de `RoleContext`).
/// - rbac present → exige `Admin+` sur la guild cible (resolue depuis le
///   path ou le body selon le handler). Superadmin bypass gere par
///   `check_role_for_guild`.
async fn gate_wallet_admin(
    state: &AppState,
    rbac: &Option<Extension<RoleContext>>,
    guild_id: &str,
    label: &'static str,
) -> Result<(), ApiError> {
    check_role_for_guild(state, rbac, guild_id, Role::Admin, label).await
}

// ── Handlers ──

/// GET /api/wallet/{guild_id}/{user_id}
pub async fn get_wallet(
    State(state): State<AppState>,
    ValidatedGuildUser { guild_id, user_id }: ValidatedGuildUser,
) -> Result<Json<Wallet>, ApiError> {
    let wallet = state.wallet_uc.get_or_create(&guild_id, &user_id).await?;
    Ok(Json(wallet))
}

/// POST /api/wallet/{guild_id}/{user_id}/credit
pub async fn credit(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    ValidatedGuildUser { guild_id, user_id }: ValidatedGuildUser,
    Json(dto): Json<CreditDebitDto>,
) -> Result<Json<Wallet>, ApiError> {
    gate_wallet_admin(
        &state,
        &rbac,
        &guild_id,
        "admin+ requis pour crediter un wallet",
    )
    .await?;
    validate_positive_amount(dto.amount)
        .map_err(|m| ApiError(DomainError::ValidationError(m.into())))?;

    let mutation = state
        .wallet_uc
        .credit(
            &guild_id,
            &user_id,
            dto.amount,
            &dto.source,
            &dto.description,
        )
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
    rbac: Option<Extension<RoleContext>>,
    ValidatedGuildUser { guild_id, user_id }: ValidatedGuildUser,
    Json(dto): Json<CreditDebitDto>,
) -> Result<Json<Wallet>, ApiError> {
    gate_wallet_admin(
        &state,
        &rbac,
        &guild_id,
        "admin+ requis pour debiter un wallet",
    )
    .await?;
    validate_positive_amount(dto.amount)
        .map_err(|m| ApiError(DomainError::ValidationError(m.into())))?;

    let mutation = state
        .wallet_uc
        .debit(
            &guild_id,
            &user_id,
            dto.amount,
            &dto.source,
            &dto.description,
        )
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
    rbac: Option<Extension<RoleContext>>,
    Json(dto): Json<TransferDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    validation::validate_discord_id("guild_id", &dto.guild_id).map_err(ApiError)?;
    // Endpoint admin "move coins" : le `from_user_id` est fourni dans le body,
    // donc un caller desktop pourrait deplacer les coins de N'IMPORTE quel
    // user. On exige Admin+ sur la guild cible (pas un self-transfer derive du
    // principal — c'est un outil d'administration, cf. le transfert gameplay
    // user-to-user qui vit sur /api/coude/{guild}/transfer). Bot interne sans
    // RoleContext passe (gameplay legitime).
    gate_wallet_admin(
        &state,
        &rbac,
        dto.guild_id.as_str(),
        "admin+ requis pour un transfert wallet",
    )
    .await?;
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
    ValidatedGuild { guild_id }: ValidatedGuild,
    Query(params): Query<LimitQuery>,
) -> Result<Json<Vec<Wallet>>, ApiError> {
    let limit = normalize_limit(params.limit, 20, 100);
    let wallets = state.wallet_uc.leaderboard(&guild_id, limit).await?;
    Ok(Json(wallets))
}

/// GET /api/wallet/{guild_id}/{user_id}/transactions?limit=20
pub async fn transactions(
    State(state): State<AppState>,
    ValidatedGuildUser { guild_id, user_id }: ValidatedGuildUser,
    Query(params): Query<LimitQuery>,
) -> Result<Json<Vec<WalletTransaction>>, ApiError> {
    let limit = normalize_limit(params.limit, 20, 100);
    let txs = state
        .wallet_uc
        .get_transactions(&guild_id, &user_id, limit)
        .await?;
    Ok(Json(txs))
}

/// GET /api/wallet/{guild_id}/all — liste tous les wallets d'un serveur.
/// Utilise par la page Wallet du desktop.
pub async fn list_wallets(
    State(state): State<AppState>,
    ValidatedGuild { guild_id }: ValidatedGuild,
) -> Result<Json<Vec<Wallet>>, ApiError> {
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
    rbac: Option<Extension<RoleContext>>,
    ValidatedGuildUser { guild_id, user_id }: ValidatedGuildUser,
    Json(dto): Json<ResetWalletDto>,
) -> Result<Json<Wallet>, ApiError> {
    gate_wallet_admin(
        &state,
        &rbac,
        &guild_id,
        "admin+ requis pour reset un wallet",
    )
    .await?;
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
    ValidatedGuild { guild_id }: ValidatedGuild,
    Json(dto): Json<ResetWalletDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    crate::adapters::inbound::http::middleware::component_gates::check_component_role(
        &state,
        &rbac,
        &guild_id,
        "db.reset.wallets",
        "role insuffisant pour reset bulk des wallets",
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
    Ok(Json(
        serde_json::json!({ "affected": affected, "new_balance": new_balance }),
    ))
}

#[cfg(test)]
#[path = "tests/wallet.rs"]
mod tests;
