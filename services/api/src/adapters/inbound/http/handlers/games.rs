use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::{Extension, Json};
use serde::{Deserialize, Serialize};

use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::middleware::rbac::{require_role, Role, RoleContext};
use crate::adapters::inbound::http::state::AppState;
use crate::domain::entities::{
    format_custom_emoji, is_allowed_emoji_mime, normalize_game_name, normalize_optional_tag,
    parse_role_color_hex, slugify_emoji_name, DEFAULT_GAME_ROLE_COLOR, MAX_EMOJI_IMAGE_BYTES,
};
use crate::domain::errors::DomainError;

// ── DTOs ──

#[derive(Debug, Serialize)]
pub struct GameDto {
    pub id: String,
    pub guild_id: String,
    pub game_name: String,
    pub created_by: String,
    pub created_at: String,
    pub emoji: Option<String>,
    pub category: Option<String>,
    pub role_id: Option<String>,
}

impl From<crate::ports::outbound::Game> for GameDto {
    fn from(g: crate::ports::outbound::Game) -> Self {
        Self {
            id: g.id,
            guild_id: g.guild_id,
            game_name: g.game_name,
            created_by: g.created_by,
            created_at: g.created_at,
            emoji: g.emoji,
            category: g.category,
            role_id: g.role_id,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateGameDto {
    pub guild_id: String,
    pub game_name: String,
    pub created_by: String,
    #[serde(default)]
    pub emoji: Option<String>,
    #[serde(default)]
    pub category: Option<String>,
    /// Role Discord associe au jeu. Si absent, l'API essaiera de creer
    /// automatiquement un role via Discord API (workflow UI web).
    #[serde(default)]
    pub role_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SetRoleIdDto {
    // `null` = reset a NULL, absent = NOT_PROVIDED ; ici on traite both comme "null ou valeur".
    #[serde(default)]
    pub role_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct GamePanelDto {
    pub id: String,
    pub guild_id: String,
    pub channel_id: String,
    pub message_id: String,
    pub category: Option<String>,
}

impl From<crate::ports::outbound::GamePanel> for GamePanelDto {
    fn from(p: crate::ports::outbound::GamePanel) -> Self {
        Self {
            id: p.id,
            guild_id: p.guild_id,
            channel_id: p.channel_id,
            message_id: p.message_id,
            category: p.category,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct SavePanelDto {
    pub channel_id: String,
    pub message_id: String,
    #[serde(default)]
    pub category: Option<String>,
}

// ── Games CRUD (via GameRepository) ──

pub async fn list_games(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
) -> Result<Json<Vec<GameDto>>, ApiError> {
    let games = state.game_repo.list(&guild_id).await?;
    Ok(Json(games.into_iter().map(Into::into).collect()))
}

pub async fn create_game(
    State(state): State<AppState>,
    Json(dto): Json<CreateGameDto>,
) -> Result<Json<GameDto>, ApiError> {
    let name = normalize_game_name(&dto.game_name)
        .map_err(|m| ApiError(DomainError::ValidationError(m.into())))?;
    let emoji_owned = normalize_optional_tag(dto.emoji.as_deref());
    let category_owned = normalize_optional_tag(dto.category.as_deref());
    let emoji = emoji_owned.as_deref();
    let category = category_owned.as_deref();

    // Si le DTO fournit un role_id (workflow bot qui a deja cree le role),
    // on l'utilise tel quel. Sinon (workflow UI web), on cree le role
    // Discord via le bot token et on rollback en cas d'echec DB.
    let provided_role = dto.role_id.as_deref().map(str::trim).filter(|s| !s.is_empty());
    let (role_id_to_store, created_role_for_rollback): (Option<String>, Option<String>) = if let Some(r) = provided_role {
        (Some(r.to_string()), None)
    } else {
        // Lit la couleur de role configuree pour game-bot.
        let color_hex = state
            .bot_config_repo
            .get_config(&dto.guild_id, "game-bot")
            .await
            .ok()
            .and_then(|entries| {
                entries
                    .into_iter()
                    .find(|e| e.config_key == "role_color")
                    .map(|e| e.config_value)
            })
            .unwrap_or_else(|| format!("{:06x}", DEFAULT_GAME_ROLE_COLOR));
        let color = parse_role_color_hex(&color_hex, DEFAULT_GAME_ROLE_COLOR);

        let created = state
            .discord_api
            .create_role(&dto.guild_id, &name, color, None)
            .await?;
        // On veut mentionable=true, hoist=false : patch apres creation.
        let new_id = created
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| DomainError::Internal("Discord n'a pas renvoye l'id du role cree".into()))?
            .to_string();
        // Best-effort : positionne mentionable=true.
        let _ = state
            .discord_api
            .edit_role(&dto.guild_id, &new_id, None, None, None, Some(true), Some(false))
            .await;
        (Some(new_id.clone()), Some(new_id))
    };

    let role_ref = role_id_to_store.as_deref();
    let result = state
        .game_repo
        .create(&dto.guild_id, &name, &dto.created_by, emoji, category, role_ref)
        .await;
    match result {
        Ok(game) => Ok(Json(game.into())),
        Err(e) => {
            // Rollback du role cree si l'insert DB a echoue.
            if let Some(rid) = created_role_for_rollback {
                let _ = state.discord_api.delete_role(&dto.guild_id, &rid).await;
            }
            Err(e.into())
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct UpdateGameDto {
    #[serde(default)]
    pub game_name: Option<String>,
    // emoji/category : `null` = mettre a NULL, absent = ne pas toucher.
    #[serde(default, deserialize_with = "deserialize_opt_opt")]
    pub emoji: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_opt_opt")]
    pub category: Option<Option<String>>,
}

fn deserialize_opt_opt<'de, D>(deserializer: D) -> Result<Option<Option<String>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let v = Option::<Option<String>>::deserialize(deserializer)?;
    Ok(Some(v.unwrap_or(None)))
}

pub async fn update_game(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Path((guild_id, game_id)): Path<(String, String)>,
    Json(dto): Json<UpdateGameDto>,
) -> Result<Json<GameDto>, ApiError> {
    if let Some(Extension(ctx)) = rbac {
        require_role(&ctx, Role::Admin).map_err(|_| {
            ApiError(DomainError::Forbidden(
                "admin+ requis pour modifier une game".into(),
            ))
        })?;
    }

    let name_owned: Option<String> = match dto.game_name.as_deref() {
        Some(raw) if !raw.trim().is_empty() => Some(
            normalize_game_name(raw)
                .map_err(|m| ApiError(DomainError::ValidationError(m.into())))?,
        ),
        _ => None,
    };

    let emoji: Option<Option<String>> =
        dto.emoji.as_ref().map(|opt| normalize_optional_tag(opt.as_deref()));
    let category: Option<Option<String>> =
        dto.category.as_ref().map(|opt| normalize_optional_tag(opt.as_deref()));

    let updated = state
        .game_repo
        .update(
            &guild_id,
            &game_id,
            name_owned.as_deref(),
            emoji.as_ref().map(|o| o.as_deref()),
            category.as_ref().map(|o| o.as_deref()),
        )
        .await?;
    match updated {
        Some(g) => Ok(Json(g.into())),
        None => Err(DomainError::NotFound("Jeu introuvable".into()).into()),
    }
}

pub async fn delete_game(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Path((guild_id, game_id)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    if let Some(Extension(ctx)) = rbac {
        require_role(&ctx, Role::Admin)
            .map_err(|_| ApiError(DomainError::Forbidden("admin+ requis pour supprimer une game".into())))?;
    }
    if !state.game_repo.delete(&guild_id, &game_id).await? {
        return Err(DomainError::NotFound("Jeu introuvable".into()).into());
    }
    Ok(StatusCode::NO_CONTENT)
}

// ── Role binding ──

/// PATCH /api/games/{guild_id}/{game_id}/role
/// Body: `{ "role_id": "..." | null }` — `null` efface la liaison.
pub async fn set_role_id(
    State(state): State<AppState>,
    Path((guild_id, game_id)): Path<(String, String)>,
    Json(dto): Json<SetRoleIdDto>,
) -> Result<Json<GameDto>, ApiError> {
    let role_owned = normalize_optional_tag(dto.role_id.as_deref());
    let updated = state
        .game_repo
        .set_role_id(&guild_id, &game_id, role_owned.as_deref())
        .await?;
    match updated {
        Some(g) => Ok(Json(g.into())),
        None => Err(DomainError::NotFound("Jeu introuvable".into()).into()),
    }
}

pub async fn get_game_by_name(
    State(state): State<AppState>,
    Path((guild_id, game_name)): Path<(String, String)>,
) -> Result<Json<Option<GameDto>>, ApiError> {
    let game = state.game_repo.find_by_name(&guild_id, &game_name).await?;
    Ok(Json(game.map(Into::into)))
}

// ── Panels ──

pub async fn save_panel(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
    Json(dto): Json<SavePanelDto>,
) -> Result<Json<GamePanelDto>, ApiError> {
    let category_owned = normalize_optional_tag(dto.category.as_deref());
    let panel = state
        .game_repo
        .save_panel(&guild_id, &dto.channel_id, &dto.message_id, category_owned.as_deref())
        .await?;
    Ok(Json(panel.into()))
}

pub async fn find_panel_by_message(
    State(state): State<AppState>,
    Path((guild_id, message_id)): Path<(String, String)>,
) -> Result<Json<Option<GamePanelDto>>, ApiError> {
    let panel = state.game_repo.find_panel_by_message(&guild_id, &message_id).await?;
    Ok(Json(panel.map(Into::into)))
}

pub async fn list_panels(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
) -> Result<Json<Vec<GamePanelDto>>, ApiError> {
    let panels = state.game_repo.list_panels(&guild_id).await?;
    Ok(Json(panels.into_iter().map(Into::into).collect()))
}

pub async fn list_games_by_category(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
    axum::extract::Query(q): axum::extract::Query<CategoryQuery>,
) -> Result<Json<Vec<GameDto>>, ApiError> {
    let cat_owned = normalize_optional_tag(q.category.as_deref());
    let games = state.game_repo.list_by_category(&guild_id, cat_owned.as_deref()).await?;
    Ok(Json(games.into_iter().map(Into::into).collect()))
}

#[derive(Debug, Deserialize)]
pub struct CategoryQuery {
    #[serde(default)]
    pub category: Option<String>,
}

// ── Upload emoji ──

#[derive(Debug, Serialize)]
pub struct UploadEmojiResponse {
    pub emoji: String,
    pub emoji_id: String,
    pub name: String,
    pub animated: bool,
}

pub async fn upload_emoji(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Path(guild_id): Path<String>,
    mut multipart: axum::extract::Multipart,
) -> Result<Json<UploadEmojiResponse>, ApiError> {
    if let Some(Extension(ctx)) = rbac {
        require_role(&ctx, Role::Admin).map_err(|_| {
            ApiError(DomainError::Forbidden(
                "admin+ requis pour uploader un emoji".into(),
            ))
        })?;
    }

    let mut name: Option<String> = None;
    let mut image_bytes: Option<Vec<u8>> = None;
    let mut mime: Option<String> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError(DomainError::ValidationError(format!("Multipart invalide : {e}"))))?
    {
        let field_name = field.name().unwrap_or("").to_string();
        match field_name.as_str() {
            "name" => {
                let v = field.text().await.map_err(|e| {
                    ApiError(DomainError::ValidationError(format!(
                        "Lecture champ name : {e}"
                    )))
                })?;
                name = Some(v);
            }
            "image" => {
                let ct = field.content_type().map(|s| s.to_string());
                let data = field.bytes().await.map_err(|e| {
                    ApiError(DomainError::ValidationError(format!(
                        "Lecture image : {e}"
                    )))
                })?;
                mime = ct.or_else(|| Some("image/png".to_string()));
                image_bytes = Some(data.to_vec());
            }
            _ => { /* ignore */ }
        }
    }

    let raw_name = name.ok_or_else(|| {
        ApiError(DomainError::ValidationError(
            "Champ 'name' manquant".into(),
        ))
    })?;
    let bytes = image_bytes.ok_or_else(|| {
        ApiError(DomainError::ValidationError(
            "Champ 'image' manquant".into(),
        ))
    })?;

    if bytes.len() > MAX_EMOJI_IMAGE_BYTES {
        // Compute le KB depuis la constante pour eviter toute derive
        // si la limite change un jour.
        return Err(DomainError::ValidationError(format!(
            "L'image depasse {} KB (limite Discord).",
            MAX_EMOJI_IMAGE_BYTES / 1024
        ))
        .into());
    }

    let mime = mime.unwrap_or_else(|| "image/png".to_string());
    if !is_allowed_emoji_mime(&mime) {
        return Err(DomainError::ValidationError(format!(
            "Type d'image non supporte : {mime}"
        ))
        .into());
    }

    let emoji_name = slugify_emoji_name(&raw_name);

    // Determine la guild cible (emoji_host_guild_id ou guild courante).
    let host_guild = state
        .bot_config_repo
        .get_config(&guild_id, "game-bot")
        .await
        .map(|entries| {
            entries
                .into_iter()
                .find(|e| e.config_key == "emoji_host_guild_id")
                .map(|e| e.config_value)
                .filter(|v| !v.trim().is_empty())
        })
        .unwrap_or(None)
        .unwrap_or_else(|| guild_id.clone());

    let (emoji_id, final_name, animated) = state
        .discord_api
        .upload_emoji(&host_guild, &emoji_name, &bytes, &mime)
        .await?;

    let formatted = format_custom_emoji(&final_name, &emoji_id, animated);

    Ok(Json(UploadEmojiResponse {
        emoji: formatted,
        emoji_id,
        name: final_name,
        animated,
    }))
}

