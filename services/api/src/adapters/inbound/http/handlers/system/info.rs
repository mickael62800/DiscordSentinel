//! GET /api/system/info — etat detaille du systeme pour le panneau d'admin web.
//!
//! Retourne :
//!   - la liste nominative des bots/workers connus avec leur etat online,
//!   - les metriques CPU/RAM de l'host (necessite `pid: host` dans compose),
//!   - les metriques CPU/RAM du process API lui-meme,
//!   - l'uptime du process API,
//!   - la taille de la base de donnees PostgreSQL.
//!
//! Sources :
//!   - `bots:known` (Redis SET) + `bot:online:{name}` (EXISTS avec TTL 90s)
//!     pour la liste + etat des services.
//!   - `sysinfo` pour CPU/RAM (host grace au PID namespace partage).
//!   - `STARTED_AT` (OnceLock initialise au demarrage) pour l'uptime.
//!   - `pg_database_size(current_database())` pour la taille BDD.

use std::sync::OnceLock;
use std::time::Instant;

use axum::extract::State;
use axum::Json;
use redis::AsyncCommands;
use serde::Serialize;
use sysinfo::ProcessRefreshKind;
use sysinfo::RefreshKind;
use sysinfo::System;
use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::state::AppState;

/// Moment de demarrage du process API. Initialise une seule fois au premier
/// appel (ou explicitement depuis main.rs via `record_startup()`).
static STARTED_AT: OnceLock<Instant> = OnceLock::new();

/// A appeler depuis main.rs pour fixer l'uptime reel. Si non appele, le
/// premier appel a l'endpoint fixera la valeur.
pub fn record_startup() {
    let _ = STARTED_AT.set(Instant::now());
}

fn uptime_seconds() -> u64 {
    STARTED_AT
        .get_or_init(Instant::now)
        .elapsed()
        .as_secs()
}

#[derive(Debug, Serialize)]
pub struct ServiceStatusDto {
    pub name: String,
    pub online: bool,
}

#[derive(Debug, Serialize)]
pub struct HostMetricsDto {
    pub cpu_percent: f32,
    pub cpu_cores: usize,
    pub mem_used_mb: u64,
    pub mem_total_mb: u64,
}

#[derive(Debug, Serialize)]
pub struct ProcessMetricsDto {
    pub cpu_percent: f32,
    pub mem_used_mb: u64,
}

#[derive(Debug, Serialize, Default)]
pub struct RedisMetricsDto {
    pub used_memory_mb: u64,
    pub connected_clients: u64,
    pub total_keys: u64,
    pub uptime_seconds: u64,
}

#[derive(Debug, Serialize)]
pub struct SystemInfoDto {
    pub bots: Vec<ServiceStatusDto>,
    pub workers: Vec<ServiceStatusDto>,
    pub host: HostMetricsDto,
    pub process: ProcessMetricsDto,
    pub redis: RedisMetricsDto,
    pub uptime_seconds: u64,
    pub db_size_mb: u64,
}

/// Parse la sortie de `INFO` Redis (format "key:value" par ligne) et
/// extrait les champs qui nous interessent.
fn parse_redis_info(raw: &str) -> RedisMetricsDto {
    let mut dto = RedisMetricsDto::default();
    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((k, v)) = line.split_once(':') else {
            continue;
        };
        match k {
            "used_memory" => {
                if let Ok(bytes) = v.parse::<u64>() {
                    dto.used_memory_mb = bytes / 1024 / 1024;
                }
            }
            "connected_clients" => {
                dto.connected_clients = v.parse().unwrap_or(0);
            }
            "uptime_in_seconds" => {
                dto.uptime_seconds = v.parse().unwrap_or(0);
            }
            k if k.starts_with("db") => {
                // Ex: "db0:keys=1234,expires=56,avg_ttl=789"
                if let Some(keys_part) = v.split(',').find(|p| p.starts_with("keys=")) {
                    if let Some(n) = keys_part.strip_prefix("keys=") {
                        dto.total_keys += n.parse::<u64>().unwrap_or(0);
                    }
                }
            }
            _ => {}
        }
    }
    dto
}

pub async fn get_system_info(
    State(state): State<AppState>,
) -> Result<Json<SystemInfoDto>, ApiError> {
    // ── 1. Liste nominative + metriques Redis ──
    let (mut bots, mut workers) = (Vec::new(), Vec::new());
    let mut redis_metrics = RedisMetricsDto::default();
    if let Ok(mut conn) = state.redis_client.get_multiplexed_async_connection().await {
        let known: Vec<String> = conn.smembers("bots:known").await.unwrap_or_default();
        for name in known {
            let online: bool = conn
                .exists::<_, bool>(format!("bot:online:{}", name))
                .await
                .unwrap_or(false);
            let entry = ServiceStatusDto {
                name: name.clone(),
                online,
            };
            if name.contains("worker") {
                workers.push(entry);
            } else {
                bots.push(entry);
            }
        }

        // INFO Redis — memoire, clients, uptime, nb de cles
        let raw: String = redis::cmd("INFO")
            .query_async(&mut conn)
            .await
            .unwrap_or_default();
        redis_metrics = parse_redis_info(&raw);
    }
    bots.sort_by(|a, b| a.name.cmp(&b.name));
    workers.sort_by(|a, b| a.name.cmp(&b.name));

    // ── 2. Metriques systeme via sysinfo ──
    // Necessite `pid: host` dans docker-compose pour voir les stats reelles
    // de la machine et pas seulement celles du conteneur.
    let mut sys = System::new_with_specifics(
        RefreshKind::nothing()
            .with_cpu(sysinfo::CpuRefreshKind::everything())
            .with_memory(sysinfo::MemoryRefreshKind::everything())
            .with_processes(ProcessRefreshKind::everything()),
    );
    // sysinfo a besoin de 2 refresh espaces pour calculer un delta CPU fiable.
    sys.refresh_cpu_usage();
    tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    sys.refresh_cpu_usage();
    sys.refresh_memory();
    sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

    let cpu_percent = sys.global_cpu_usage();
    let cpu_cores = sys.cpus().len();
    let mem_used_mb = sys.used_memory() / 1024 / 1024;
    let mem_total_mb = sys.total_memory() / 1024 / 1024;

    // Process API : on utilise le PID courant.
    let (proc_cpu, proc_mem_mb) = {
        let pid = sysinfo::get_current_pid().ok();
        match pid.and_then(|p| sys.process(p)) {
            Some(p) => (p.cpu_usage(), p.memory() / 1024 / 1024),
            None => (0.0, 0),
        }
    };

    // ── 3. Taille BDD PostgreSQL ──
    let db_size_bytes: i64 = sqlx::query_scalar("SELECT pg_database_size(current_database())")
        .fetch_one(&state.pg_pool)
        .await
        .unwrap_or(0);
    let db_size_mb = (db_size_bytes / 1024 / 1024) as u64;

    Ok(Json(SystemInfoDto {
        bots,
        workers,
        host: HostMetricsDto {
            cpu_percent,
            cpu_cores,
            mem_used_mb,
            mem_total_mb,
        },
        process: ProcessMetricsDto {
            cpu_percent: proc_cpu,
            mem_used_mb: proc_mem_mb,
        },
        redis: redis_metrics,
        uptime_seconds: uptime_seconds(),
        db_size_mb,
    }))
}

#[cfg(test)]
#[path = "tests/info.rs"]
mod tests;
