use crate::adapters::inbound::http::extractors::ValidatedGuild;
use axum::extract::{Path, Query, State};
use axum::Json;
use uuid::Uuid;

use crate::adapters::inbound::http::dto::community::confessions::{
    parse_report_status, ConfessionDto, ConfigDto, CreateConfessionDto, CreateReplyDto,
    CreateReportDto, DeleteConfessionDto, EditConfessionDto, ReplyDto, ReportDto, ResolveReportDto,
    SaveConfigDto, UpdateMessageRefsDto, UpdateReplyMessageDto,
};
use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::helpers::single_dto;
use crate::adapters::inbound::http::middleware::rbac::{
    check_role_for_guild, lookup_role, lookup_role_row, RoleContext,
};
use crate::adapters::inbound::http::state::AppState;
use crate::ports::inbound::community::manage_confessions::{
    CreateConfessionCommand, CreateReplyCommand, CreateReportCommand,
};
use axum::Extension;
use sentinel_core::domain::entities::community::confession::{ConfessionConfig, ReportStatus};
use sentinel_core::domain::enums::system::role::Role;
use sentinel_core::domain::errors::DomainError;

#[derive(serde::Deserialize)]
pub struct ListConfessionsQuery {
    pub limit: Option<i64>,
    pub include_deleted: Option<bool>,
}

#[derive(serde::Deserialize)]
pub struct ListReportsQuery {
    pub status: Option<String>,
    pub limit: Option<i64>,
}

// ── Helpers RBAC / anonymat ───────────────────────────────────────────────
//
// L'INTERET des confessions est l'anonymat : l'`author_user_id` (et le
// `reporter_user_id` des signalements) ne doit JAMAIS fuir vers un caller web
// qui n'a pas le role suffisant. On reproduit le pattern web-vs-bot deja
// applique a automod (`effective_facts`), tickets (`require_ticket_web`) et
// voice (`gate_by_channel_id`) :
//
//   - Pas de `RoleContext` (appel bot/interne, AuthKind::Internal/Bearer) =>
//     confiance totale : acces complet (le bot a besoin de l'auteur pour le
//     cooldown / la commande reveal).
//   - `RoleContext` present (caller web, X-Discord-Token) => on enforce le
//     role REEL sur la guild de la confession.

/// Role web effectif pour une guild dont l'appartenance est DEJA validee par
/// `guild_auth_middleware` (le `{guild_id}` est dans le path).
///
/// - Bot/interne (`None`) => `None` (pas de redaction).
/// - Web => `Some(role)` (fallback `Viewer` si pas de row `api_user_guilds`,
///   le user reste un membre Discord legitime valide par guild_auth).
/// - Superadmin => `Some(Owner)`.
async fn web_role_scoped(
    state: &AppState,
    rbac: &Option<Extension<RoleContext>>,
    guild_id: &str,
) -> Result<Option<Role>, ApiError> {
    let Some(Extension(ctx)) = rbac else {
        return Ok(None);
    };
    if state
        .superadmin_user_ids
        .iter()
        .any(|id| id == &ctx.discord_user_id)
    {
        return Ok(Some(Role::Owner));
    }
    lookup_role(state, &ctx.discord_user_id, guild_id)
        .await
        .map(Some)
        .map_err(|e| {
            ApiError(DomainError::Internal(format!(
                "RBAC lookup (confessions): {e}"
            )))
        })
}

/// Role web effectif pour une ressource dont le `guild_id` n'est PAS dans le
/// path (routes `by-id` / `by-message-id` / `replies`). Comme
/// `guild_auth_middleware` ne s'applique pas, on DOIT verifier explicitement
/// l'appartenance : un caller web sans aucun role sur la guild de la
/// confession est refuse (403) — sinon un fetch cross-guild par message_id
/// public deanonymiserait la confession.
///
/// - Bot/interne (`None`) => `None` (pas de redaction).
/// - Web membre (row presente) => `Some(role)`.
/// - Superadmin => `Some(Owner)`.
/// - Web NON membre (aucune row) => `Err(403)`.
async fn web_role_strict(
    state: &AppState,
    rbac: &Option<Extension<RoleContext>>,
    guild_id: &str,
) -> Result<Option<Role>, ApiError> {
    let Some(Extension(ctx)) = rbac else {
        return Ok(None);
    };
    if state
        .superadmin_user_ids
        .iter()
        .any(|id| id == &ctx.discord_user_id)
    {
        return Ok(Some(Role::Owner));
    }
    match lookup_role_row(state, &ctx.discord_user_id, guild_id).await {
        Ok(Some(role)) => Ok(Some(role)),
        Ok(None) => Err(ApiError(DomainError::Forbidden(
            "Acces refuse : vous n'etes pas membre de ce serveur".into(),
        ))),
        Err(e) => Err(ApiError(DomainError::Internal(format!(
            "RBAC lookup (confessions): {e}"
        )))),
    }
}

/// `true` => il faut REDACTER l'identite (auteur / reporter).
/// Bot (`None`) ne redacte jamais ; web redacte si le role n'atteint pas
/// `required`.
fn must_redact(role: Option<Role>, required: Role) -> bool {
    match role {
        None => false,
        Some(r) => !r.satisfies(required),
    }
}

/// DTO confession avec redaction conditionnelle de `author_user_id`.
fn confession_dto(
    c: sentinel_core::domain::entities::community::confession::Confession,
    redact: bool,
) -> ConfessionDto {
    let mut dto = ConfessionDto::from(c);
    if redact {
        dto.author_user_id.clear();
    }
    dto
}

fn reply_dto(
    r: sentinel_core::domain::entities::community::confession::ConfessionReply,
    redact: bool,
) -> ReplyDto {
    let mut dto = ReplyDto::from(r);
    if redact {
        dto.author_user_id.clear();
    }
    dto
}

fn report_dto(
    r: sentinel_core::domain::entities::community::confession::ConfessionReport,
    redact: bool,
) -> ReportDto {
    let mut dto = ReportDto::from(r);
    if redact {
        dto.reporter_user_id.clear();
    }
    dto
}

/// Identite de l'acteur a utiliser : pour un caller web on derive l'id du
/// PRINCIPAL authentifie (`ctx.discord_user_id`), en ignorant la valeur du
/// body (sinon un user pourrait forger un autre auteur / reporter, ou spoofer
/// la propriete d'une confession dans `edit_content`). Pour le bot/interne on
/// garde la valeur du body (le bot transmet le vrai soumetteur).
fn actor_id(rbac: &Option<Extension<RoleContext>>, body_value: String) -> String {
    match rbac {
        Some(Extension(ctx)) => ctx.discord_user_id.clone(),
        None => body_value,
    }
}

// ── Confessions ─────────────────────────────────────────────────────────

pub async fn create_confession(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Json(dto): Json<CreateConfessionDto>,
) -> Result<Json<ConfessionDto>, ApiError> {
    let author_user_id = actor_id(&rbac, dto.author_user_id);
    let c = state
        .confessions_uc
        .create(CreateConfessionCommand {
            guild_id: dto.guild_id.clone(),
            author_user_id,
            content: dto.content,
        })
        .await?;
    state.broadcaster.broadcast(
        "confession_created",
        serde_json::json!({
            "guild_id": &c.guild_id,
            "id": c.id,
            "public_number": c.public_number,
        }),
    );
    Ok(single_dto(c))
}

pub async fn update_message_refs(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Path(id): Path<Uuid>,
    Json(dto): Json<UpdateMessageRefsDto>,
) -> Result<Json<()>, ApiError> {
    // Mutation technique (refs Discord) : gate Moderator+ pour le web ;
    // bot/interne = pass-through.
    if rbac.is_some() {
        if let Ok(existing) = state.confessions_uc.get(id).await {
            check_role_for_guild(
                &state,
                &rbac,
                &existing.guild_id,
                Role::Moderator,
                "moderator+ requis",
            )
            .await?;
        }
    }
    state
        .confessions_uc
        .update_message_refs(id, dto.message_id, dto.channel_id, dto.thread_id)
        .await?;
    Ok(Json(()))
}

pub async fn edit_confession(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Path(id): Path<Uuid>,
    Json(dto): Json<EditConfessionDto>,
) -> Result<Json<ConfessionDto>, ApiError> {
    // Gate Moderator+ (web) ; ownership re-checkee par le core contre l'id
    // derive du principal.
    if rbac.is_some() {
        if let Ok(existing) = state.confessions_uc.get(id).await {
            check_role_for_guild(
                &state,
                &rbac,
                &existing.guild_id,
                Role::Moderator,
                "moderator+ requis pour editer une confession",
            )
            .await?;
        }
    }
    // S2 : pour le web, l'auteur compare est le PRINCIPAL (anti-spoof).
    let author_user_id = actor_id(&rbac, dto.author_user_id);
    let c = state
        .confessions_uc
        .edit_content(id, &author_user_id, dto.content)
        .await?;
    state.broadcaster.broadcast(
        "confession_edited",
        serde_json::json!({
            "guild_id": &c.guild_id,
            "id": c.id,
            "public_number": c.public_number,
        }),
    );
    Ok(single_dto(c))
}

pub async fn delete_confession(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Path(id): Path<Uuid>,
    Json(dto): Json<DeleteConfessionDto>,
) -> Result<Json<ConfessionDto>, ApiError> {
    // Gate RBAC web (moderator+) : on resout la guild via la confession.
    // Appel bot (pas de RoleContext) = pass-through.
    if rbac.is_some() {
        if let Ok(existing) = state.confessions_uc.get(id).await {
            check_role_for_guild(
                &state,
                &rbac,
                &existing.guild_id,
                Role::Moderator,
                "moderator+ requis pour supprimer une confession",
            )
            .await?;
        }
    }
    let deleted_by = actor_id(&rbac, dto.deleted_by);
    let c = state
        .confessions_uc
        .delete(id, deleted_by, dto.reason)
        .await?;
    state.broadcaster.broadcast(
        "confession_deleted",
        serde_json::json!({
            "guild_id": &c.guild_id,
            "id": c.id,
            "public_number": c.public_number,
            "message_id": &c.message_id,
            "channel_id": &c.channel_id,
        }),
    );
    Ok(single_dto(c))
}

pub async fn get_confession(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Path(id): Path<Uuid>,
) -> Result<Json<ConfessionDto>, ApiError> {
    let c = state.confessions_uc.get(id).await?;
    // Route sans {guild_id} : on verifie l'appartenance + le role sur la guild
    // de la confession (sinon fetch cross-guild = deanon).
    let role = web_role_strict(&state, &rbac, &c.guild_id).await?;
    let redact = must_redact(role, Role::Admin);
    Ok(Json(confession_dto(c, redact)))
}

pub async fn get_by_message_id(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Path(message_id): Path<String>,
) -> Result<Json<Option<ConfessionDto>>, ApiError> {
    let Some(c) = state.confessions_uc.get_by_message_id(&message_id).await? else {
        return Ok(Json(None));
    };
    // Route publique par message_id : verifier appartenance a la guild de la
    // confession avant TOUT retour (sinon deanon cross-guild).
    let role = web_role_strict(&state, &rbac, &c.guild_id).await?;
    let redact = must_redact(role, Role::Admin);
    Ok(Json(Some(confession_dto(c, redact))))
}

pub async fn list_confessions(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    ValidatedGuild { guild_id }: ValidatedGuild,
    Query(params): Query<ListConfessionsQuery>,
) -> Result<Json<Vec<ConfessionDto>>, ApiError> {
    let limit = params.limit.unwrap_or(50).min(500);
    let include_deleted = params.include_deleted.unwrap_or(false);
    let role = web_role_scoped(&state, &rbac, &guild_id).await?;
    let redact = must_redact(role, Role::Admin);
    let list = state
        .confessions_uc
        .list(&guild_id, limit, include_deleted)
        .await?;
    Ok(Json(
        list.into_iter()
            .map(|c| confession_dto(c, redact))
            .collect(),
    ))
}

// ── Replies ─────────────────────────────────────────────────────────────

pub async fn create_reply(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Path(confession_id): Path<Uuid>,
    Json(dto): Json<CreateReplyDto>,
) -> Result<Json<ReplyDto>, ApiError> {
    // Action utilisateur normale : tout membre de la guild peut repondre.
    // On verifie juste l'appartenance (web) et on derive l'auteur du principal.
    if rbac.is_some() {
        if let Ok(conf) = state.confessions_uc.get(confession_id).await {
            web_role_strict(&state, &rbac, &conf.guild_id).await?;
        }
    }
    let author_user_id = actor_id(&rbac, dto.author_user_id);
    let r = state
        .confessions_uc
        .create_reply(CreateReplyCommand {
            confession_id,
            author_user_id,
            content: dto.content,
            is_anonymous: dto.is_anonymous,
        })
        .await?;
    state.broadcaster.broadcast(
        "confession_reply_created",
        serde_json::json!({
            "confession_id": confession_id,
            "id": r.id,
            "public_number": r.public_number,
        }),
    );
    Ok(single_dto(r))
}

pub async fn update_reply_message_id(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(dto): Json<UpdateReplyMessageDto>,
) -> Result<Json<()>, ApiError> {
    state
        .confessions_uc
        .update_reply_message_id(id, dto.message_id)
        .await?;
    Ok(Json(()))
}

pub async fn delete_reply(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Path(id): Path<Uuid>,
    Json(dto): Json<DeleteConfessionDto>,
) -> Result<Json<ReplyDto>, ApiError> {
    // Gate Moderator+ (web) : on resout la guild via la confession parente.
    if rbac.is_some() {
        if let Ok(existing) = state.confessions_uc.get_reply_parent_guild(id).await {
            check_role_for_guild(
                &state,
                &rbac,
                &existing,
                Role::Moderator,
                "moderator+ requis pour supprimer une reponse",
            )
            .await?;
        }
    }
    let deleted_by = actor_id(&rbac, dto.deleted_by);
    let r = state.confessions_uc.delete_reply(id, deleted_by).await?;
    state.broadcaster.broadcast(
        "confession_reply_deleted",
        serde_json::json!({
            "confession_id": r.confession_id,
            "id": r.id,
            "message_id": &r.message_id,
        }),
    );
    Ok(single_dto(r))
}

pub async fn list_replies(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Path(confession_id): Path<Uuid>,
) -> Result<Json<Vec<ReplyDto>>, ApiError> {
    // Route sans {guild_id} : on resout la guild via la confession parente,
    // on verifie l'appartenance, puis on redacte les auteurs sous Admin.
    let conf = state.confessions_uc.get(confession_id).await?;
    let role = web_role_strict(&state, &rbac, &conf.guild_id).await?;
    let redact = must_redact(role, Role::Admin);
    let list = state.confessions_uc.list_replies(confession_id).await?;
    Ok(Json(
        list.into_iter().map(|r| reply_dto(r, redact)).collect(),
    ))
}

// ── Reports ─────────────────────────────────────────────────────────────

pub async fn create_report(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Json(dto): Json<CreateReportDto>,
) -> Result<Json<ReportDto>, ApiError> {
    // Action utilisateur normale : tout membre peut signaler. Web => verifie
    // l'appartenance a la guild ciblee et derive le reporter du principal.
    if rbac.is_some() {
        web_role_strict(&state, &rbac, &dto.guild_id).await?;
    }
    let reporter_user_id = actor_id(&rbac, dto.reporter_user_id);
    let r = state
        .confessions_uc
        .create_report(CreateReportCommand {
            guild_id: dto.guild_id.clone(),
            confession_id: dto.confession_id,
            reply_id: dto.reply_id,
            reporter_user_id,
            reason: dto.reason,
        })
        .await?;
    state.broadcaster.broadcast(
        "confession_report_created",
        serde_json::json!({ "guild_id": &r.guild_id, "id": r.id }),
    );
    Ok(single_dto(r))
}

pub async fn list_reports(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    ValidatedGuild { guild_id }: ValidatedGuild,
    Query(params): Query<ListReportsQuery>,
) -> Result<Json<Vec<ReportDto>>, ApiError> {
    let limit = params.limit.unwrap_or(50).min(500);
    let status = params.status.as_deref().and_then(ReportStatus::from_str);
    // A5 : `reporter_user_id` reserve aux Moderateurs+ (web) ; redacte en
    // dessous. Le bot a un acces complet.
    let role = web_role_scoped(&state, &rbac, &guild_id).await?;
    let redact = must_redact(role, Role::Moderator);
    let list = state
        .confessions_uc
        .list_reports(&guild_id, status, limit)
        .await?;
    Ok(Json(
        list.into_iter().map(|r| report_dto(r, redact)).collect(),
    ))
}

pub async fn resolve_report(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Path(id): Path<Uuid>,
    Json(dto): Json<ResolveReportDto>,
) -> Result<Json<()>, ApiError> {
    let status =
        parse_report_status(&dto.status).map_err(|m| ApiError(DomainError::ValidationError(m)))?;
    // Gate Moderator+ (web) : on resout la guild via le report.
    if rbac.is_some() {
        if let Ok(report_guild) = state.confessions_uc.get_report_guild(id).await {
            check_role_for_guild(
                &state,
                &rbac,
                &report_guild,
                Role::Moderator,
                "moderator+ requis pour resoudre un signalement",
            )
            .await?;
        }
    }
    let resolved_by = actor_id(&rbac, dto.resolved_by);
    state
        .confessions_uc
        .resolve_report(id, status, resolved_by)
        .await?;
    Ok(Json(()))
}

// ── Config ──────────────────────────────────────────────────────────────

pub async fn get_config(
    State(state): State<AppState>,
    ValidatedGuild { guild_id }: ValidatedGuild,
) -> Result<Json<ConfigDto>, ApiError> {
    let cfg = state.confessions_uc.get_config(&guild_id).await?;
    Ok(single_dto(cfg))
}

pub async fn save_config(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Json(dto): Json<SaveConfigDto>,
) -> Result<Json<ConfigDto>, ApiError> {
    check_role_for_guild(
        &state,
        &rbac,
        &dto.guild_id,
        Role::Admin,
        "admin+ requis pour modifier la config des confessions",
    )
    .await?;
    let cfg = ConfessionConfig {
        guild_id: dto.guild_id,
        enabled: dto.enabled,
        channel_id: dto.channel_id,
        panel_message_id: dto.panel_message_id,
        cooldown_secs: dto.cooldown_secs,
        max_per_day: dto.max_per_day,
        min_chars: dto.min_chars,
        max_chars: dto.max_chars,
        automod_enabled: dto.automod_enabled,
        banned_user_ids: dto.banned_user_ids,
        updated_at: chrono::Utc::now(),
    };
    let saved = state.confessions_uc.save_config(cfg).await?;
    Ok(single_dto(saved))
}
