use axum::extract::{Path, State};
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::state::AppState;
use crate::domain::errors::DomainError;

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct WelcomeConfigDto {
    pub guild_id: String,
    pub welcome_enabled: bool,
    pub welcome_channel_id: Option<String>,
    pub welcome_message: String,
    pub welcome_embed_color: String,
    pub welcome_dm_enabled: bool,
    pub welcome_dm_message: String,
    pub leave_enabled: bool,
    pub leave_channel_id: Option<String>,
    pub leave_message: String,
    pub rules_enabled: bool,
    pub rules_channel_id: Option<String>,
    pub rules_message: String,
    pub rules_role_id: Option<String>,
    pub rules_button_label: String,
    pub counter_enabled: bool,
    pub counter_channel_id: Option<String>,
    pub counter_format: String,
    pub anniversary_enabled: bool,
    pub anniversary_channel_id: Option<String>,
    pub anniversary_message: String,
    pub rejoin_message: String,
}

#[derive(Debug, Deserialize)]
pub struct SaveWelcomeConfigDto {
    pub welcome_enabled: Option<bool>,
    pub welcome_channel_id: Option<String>,
    pub welcome_message: Option<String>,
    pub welcome_embed_color: Option<String>,
    pub welcome_dm_enabled: Option<bool>,
    pub welcome_dm_message: Option<String>,
    pub leave_enabled: Option<bool>,
    pub leave_channel_id: Option<String>,
    pub leave_message: Option<String>,
    pub rules_enabled: Option<bool>,
    pub rules_channel_id: Option<String>,
    pub rules_message: Option<String>,
    pub rules_role_id: Option<String>,
    pub rules_button_label: Option<String>,
    pub counter_enabled: Option<bool>,
    pub counter_channel_id: Option<String>,
    pub counter_format: Option<String>,
    pub anniversary_enabled: Option<bool>,
    pub anniversary_channel_id: Option<String>,
    pub anniversary_message: Option<String>,
    pub rejoin_message: Option<String>,
}

/// GET /api/welcome/{guild_id}
pub async fn get_config(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
) -> Result<Json<WelcomeConfigDto>, ApiError> {
    let config = sqlx::query_as::<_, WelcomeConfigDto>(
        "SELECT guild_id, welcome_enabled, welcome_channel_id, welcome_message, welcome_embed_color, \
         welcome_dm_enabled, welcome_dm_message, leave_enabled, leave_channel_id, leave_message, \
         rules_enabled, rules_channel_id, rules_message, rules_role_id, rules_button_label, \
         counter_enabled, counter_channel_id, counter_format, \
         anniversary_enabled, anniversary_channel_id, anniversary_message, rejoin_message \
         FROM welcome_config WHERE guild_id = $1",
    )
    .bind(&guild_id)
    .fetch_optional(&state.pg_pool)
    .await
    .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?;

    Ok(Json(config.unwrap_or(WelcomeConfigDto {
        guild_id: guild_id.clone(),
        welcome_enabled: true,
        welcome_channel_id: None,
        welcome_message: "Bienvenue {user} sur **{server}** ! Tu es le **{count}e** membre.".into(),
        welcome_embed_color: "3498db".into(),
        welcome_dm_enabled: false,
        welcome_dm_message: "Bienvenue sur **{server}** !".into(),
        leave_enabled: true,
        leave_channel_id: None,
        leave_message: "{user} nous a quittes. Nous sommes maintenant **{count}** membres.".into(),
        rules_enabled: false,
        rules_channel_id: None,
        rules_message: "Lis les regles et clique sur le bouton pour acceder au serveur.".into(),
        rules_role_id: None,
        rules_button_label: "J'accepte les regles".into(),
        counter_enabled: false,
        counter_channel_id: None,
        counter_format: "Membres : {count}".into(),
        anniversary_enabled: false,
        anniversary_channel_id: None,
        anniversary_message: "Felicitations {user}, ca fait **{years} an(s)** que tu es sur **{server}** !".into(),
        rejoin_message: "Content de te revoir {user} ! Tu nous avais manque.".into(),
    })))
}

/// PUT /api/welcome/{guild_id}
pub async fn save_config(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
    Json(dto): Json<SaveWelcomeConfigDto>,
) -> Result<Json<WelcomeConfigDto>, ApiError> {
    let config = sqlx::query_as::<_, WelcomeConfigDto>(
        r#"INSERT INTO welcome_config (guild_id,
            welcome_enabled, welcome_channel_id, welcome_message, welcome_embed_color,
            welcome_dm_enabled, welcome_dm_message,
            leave_enabled, leave_channel_id, leave_message,
            rules_enabled, rules_channel_id, rules_message, rules_role_id, rules_button_label,
            counter_enabled, counter_channel_id, counter_format,
            anniversary_enabled, anniversary_channel_id, anniversary_message, rejoin_message)
           VALUES ($1,
            COALESCE($2, true), $3, COALESCE($4, 'Bienvenue {user} sur **{server}** !'), COALESCE($5, '3498db'),
            COALESCE($6, false), COALESCE($7, 'Bienvenue sur **{server}** !'),
            COALESCE($8, true), $9, COALESCE($10, '{user} nous a quittes.'),
            COALESCE($11, false), $12, COALESCE($13, 'Lis les regles.'), $14, COALESCE($15, 'J''accepte les regles'),
            COALESCE($16, false), $17, COALESCE($18, 'Membres : {count}'),
            COALESCE($19, false), $20, COALESCE($21, 'Felicitations {user} !'),
            COALESCE($22, 'Content de te revoir {user} !'))
           ON CONFLICT (guild_id) DO UPDATE SET
            welcome_enabled = COALESCE($2, welcome_config.welcome_enabled),
            welcome_channel_id = COALESCE($3, welcome_config.welcome_channel_id),
            welcome_message = COALESCE($4, welcome_config.welcome_message),
            welcome_embed_color = COALESCE($5, welcome_config.welcome_embed_color),
            welcome_dm_enabled = COALESCE($6, welcome_config.welcome_dm_enabled),
            welcome_dm_message = COALESCE($7, welcome_config.welcome_dm_message),
            leave_enabled = COALESCE($8, welcome_config.leave_enabled),
            leave_channel_id = COALESCE($9, welcome_config.leave_channel_id),
            leave_message = COALESCE($10, welcome_config.leave_message),
            rules_enabled = COALESCE($11, welcome_config.rules_enabled),
            rules_channel_id = COALESCE($12, welcome_config.rules_channel_id),
            rules_message = COALESCE($13, welcome_config.rules_message),
            rules_role_id = COALESCE($14, welcome_config.rules_role_id),
            rules_button_label = COALESCE($15, welcome_config.rules_button_label),
            counter_enabled = COALESCE($16, welcome_config.counter_enabled),
            counter_channel_id = COALESCE($17, welcome_config.counter_channel_id),
            counter_format = COALESCE($18, welcome_config.counter_format),
            anniversary_enabled = COALESCE($19, welcome_config.anniversary_enabled),
            anniversary_channel_id = COALESCE($20, welcome_config.anniversary_channel_id),
            anniversary_message = COALESCE($21, welcome_config.anniversary_message),
            rejoin_message = COALESCE($22, welcome_config.rejoin_message),
            updated_at = NOW()
           RETURNING guild_id, welcome_enabled, welcome_channel_id, welcome_message, welcome_embed_color,
            welcome_dm_enabled, welcome_dm_message, leave_enabled, leave_channel_id, leave_message,
            rules_enabled, rules_channel_id, rules_message, rules_role_id, rules_button_label,
            counter_enabled, counter_channel_id, counter_format,
            anniversary_enabled, anniversary_channel_id, anniversary_message, rejoin_message"#,
    )
    .bind(&guild_id)
    .bind(dto.welcome_enabled)
    .bind(&dto.welcome_channel_id)
    .bind(&dto.welcome_message)
    .bind(&dto.welcome_embed_color)
    .bind(dto.welcome_dm_enabled)
    .bind(&dto.welcome_dm_message)
    .bind(dto.leave_enabled)
    .bind(&dto.leave_channel_id)
    .bind(&dto.leave_message)
    .bind(dto.rules_enabled)
    .bind(&dto.rules_channel_id)
    .bind(&dto.rules_message)
    .bind(&dto.rules_role_id)
    .bind(&dto.rules_button_label)
    .bind(dto.counter_enabled)
    .bind(&dto.counter_channel_id)
    .bind(&dto.counter_format)
    .bind(dto.anniversary_enabled)
    .bind(&dto.anniversary_channel_id)
    .bind(&dto.anniversary_message)
    .bind(&dto.rejoin_message)
    .fetch_one(&state.pg_pool)
    .await
    .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?;

    Ok(Json(config))
}
