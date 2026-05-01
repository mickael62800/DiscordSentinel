//! GET/POST/DELETE /api/docker/* — administration Docker via le socket /var/run/docker.sock.
//!
//! Toutes les actions destructives (start/stop/restart/delete/prune) sont gardees
//! par require_superadmin. Les GET listing/inspect sont gates par moderator+ via
//! le middleware standard (suffisant : ils n'exposent que des metadonnees techniques).
//!
//! Necessite que /var/run/docker.sock soit monte (RW) dans le conteneur API.

use std::collections::HashMap;
use std::sync::OnceLock;

use axum::extract::Extension;
use axum::extract::Path;
use axum::extract::Query;
use axum::extract::State;
use axum::Json;
use bollard::container::ListContainersOptions;
use bollard::container::LogsOptions;
use bollard::container::RemoveContainerOptions;
use bollard::container::RestartContainerOptions;
use bollard::container::StopContainerOptions;
use bollard::image::ListImagesOptions;
use bollard::image::RemoveImageOptions;
use bollard::network::ListNetworksOptions;
use bollard::system::EventsOptions;
use bollard::volume::ListVolumesOptions;
use bollard::Docker;
use futures_util::StreamExt;
use serde::Deserialize;
use serde::Serialize;

use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::helpers::ok_response;
use crate::adapters::inbound::http::middleware::rbac::require_superadmin;
use crate::adapters::inbound::http::middleware::rbac::RoleContext;
use crate::adapters::inbound::http::state::AppState;
use crate::domain::errors::DomainError;

/// Singleton du client Docker. Bollard ouvre une connexion lazy au socket.
static DOCKER: OnceLock<Docker> = OnceLock::new();

fn docker() -> Result<&'static Docker, ApiError> {
    if let Some(d) = DOCKER.get() {
        return Ok(d);
    }
    let d = Docker::connect_with_local_defaults()
        .map_err(|e| ApiError(DomainError::Internal(format!("docker socket: {}", e))))?;
    let _ = DOCKER.set(d);
    Ok(DOCKER.get().expect("docker just initialized"))
}

fn forbid(msg: &str) -> ApiError {
    ApiError(DomainError::Forbidden(msg.into()))
}

fn gate_super(state: &AppState, rbac: &Option<Extension<RoleContext>>) -> Result<(), ApiError> {
    let Some(Extension(ctx)) = rbac else {
        return Err(forbid("auth requise"));
    };
    require_superadmin(state, ctx).map_err(|_| forbid("superadmin requis"))
}

/// Helper d'audit log pour les actions Docker destructives.
/// Tracking via tracing::info! structure -> apparait dans les logs API
/// avec actor.user_id, action, target. Permet de retrouver qui a lance
/// quoi en cas de probleme post-mortem.
/// Logue en `tracing::info` ET en BDD `server_events` pour qu'il soit visible
/// sur la page Securite serveur.
fn audit_docker(
    state: &AppState,
    rbac: &Option<Extension<RoleContext>>,
    action: &str,
    target: &str,
) {
    let actor = match rbac {
        Some(Extension(ctx)) => ctx.discord_user_id.clone(),
        None => "anonymous".to_string(),
    };
    tracing::info!(
        target: "audit::docker",
        actor = %actor,
        action = action,
        target = target,
        "docker admin action"
    );
    let pool = state.pg_pool.clone();
    let actor_owned = actor.clone();
    let action_owned = format!("docker.{}", action);
    let target_owned = target.to_string();
    let severity = if action.contains("prune") || action.contains("remove") {
        "warn"
    } else {
        "info"
    };
    let sev_owned = severity.to_string();
    tokio::spawn(async move {
        crate::adapters::inbound::http::handlers::system::server_events::record_server_event(
            &pool,
            &actor_owned,
            None,
            &action_owned,
            Some(&target_owned),
            &sev_owned,
            serde_json::json!({}),
        )
        .await;
    });
}

fn map_err(e: bollard::errors::Error) -> ApiError {
    ApiError(DomainError::Internal(format!("docker: {}", e)))
}

// ── DTOs ──────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct OverviewDto {
    pub version: String,
    pub api_version: String,
    pub os: String,
    pub arch: String,
    pub kernel: String,
    pub containers_running: i64,
    pub containers_paused: i64,
    pub containers_stopped: i64,
    pub images_count: i64,
    pub volumes_count: i64,
    pub networks_count: i64,
    pub layers_size_bytes: i64,
    pub images_size_bytes: i64,
    pub containers_size_bytes: i64,
    pub volumes_size_bytes: i64,
    pub build_cache_size_bytes: i64,
    pub reclaimable_images_bytes: i64,
    pub reclaimable_containers_bytes: i64,
    pub reclaimable_volumes_bytes: i64,
    pub reclaimable_build_cache_bytes: i64,
}

#[derive(Debug, Serialize)]
pub struct ContainerDto {
    pub id: String,
    pub names: Vec<String>,
    pub image: String,
    pub state: String,
    pub status: String,
    pub created: i64,
    pub size_rw_bytes: Option<i64>,
    pub size_root_fs_bytes: Option<i64>,
    pub ports: Vec<String>,
    pub labels: HashMap<String, String>,
}

#[derive(Debug, Serialize)]
pub struct ImageDto {
    pub id: String,
    pub repo_tags: Vec<String>,
    pub repo_digests: Vec<String>,
    pub created: i64,
    pub size_bytes: i64,
    pub shared_size_bytes: i64,
    pub virtual_size_bytes: i64,
    pub containers: i64,
    pub dangling: bool,
}

#[derive(Debug, Serialize)]
pub struct VolumeDto {
    pub name: String,
    pub driver: String,
    pub mountpoint: String,
    pub created_at: Option<String>,
    pub size_bytes: Option<i64>,
    pub ref_count: Option<i64>,
    pub in_use: bool,
}

#[derive(Debug, Serialize)]
pub struct NetworkDto {
    pub id: String,
    pub name: String,
    pub driver: String,
    pub scope: String,
    pub internal: bool,
    pub containers_count: usize,
}

#[derive(Debug, Serialize)]
pub struct PruneResultDto {
    pub deleted: Vec<String>,
    pub space_reclaimed_bytes: u64,
}

// ── Overview (df + version) ───────────────────────────────────────────────

pub async fn get_overview(State(_state): State<AppState>) -> Result<Json<OverviewDto>, ApiError> {
    let d = docker()?;
    let v = d.version().await.map_err(map_err)?;
    let info = d.info().await.map_err(map_err)?;
    let df = d.df().await.map_err(map_err)?;

    // Tailles + reclaimable depuis df
    let mut images_size: i64 = 0;
    let mut layers_size: i64 = df.layers_size.unwrap_or(0);
    let mut reclaimable_images: i64 = 0;
    if let Some(images) = df.images.as_ref() {
        for img in images {
            images_size += img.size;
            if img.containers == 0 {
                reclaimable_images += img.size;
            }
        }
    }
    if layers_size == 0 {
        layers_size = images_size;
    }

    let mut containers_size: i64 = 0;
    let mut reclaimable_containers: i64 = 0;
    if let Some(containers) = df.containers.as_ref() {
        for c in containers {
            containers_size += c.size_rw.unwrap_or(0);
            if c.state.as_deref() != Some("running") {
                reclaimable_containers += c.size_rw.unwrap_or(0);
            }
        }
    }

    let mut volumes_size: i64 = 0;
    let mut reclaimable_volumes: i64 = 0;
    if let Some(volumes) = df.volumes.as_ref() {
        for v in volumes {
            if let Some(usage) = v.usage_data.as_ref() {
                volumes_size += usage.size;
                if usage.ref_count == 0 {
                    reclaimable_volumes += usage.size;
                }
            }
        }
    }

    let mut build_cache_size: i64 = 0;
    let mut reclaimable_build: i64 = 0;
    if let Some(cache) = df.build_cache.as_ref() {
        for c in cache {
            let s = c.size.unwrap_or(0);
            build_cache_size += s;
            if !c.in_use.unwrap_or(false) {
                reclaimable_build += s;
            }
        }
    }

    Ok(Json(OverviewDto {
        version: v.version.unwrap_or_default(),
        api_version: v.api_version.unwrap_or_default(),
        os: v.os.unwrap_or_default(),
        arch: v.arch.unwrap_or_default(),
        kernel: v.kernel_version.unwrap_or_default(),
        containers_running: info.containers_running.unwrap_or(0),
        containers_paused: info.containers_paused.unwrap_or(0),
        containers_stopped: info.containers_stopped.unwrap_or(0),
        images_count: info.images.unwrap_or(0),
        volumes_count: df.volumes.as_ref().map(|v| v.len() as i64).unwrap_or(0),
        networks_count: 0, // rempli par list_networks separement si besoin
        layers_size_bytes: layers_size,
        images_size_bytes: images_size,
        containers_size_bytes: containers_size,
        volumes_size_bytes: volumes_size,
        build_cache_size_bytes: build_cache_size,
        reclaimable_images_bytes: reclaimable_images,
        reclaimable_containers_bytes: reclaimable_containers,
        reclaimable_volumes_bytes: reclaimable_volumes,
        reclaimable_build_cache_bytes: reclaimable_build,
    }))
}

// ── Containers ────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ListContainersQuery {
    #[serde(default)]
    pub all: Option<bool>,
}

pub async fn list_containers(
    State(_state): State<AppState>,
    Query(q): Query<ListContainersQuery>,
) -> Result<Json<Vec<ContainerDto>>, ApiError> {
    let d = docker()?;
    let opts = ListContainersOptions::<String> {
        all: q.all.unwrap_or(true),
        size: true,
        ..Default::default()
    };
    let list = d.list_containers(Some(opts)).await.map_err(map_err)?;
    let out: Vec<ContainerDto> = list
        .into_iter()
        .map(|c| ContainerDto {
            id: c.id.unwrap_or_default(),
            names: c.names.unwrap_or_default(),
            image: c.image.unwrap_or_default(),
            state: c.state.unwrap_or_default(),
            status: c.status.unwrap_or_default(),
            created: c.created.unwrap_or(0),
            size_rw_bytes: c.size_rw,
            size_root_fs_bytes: c.size_root_fs,
            ports: c
                .ports
                .unwrap_or_default()
                .into_iter()
                .map(|p| {
                    let priv_port = p.private_port;
                    let pub_port = p.public_port.unwrap_or(0);
                    let typ = p
                        .typ
                        .map(|t| format!("{:?}", t).to_lowercase())
                        .unwrap_or_else(|| "tcp".to_string());
                    if pub_port > 0 {
                        format!("{}:{}/{}", pub_port, priv_port, typ)
                    } else {
                        format!("{}/{}", priv_port, typ)
                    }
                })
                .collect(),
            labels: c.labels.unwrap_or_default(),
        })
        .collect();
    Ok(Json(out))
}

pub async fn start_container(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    gate_super(&state, &rbac)?;
    audit_docker(&state, &rbac, "container.start", &id);
    let d = docker()?;
    d.start_container::<String>(&id, None).await.map_err(map_err)?;
    Ok(ok_response())
}

#[derive(Debug, Deserialize)]
pub struct StopQuery {
    #[serde(default)]
    pub timeout: Option<i64>,
}

pub async fn stop_container(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Path(id): Path<String>,
    Query(q): Query<StopQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    gate_super(&state, &rbac)?;
    audit_docker(&state, &rbac, "container.stop", &id);
    let d = docker()?;
    let opts = StopContainerOptions {
        t: q.timeout.unwrap_or(10),
    };
    d.stop_container(&id, Some(opts)).await.map_err(map_err)?;
    Ok(ok_response())
}

pub async fn restart_container(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Path(id): Path<String>,
    Query(q): Query<StopQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    gate_super(&state, &rbac)?;
    audit_docker(&state, &rbac, "container.restart", &id);
    let d = docker()?;
    let opts = RestartContainerOptions {
        t: q.timeout.unwrap_or(10) as isize,
    };
    d.restart_container(&id, Some(opts)).await.map_err(map_err)?;
    Ok(ok_response())
}

#[derive(Debug, Deserialize)]
pub struct RemoveContainerQuery {
    #[serde(default)]
    pub force: Option<bool>,
    #[serde(default)]
    pub volumes: Option<bool>,
}

pub async fn remove_container(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Path(id): Path<String>,
    Query(q): Query<RemoveContainerQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    gate_super(&state, &rbac)?;
    audit_docker(&state, &rbac, "container.remove", &id);
    let d = docker()?;
    let opts = RemoveContainerOptions {
        force: q.force.unwrap_or(false),
        v: q.volumes.unwrap_or(false),
        ..Default::default()
    };
    d.remove_container(&id, Some(opts)).await.map_err(map_err)?;
    Ok(ok_response())
}

#[derive(Debug, Deserialize)]
pub struct LogsQuery {
    #[serde(default)]
    pub tail: Option<u32>,
    #[serde(default)]
    pub timestamps: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct LogsDto {
    pub logs: String,
}

pub async fn container_logs(
    State(_state): State<AppState>,
    Path(id): Path<String>,
    Query(q): Query<LogsQuery>,
) -> Result<Json<LogsDto>, ApiError> {
    let d = docker()?;
    let tail = q.tail.unwrap_or(200).min(5000).to_string();
    let opts = LogsOptions::<String> {
        stdout: true,
        stderr: true,
        tail,
        timestamps: q.timestamps.unwrap_or(false),
        follow: false,
        ..Default::default()
    };
    let mut stream = d.logs(&id, Some(opts));
    let mut out = String::new();
    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(c) => out.push_str(&c.to_string()),
            Err(e) => return Err(map_err(e)),
        }
        if out.len() > 2_000_000 {
            out.push_str("\n[...troncature 2MB...]");
            break;
        }
    }
    Ok(Json(LogsDto { logs: out }))
}

// ── Images ────────────────────────────────────────────────────────────────

pub async fn list_images(
    State(_state): State<AppState>,
) -> Result<Json<Vec<ImageDto>>, ApiError> {
    let d = docker()?;
    let opts = ListImagesOptions::<String> {
        all: false,
        ..Default::default()
    };
    let list = d.list_images(Some(opts)).await.map_err(map_err)?;
    let out: Vec<ImageDto> = list
        .into_iter()
        .map(|i| {
            let repo_tags = i.repo_tags.clone();
            let dangling = repo_tags.is_empty()
                || repo_tags.iter().all(|t| t == "<none>:<none>");
            ImageDto {
                id: i.id,
                repo_tags,
                repo_digests: i.repo_digests,
                created: i.created,
                size_bytes: i.size,
                shared_size_bytes: i.shared_size,
                virtual_size_bytes: i.virtual_size.unwrap_or(0),
                containers: i.containers,
                dangling,
            }
        })
        .collect();
    Ok(Json(out))
}

#[derive(Debug, Deserialize)]
pub struct RemoveImageQuery {
    #[serde(default)]
    pub force: Option<bool>,
    #[serde(default)]
    pub no_prune: Option<bool>,
}

pub async fn remove_image(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Path(id): Path<String>,
    Query(q): Query<RemoveImageQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    gate_super(&state, &rbac)?;
    audit_docker(&state, &rbac, "image.remove", &id);
    let d = docker()?;
    let opts = RemoveImageOptions {
        force: q.force.unwrap_or(false),
        noprune: q.no_prune.unwrap_or(false),
    };
    d.remove_image(&id, Some(opts), None).await.map_err(map_err)?;
    Ok(ok_response())
}

// ── Volumes ───────────────────────────────────────────────────────────────

pub async fn list_volumes(
    State(_state): State<AppState>,
) -> Result<Json<Vec<VolumeDto>>, ApiError> {
    let d = docker()?;
    let resp = d
        .list_volumes(None::<ListVolumesOptions<String>>)
        .await
        .map_err(map_err)?;
    let out: Vec<VolumeDto> = resp
        .volumes
        .unwrap_or_default()
        .into_iter()
        .map(|v| {
            let usage = v.usage_data;
            let (size, ref_count) = match &usage {
                Some(u) => (Some(u.size), Some(u.ref_count)),
                None => (None, None),
            };
            VolumeDto {
                name: v.name,
                driver: v.driver,
                mountpoint: v.mountpoint,
                created_at: v.created_at,
                size_bytes: size,
                ref_count,
                in_use: ref_count.map(|r| r > 0).unwrap_or(false),
            }
        })
        .collect();
    Ok(Json(out))
}

pub async fn remove_volume(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Path(name): Path<String>,
    Query(q): Query<RemoveImageQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    gate_super(&state, &rbac)?;
    audit_docker(&state, &rbac, "volume.remove", &name);
    let d = docker()?;
    let opts = bollard::volume::RemoveVolumeOptions {
        force: q.force.unwrap_or(false),
    };
    d.remove_volume(&name, Some(opts)).await.map_err(map_err)?;
    Ok(ok_response())
}

// ── Networks ──────────────────────────────────────────────────────────────

pub async fn list_networks(
    State(_state): State<AppState>,
) -> Result<Json<Vec<NetworkDto>>, ApiError> {
    let d = docker()?;
    let list = d
        .list_networks(None::<ListNetworksOptions<String>>)
        .await
        .map_err(map_err)?;
    let out: Vec<NetworkDto> = list
        .into_iter()
        .map(|n| NetworkDto {
            id: n.id.unwrap_or_default(),
            name: n.name.unwrap_or_default(),
            driver: n.driver.unwrap_or_default(),
            scope: n.scope.unwrap_or_default(),
            internal: n.internal.unwrap_or(false),
            containers_count: n.containers.map(|c| c.len()).unwrap_or(0),
        })
        .collect();
    Ok(Json(out))
}

// ── Prune ─────────────────────────────────────────────────────────────────

pub async fn prune_containers(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
) -> Result<Json<PruneResultDto>, ApiError> {
    gate_super(&state, &rbac)?;
    audit_docker(&state, &rbac, "prune.containers", "*");
    let d = docker()?;
    let r = d
        .prune_containers(None::<bollard::container::PruneContainersOptions<String>>)
        .await
        .map_err(map_err)?;
    Ok(Json(PruneResultDto {
        deleted: r.containers_deleted.unwrap_or_default(),
        space_reclaimed_bytes: r.space_reclaimed.unwrap_or(0) as u64,
    }))
}

#[derive(Debug, Deserialize)]
pub struct PruneImagesQuery {
    /// Si `true` : supprime aussi les images non taggees mais utilisees nulle part.
    /// Si `false` (defaut) : seulement les "dangling" (sans tag).
    #[serde(default)]
    pub all: Option<bool>,
}

pub async fn prune_images(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Query(q): Query<PruneImagesQuery>,
) -> Result<Json<PruneResultDto>, ApiError> {
    gate_super(&state, &rbac)?;
    audit_docker(
        &state,
        &rbac,
        "prune.images",
        if q.all.unwrap_or(false) { "all=true" } else { "dangling=true" },
    );
    let d = docker()?;
    let mut filters: HashMap<String, Vec<String>> = HashMap::new();
    let dangling = if q.all.unwrap_or(false) { "false" } else { "true" };
    filters.insert("dangling".to_string(), vec![dangling.to_string()]);
    let opts = bollard::image::PruneImagesOptions { filters };
    let r = d.prune_images(Some(opts)).await.map_err(map_err)?;
    let deleted: Vec<String> = r
        .images_deleted
        .unwrap_or_default()
        .into_iter()
        .filter_map(|i| i.deleted.or(i.untagged))
        .collect();
    Ok(Json(PruneResultDto {
        deleted,
        space_reclaimed_bytes: r.space_reclaimed.unwrap_or(0) as u64,
    }))
}

pub async fn prune_volumes(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
) -> Result<Json<PruneResultDto>, ApiError> {
    gate_super(&state, &rbac)?;
    audit_docker(&state, &rbac, "prune.volumes", "*");
    let d = docker()?;
    let r = d
        .prune_volumes(None::<bollard::volume::PruneVolumesOptions<String>>)
        .await
        .map_err(map_err)?;
    Ok(Json(PruneResultDto {
        deleted: r.volumes_deleted.unwrap_or_default(),
        space_reclaimed_bytes: r.space_reclaimed.unwrap_or(0) as u64,
    }))
}

pub async fn prune_networks(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
) -> Result<Json<PruneResultDto>, ApiError> {
    gate_super(&state, &rbac)?;
    audit_docker(&state, &rbac, "prune.networks", "*");
    let d = docker()?;
    let r = d
        .prune_networks(None::<bollard::network::PruneNetworksOptions<String>>)
        .await
        .map_err(map_err)?;
    Ok(Json(PruneResultDto {
        deleted: r.networks_deleted.unwrap_or_default(),
        space_reclaimed_bytes: 0,
    }))
}

#[derive(Debug, Serialize)]
pub struct PruneSystemDto {
    pub containers: PruneResultDto,
    pub images: PruneResultDto,
    pub volumes: PruneResultDto,
    pub networks: PruneResultDto,
    pub total_space_reclaimed_bytes: u64,
}

#[derive(Debug, Deserialize)]
pub struct PruneSystemQuery {
    #[serde(default)]
    pub volumes: Option<bool>,
    #[serde(default)]
    pub all_images: Option<bool>,
}

/// POST /api/docker/prune/system — prune complet (containers + images + networks
/// + volumes optionnels). Equivalent `docker system prune` cote CLI.
pub async fn prune_system(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Query(q): Query<PruneSystemQuery>,
) -> Result<Json<PruneSystemDto>, ApiError> {
    gate_super(&state, &rbac)?;
    audit_docker(
        &state,
        &rbac,
        "prune.system",
        &format!(
            "volumes={},all_images={}",
            q.volumes.unwrap_or(false),
            q.all_images.unwrap_or(false)
        ),
    );
    let d = docker()?;

    let containers = d
        .prune_containers(None::<bollard::container::PruneContainersOptions<String>>)
        .await
        .map_err(map_err)?;

    let mut img_filters: HashMap<String, Vec<String>> = HashMap::new();
    let dangling = if q.all_images.unwrap_or(false) { "false" } else { "true" };
    img_filters.insert("dangling".to_string(), vec![dangling.to_string()]);
    let images = d
        .prune_images(Some(bollard::image::PruneImagesOptions {
            filters: img_filters,
        }))
        .await
        .map_err(map_err)?;

    let networks = d
        .prune_networks(None::<bollard::network::PruneNetworksOptions<String>>)
        .await
        .map_err(map_err)?;

    let volumes = if q.volumes.unwrap_or(false) {
        d.prune_volumes(None::<bollard::volume::PruneVolumesOptions<String>>)
            .await
            .map_err(map_err)?
    } else {
        bollard::models::VolumePruneResponse {
            volumes_deleted: Some(vec![]),
            space_reclaimed: Some(0),
        }
    };

    let cont_space = containers.space_reclaimed.unwrap_or(0) as u64;
    let img_space = images.space_reclaimed.unwrap_or(0) as u64;
    let vol_space = volumes.space_reclaimed.unwrap_or(0) as u64;

    let containers_dto = PruneResultDto {
        deleted: containers.containers_deleted.unwrap_or_default(),
        space_reclaimed_bytes: cont_space,
    };
    let images_dto = PruneResultDto {
        deleted: images
            .images_deleted
            .unwrap_or_default()
            .into_iter()
            .filter_map(|i| i.deleted.or(i.untagged))
            .collect(),
        space_reclaimed_bytes: img_space,
    };
    let volumes_dto = PruneResultDto {
        deleted: volumes.volumes_deleted.unwrap_or_default(),
        space_reclaimed_bytes: vol_space,
    };
    let networks_dto = PruneResultDto {
        deleted: networks.networks_deleted.unwrap_or_default(),
        space_reclaimed_bytes: 0,
    };

    Ok(Json(PruneSystemDto {
        total_space_reclaimed_bytes: cont_space + img_space + vol_space,
        containers: containers_dto,
        images: images_dto,
        volumes: volumes_dto,
        networks: networks_dto,
    }))
}

/// Garde l'import EventsOptions vivant si bollard l'utilise dans une feature future.
#[allow(dead_code)]
fn _unused_events() -> EventsOptions<String> {
    EventsOptions::<String> {
        ..Default::default()
    }
}
