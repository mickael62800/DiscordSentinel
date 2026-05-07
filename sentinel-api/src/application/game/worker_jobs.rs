//! Logique des 3 jobs du game-portal-worker, exposees via l'API.
//!
//! Ces fonctions sont appelees par les endpoints internes /api/games/internal/jobs/*
//! que le worker invoque sur un timer. Elles utilisent les use cases existants
//! et les ports outbound pour ne pas dupliquer la logique.

use std::sync::Arc;
use std::time::Duration;
use tracing::{info, warn};
use uuid::Uuid;

use crate::application::game::config_loader::load_game_portal_config;
use sentinel_core::domain::entities::game::audit::GameAuditAction;
use sentinel_core::domain::entities::game::server::GameServerStatus;
use sentinel_core::domain::errors::DomainError;
use crate::ports::outbound::game::container_runtime::{ContainerRuntime, ContainerState};
use crate::ports::outbound::game::game_audit_repository::GameAuditRepository;
use crate::ports::outbound::game::game_server_repository::{
    GameServerRepository, GameServerRuntimeUpdate,
};
use crate::ports::outbound::game::player_session_repository::PlayerSessionRepository;
use crate::ports::outbound::game::port_allocator::{PortAllocator, PortKind};
use crate::ports::outbound::game::rcon_client::{RconClient, RconConnectionParams};
use crate::ports::outbound::system::bot_config_repository::BotConfigRepository;

const RCON_HOST: &str = "127.0.0.1";

/// Bag d'adapters pour les jobs (evite des signatures kilometriques).
pub struct JobContext {
    pub server_repo: Arc<dyn GameServerRepository>,
    pub template_repo: Arc<dyn crate::ports::outbound::game::game_template_repository::GameTemplateRepository>,
    pub audit_repo: Arc<dyn GameAuditRepository>,
    pub session_repo: Arc<dyn PlayerSessionRepository>,
    pub container_runtime: Arc<dyn ContainerRuntime>,
    pub rcon_client: Arc<dyn RconClient>,
    pub port_allocator: Arc<dyn PortAllocator>,
    pub bot_config: Arc<dyn BotConfigRepository>,
}

/// Stats retournees par chaque job (pour observabilite worker -> log API).
#[derive(Debug, serde::Serialize)]
pub struct JobReport {
    pub job: &'static str,
    pub processed: usize,
    pub errors: usize,
    pub details: serde_json::Value,
}

// ════════════════════════════════════════════════════════════════════════
// JOB 1 : HEALTH CHECK
// ════════════════════════════════════════════════════════════════════════

/// Pour chaque serveur `running`, query player count via RCON `list`. Met
/// a jour last_player_count + last_active_at, ouvre/ferme les sessions.
pub async fn run_health_check(ctx: &JobContext) -> Result<JobReport, DomainError> {
    let servers = ctx.server_repo.list_running().await?;
    let mut errors = 0usize;
    let mut details = serde_json::Map::new();

    for server in &servers {
        let cfg = load_game_portal_config(&ctx.bot_config, &server.guild_id).await?;
        if !cfg.rcon_enabled {
            continue;
        }
        let Some(port) = server.rcon_port else {
            continue;
        };
        let Some(pwd) = server.rcon_password.clone() else {
            continue;
        };
        let params = RconConnectionParams {
            host: RCON_HOST.to_string(),
            port,
            password: pwd,
            timeout_secs: 5,
        };
        let resp = match ctx.rcon_client.execute(&params, "list").await {
            Ok(r) => r,
            Err(e) => {
                warn!(error = %e, server_id = %server.id, "health rcon failed");
                errors += 1;
                continue;
            }
        };
        let (count, players) = parse_minecraft_list(&resp.raw);

        // Maj last_player_count + last_active_at si > 0
        if let Err(e) = ctx
            .server_repo
            .update_player_activity(server.id, count)
            .await
        {
            warn!(error = %e, "update_player_activity");
            errors += 1;
        }

        // Diff sessions actives <-> liste actuelle
        let active = ctx.session_repo.list_active(server.id).await?;
        let active_names: std::collections::HashSet<String> =
            active.iter().map(|s| s.player_name.clone()).collect();
        let new_names: std::collections::HashSet<String> = players.iter().cloned().collect();

        for joined in new_names.difference(&active_names) {
            if let Err(e) = ctx.session_repo.open(server.id, joined).await {
                warn!(error = %e, "open session");
                errors += 1;
            }
        }
        for left in active_names.difference(&new_names) {
            if let Err(e) = ctx.session_repo.close(server.id, left).await {
                warn!(error = %e, "close session");
                errors += 1;
            }
        }
        details.insert(server.id.to_string(), serde_json::json!(count));
    }

    Ok(JobReport {
        job: "health_check",
        processed: servers.len(),
        errors,
        details: serde_json::Value::Object(details),
    })
}

/// Parse la sortie de la commande `list` Minecraft :
/// `There are 2 of a max of 20 players online: alice, bob`
fn parse_minecraft_list(raw: &str) -> (i32, Vec<String>) {
    // Compte
    let count = raw
        .split(' ')
        .find_map(|w| w.parse::<i32>().ok())
        .unwrap_or(0);
    // Liste apres ":"
    let players: Vec<String> = if let Some(idx) = raw.find(':') {
        raw[idx + 1..]
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    } else {
        vec![]
    };
    (count, players)
}

// ════════════════════════════════════════════════════════════════════════
// JOB 2 : IDLE SHUTDOWN
// ════════════════════════════════════════════════════════════════════════

/// Stop les serveurs running dont `last_active_at` est anterieur a
/// `idle_shutdown_days` jours (override par instance ou template).
pub async fn run_idle_shutdown(ctx: &JobContext) -> Result<JobReport, DomainError> {
    let servers = ctx.server_repo.list_running().await?;
    let mut stopped = 0usize;
    let mut errors = 0usize;

    let now = chrono::Utc::now();
    for server in &servers {
        // Resoud le seuil idle (instance override -> template default).
        let cfg = load_game_portal_config(&ctx.bot_config, &server.guild_id).await?;
        let days = server
            .idle_shutdown_days
            .unwrap_or(cfg.default_idle_shutdown_days);
        if days <= 0 {
            continue;
        }
        let cutoff = now - chrono::Duration::days(days as i64);
        let last = server.last_active_at.unwrap_or(server.created_at);
        if last >= cutoff {
            continue;
        }
        if server.last_player_count > 0 {
            // Quelqu'un est connecte malgre last_active_at vieux : skip.
            continue;
        }

        info!(server_id = %server.id, days, "idle shutdown");

        // Stop via container_runtime direct (pas de RBAC : c'est le worker).
        if let Some(cid) = &server.container_id {
            if let Err(e) = ctx.container_runtime.stop_container(cid, 30).await {
                warn!(error = %e, "stop container failed");
                errors += 1;
                continue;
            }
        }
        if let Err(e) = ctx
            .server_repo
            .update_runtime(
                server.id,
                GameServerRuntimeUpdate {
                    status: Some(GameServerStatus::Stopped),
                    stopped_at_now: true,
                    ..Default::default()
                },
            )
            .await
        {
            warn!(error = %e, "update_runtime stopped");
            errors += 1;
        }
        let _ = ctx.session_repo.close_all_active(server.id).await;
        let _ = ctx
            .audit_repo
            .log(
                &server.guild_id,
                Some(server.id),
                None, // actor = system
                GameAuditAction::IdleShutdown,
                serde_json::json!({ "idle_days": days }),
            )
            .await;
        stopped += 1;
    }

    Ok(JobReport {
        job: "idle_shutdown",
        processed: stopped,
        errors,
        details: serde_json::json!({ "stopped": stopped }),
    })
}

// ════════════════════════════════════════════════════════════════════════
// JOB 3 : RECONCILER
// ════════════════════════════════════════════════════════════════════════

/// Reconcilie l'etat DB <-> Docker reel.
///   - Containers Docker avec label sentinel.managed=game-portal mais
///     pas de ligne game_servers correspondante : log warning (orphelins).
///   - Lignes game_servers avec status running mais container disparu :
///     marque error + libere les ports.
pub async fn run_reconciler(ctx: &JobContext) -> Result<JobReport, DomainError> {
    let active_servers = ctx.server_repo.list_active().await?;
    let docker_containers = ctx.container_runtime.list_managed_containers().await?;

    let mut details = serde_json::Map::new();
    let mut errors = 0usize;

    // Index containers par sentinel.server_id label.
    let docker_by_id: std::collections::HashMap<String, &_> = docker_containers
        .iter()
        .filter_map(|c| {
            c.labels
                .get("sentinel.server_id")
                .map(|sid| (sid.clone(), c))
        })
        .collect();

    // 1. DB -> Docker : serveurs marques running mais container disparu/dead.
    for s in &active_servers {
        let dc = docker_by_id.get(&s.id.to_string());
        match dc {
            None => {
                if matches!(
                    s.status,
                    GameServerStatus::Running | GameServerStatus::Starting
                ) {
                    warn!(server_id = %s.id, "container disparu, marque error");
                    let _ = ctx
                        .server_repo
                        .update_status(
                            s.id,
                            GameServerStatus::Error,
                            Some("container disparu (reconciler)"),
                        )
                        .await;
                    if let Some(p) = s.host_port {
                        let _ = ctx.port_allocator.release(PortKind::Game, p).await;
                    }
                    if let Some(p) = s.rcon_port {
                        let _ = ctx.port_allocator.release(PortKind::Rcon, p).await;
                    }
                    let _ = ctx
                        .audit_repo
                        .log(
                            &s.guild_id,
                            Some(s.id),
                            None,
                            GameAuditAction::CrashDetected,
                            serde_json::json!({}),
                        )
                        .await;
                    errors += 1;
                }
            }
            Some(c) => {
                // Container present, verifions qu'il est dans le bon etat.
                if matches!(c.state, ContainerState::Exited | ContainerState::Dead)
                    && s.status == GameServerStatus::Running
                {
                    let _ = ctx
                        .server_repo
                        .update_status(
                            s.id,
                            GameServerStatus::Stopped,
                            Some("container exited (reconciler)"),
                        )
                        .await;
                    let _ = ctx.session_repo.close_all_active(s.id).await;
                }
            }
        }
    }

    // 2. Docker -> DB : containers managed sans ligne game_servers (orphelins).
    let known_ids: std::collections::HashSet<String> =
        active_servers.iter().map(|s| s.id.to_string()).collect();
    let mut orphans = 0usize;
    for c in &docker_containers {
        if let Some(sid) = c.labels.get("sentinel.server_id") {
            if !known_ids.contains(sid) {
                warn!(container_id = %c.container_id, server_id = %sid, "orphelin Docker");
                orphans += 1;
            }
        }
    }
    details.insert("orphans".into(), serde_json::json!(orphans));
    details.insert("active_db".into(), serde_json::json!(active_servers.len()));
    details.insert(
        "managed_docker".into(),
        serde_json::json!(docker_containers.len()),
    );

    Ok(JobReport {
        job: "reconciler",
        processed: active_servers.len(),
        errors,
        details: serde_json::Value::Object(details),
    })
}

// ════════════════════════════════════════════════════════════════════════
// JOB 4 : IMAGE CLEANUP
// ════════════════════════════════════════════════════════════════════════

/// Pour chaque template du catalogue, regarde s'il existe encore des
/// serveurs actifs qui utilisent ce template. Si non, et si la derniere
/// activite est plus ancienne que `unused_image_grace_days`, supprime
/// l'image Docker. Docker refusera la suppression si un container l'utilise
/// encore (defense en profondeur).
pub async fn run_image_cleanup(ctx: &JobContext) -> Result<JobReport, DomainError> {
    // Lecture de la config global (defaut sentinel-* sans guild — on prend
    // la premiere guild qui a une config game-portal). Pour rester simple,
    // on prend les defaults via une guild fictive : ils s'appliquent sauf
    // si l'admin a override.
    let cfg = load_game_portal_config(&ctx.bot_config, "_global").await?;
    if !cfg.auto_remove_unused_images {
        return Ok(JobReport {
            job: "image_cleanup",
            processed: 0,
            errors: 0,
            details: serde_json::json!({"skipped": "auto_remove_unused_images=false"}),
        });
    }
    let grace_days = cfg.unused_image_grace_days;
    if grace_days <= 0 {
        return Ok(JobReport {
            job: "image_cleanup",
            processed: 0,
            errors: 0,
            details: serde_json::json!({"skipped": "grace_days <= 0"}),
        });
    }

    let templates = ctx.template_repo.list().await?;
    let now = chrono::Utc::now();
    let mut removed = 0usize;
    let mut errors = 0usize;
    let mut details = serde_json::Map::new();

    for tpl in &templates {
        let usage = ctx.server_repo.template_usage(tpl.id).await?;
        if usage.active_count > 0 {
            continue;
        }
        let last = match usage.last_activity_at {
            Some(t) => t,
            None => continue, // template jamais utilise, image jamais pull -> rien a faire
        };
        let cutoff = now - chrono::Duration::days(grace_days as i64);
        if last >= cutoff {
            // Activite trop recente, on respecte la grace period.
            continue;
        }

        info!(template = %tpl.slug, image = %tpl.image, days = grace_days, "image cleanup");
        match ctx.container_runtime.remove_image(&tpl.image, false).await {
            Ok(true) => {
                removed += 1;
                details.insert(tpl.slug.clone(), serde_json::json!("removed"));
                let _ = ctx
                    .audit_repo
                    .log(
                        "_global",
                        None,
                        None,
                        sentinel_core::domain::entities::game::audit::GameAuditAction::Delete,
                        serde_json::json!({
                            "kind": "image_cleanup",
                            "template": tpl.slug,
                            "image": tpl.image,
                        }),
                    )
                    .await;
            }
            Ok(false) => {
                details.insert(tpl.slug.clone(), serde_json::json!("not_present"));
            }
            Err(e) => {
                warn!(error = %e, template = %tpl.slug, "image_cleanup failed");
                errors += 1;
                details.insert(tpl.slug.clone(), serde_json::json!(format!("error: {e}")));
            }
        }
    }

    Ok(JobReport {
        job: "image_cleanup",
        processed: removed,
        errors,
        details: serde_json::Value::Object(details),
    })
}

// Re-export pour le timer du worker.
#[allow(dead_code)]
pub fn default_intervals() -> (Duration, Duration, Duration) {
    (
        Duration::from_secs(30),
        Duration::from_secs(3600),
        Duration::from_secs(3600),
    )
}

#[allow(dead_code)]
pub fn unused_uuid() -> Uuid {
    Uuid::nil()
}
