//! Handlers multiplayer : tables, joueurs, clôture, listing des parties.

use super::dto::CreateTableDto;
use super::dto::JoinTableDto;
use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::extractors::ValidatedGuild;
use crate::adapters::inbound::http::middleware::rbac::check_role_for_guild;
use crate::adapters::inbound::http::middleware::rbac::RoleContext;
use crate::adapters::inbound::http::state::AppState;
use crate::ports::outbound::casino::blackjack_table_repository::BlackjackTable;
use crate::ports::outbound::casino::blackjack_table_repository::BlackjackTablePlayer;
use axum::extract::Path;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Extension;
use axum::Json;
use sentinel_core::domain::enums::system::role::Role;
use sentinel_core::domain::errors::DomainError;
/// POST /api/blackjack/tables
pub async fn create_table(
    State(state): State<AppState>,
    Json(dto): Json<CreateTableDto>,
) -> Result<Json<BlackjackTable>, ApiError> {
    use rand::seq::SliceRandom;
    use sentinel_core::domain::entities::casino::blackjack::create_deck;
    use sentinel_core::domain::entities::casino::blackjack::BLACKJACK_SHOE_DECKS;
    use sentinel_core::domain::entities::casino::blackjack::BLACKJACK_SHOE_TOTAL_CARDS;

    // Regle metier : shoe de 6 decks standard casino (cf. domain::entities::blackjack).
    let mut shoe: Vec<sentinel_core::domain::entities::casino::blackjack::Card> =
        Vec::with_capacity(BLACKJACK_SHOE_TOTAL_CARDS);
    for _ in 0..BLACKJACK_SHOE_DECKS {
        shoe.extend(create_deck());
    }
    shoe.shuffle(&mut rand::thread_rng());

    let shoe_json = serde_json::to_value(&shoe)
        .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?;

    let table = state
        .blackjack_table_repo
        .create(
            &dto.guild_id,
            &dto.channel_id,
            &dto.owner_id,
            &dto.owner_name,
            &shoe_json,
        )
        .await?;
    Ok(Json(table))
}

/// POST /api/blackjack/tables/{table_id}/join
pub async fn join_table(
    State(state): State<AppState>,
    Path(table_id): Path<String>,
    Json(dto): Json<JoinTableDto>,
) -> Result<StatusCode, ApiError> {
    let info = state
        .blackjack_table_repo
        .get_status_and_guild(&table_id)
        .await?;
    match &info {
        Some((s, _)) if s == "open" => {}
        Some(_) => return Err(DomainError::Conflict("Table fermee".into()).into()),
        None => return Err(DomainError::NotFound("Table introuvable".into()).into()),
    }
    let guild_id = info.unwrap().1;

    let current_count = state.blackjack_table_repo.count_players(&table_id).await?;
    let max_players: i64 = state
        .bot_config_repo
        .get_config(&guild_id, "blackjack-bot")
        .await
        .unwrap_or_default()
        .iter()
        .find(|c| c.config_key == "max_players_per_table")
        .and_then(|c| c.config_value.parse().ok())
        .unwrap_or(
            sentinel_core::domain::entities::casino::blackjack::DEFAULT_BLACKJACK_MAX_PLAYERS,
        );

    if current_count >= max_players {
        return Err(DomainError::ValidationError(format!(
            "Table pleine ({current_count}/{max_players} joueurs)"
        ))
        .into());
    }

    state
        .blackjack_table_repo
        .add_player(&table_id, &dto.user_id, &dto.user_name)
        .await?;
    // touch_activity best-effort : si elle echoue, la table peut etre
    // marquee inactive a tort par le worker cleanup. On log au moins.
    if let Err(e) = state.blackjack_table_repo.touch_activity(&table_id).await {
        tracing::warn!(
            event_type = "blackjack.touch_activity_failed",
            table_id = %table_id,
            error = %e,
            "Echec touch_activity : table risque d'etre nettoyee a tort"
        );
    }
    Ok(StatusCode::NO_CONTENT)
}

/// GET /api/blackjack/tables/{table_id}/players
pub async fn list_table_players(
    State(state): State<AppState>,
    Path(table_id): Path<String>,
) -> Result<Json<Vec<BlackjackTablePlayer>>, ApiError> {
    let players = state.blackjack_table_repo.list_players(&table_id).await?;
    Ok(Json(players))
}

/// GET /api/blackjack/tables/by-channel/{channel_id}
pub async fn get_table_by_channel(
    State(state): State<AppState>,
    Path(channel_id): Path<String>,
) -> Result<Json<Option<BlackjackTable>>, ApiError> {
    let table = state
        .blackjack_table_repo
        .find_open_by_channel(&channel_id)
        .await?;
    Ok(Json(table))
}

/// DELETE /api/blackjack/tables/{table_id}
pub async fn close_table(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Path(table_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let guild_id = state.blackjack_table_repo.get_guild_id(&table_id).await?;
    if let Some(ref gid) = guild_id {
        if rbac.is_some() {
            check_role_for_guild(
                &state,
                &rbac,
                gid,
                Role::Moderator,
                "moderator+ requis pour fermer une table",
            )
            .await?;
        }
    }
    state.blackjack_table_repo.close(&table_id).await?;

    // Sync bilateral : event Redis + WS pour que le bot edite l'embed
    // Discord (gris + footer) et que le web rafraichisse la liste.
    state.broadcaster.broadcast(
        "blackjack_table_closed",
        serde_json::json!({
            "table_id": &table_id,
            "action_id": &table_id,
            "guild_id": guild_id,
            "actor": { "source": "web" },
        }),
    );

    Ok(StatusCode::NO_CONTENT)
}

/// GET /api/blackjack/admin/{guild_id}/tables
pub async fn list_tables_by_guild(
    State(state): State<AppState>,
    ValidatedGuild { guild_id }: ValidatedGuild,
) -> Result<Json<Vec<BlackjackTable>>, ApiError> {
    let tables = state
        .blackjack_table_repo
        .list_open_by_guild(&guild_id)
        .await?;
    Ok(Json(tables))
}

/// GET /api/blackjack/tables/{table_id}/games
pub async fn list_table_games(
    State(state): State<AppState>,
    Path(table_id): Path<String>,
) -> Result<Json<Vec<serde_json::Value>>, ApiError> {
    let games = state.blackjack_table_repo.list_games(&table_id).await?;
    Ok(Json(games))
}
