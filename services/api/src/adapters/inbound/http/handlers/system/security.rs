//! GET /api/security/* — surveillance attaques et integrite serveur.
//!
//! Tous les endpoints sont gates admin+ (require_role).
//! Sources :
//!   - logs : table `logs` (alimentee par api_logger_middleware)
//!   - audit_logs : table `audit_logs` (Discord events + extension audit_docker)
//!   - cert TLS : lecture du fichier /etc/letsencrypt/live/{domain}/cert.pem
//!   - fail2ban : non implemente (necessite installation host)

use axum::extract::Query;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Extension;
use axum::Json;
use serde::Deserialize;
use serde::Serialize;

use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::middleware::rbac::require_role;
use crate::adapters::inbound::http::middleware::rbac::require_superadmin;
use crate::adapters::inbound::http::middleware::rbac::RoleContext;
use crate::adapters::inbound::http::state::AppState;
use crate::domain::enums::system::role::Role;
use crate::domain::errors::DomainError;

fn forbid(s: StatusCode, msg: &str) -> ApiError {
    ApiError(if s == StatusCode::FORBIDDEN {
        DomainError::Forbidden(msg.into())
    } else {
        DomainError::Internal(msg.into())
    })
}

/// Gate admin+ pour les endpoints globaux (non scopes par guild).
/// Comme le middleware RBAC ne peut pas resoudre le role sans guild_id
/// dans l'URL, on bypass pour les superadmins (env SUPERADMIN_USER_IDS).
/// Pour les non-superadmins, il faudrait soit ajouter un guild_id dans
/// l'URL, soit passer par un endpoint scope par guild.
fn gate_admin(state: &AppState, rbac: &Option<Extension<RoleContext>>) -> Result<(), ApiError> {
    let Some(Extension(ctx)) = rbac else {
        return Err(forbid(StatusCode::FORBIDDEN, "auth requise"));
    };
    // Superadmin bypass : pour les endpoints globaux comme /api/security/*,
    // c'est le seul check possible sans contexte de guild.
    if require_superadmin(state, ctx).is_ok() {
        return Ok(());
    }
    // Sinon : require admin role explicit (necessite que le middleware ait
    // resolu ctx.role, ce qui demande un guild_id quelque part).
    require_role(ctx, Role::Admin).map_err(|s| forbid(s, "superadmin requis"))
}

// ── 1. Top IPs par requetes ─────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct WindowQuery {
    /// "1h" / "24h" / "7d", defaut 1h
    pub window: Option<String>,
    pub limit: Option<i64>,
}

fn window_to_interval(w: &str) -> &'static str {
    match w {
        "24h" => "24 hours",
        "7d" => "7 days",
        _ => "1 hour",
    }
}

#[derive(Debug, Serialize)]
pub struct TopIpEntry {
    pub client_ip: String,
    pub total: i64,
    pub failed: i64,
    pub last_seen: String,
}

pub async fn top_ips(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Query(q): Query<WindowQuery>,
) -> Result<Json<Vec<TopIpEntry>>, ApiError> {
    gate_admin(&state, &rbac)?;
    let interval = window_to_interval(q.window.as_deref().unwrap_or("1h"));
    let limit = q.limit.unwrap_or(20).clamp(1, 100);

    let sql = format!(
        "SELECT \
            COALESCE(details->>'client_ip', '-') AS ip, \
            COUNT(*)::bigint AS total, \
            SUM(CASE WHEN level IN ('warn', 'error') THEN 1 ELSE 0 END)::bigint AS failed, \
            to_char(MAX(timestamp), 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS last_seen \
         FROM logs \
         WHERE category = 'api' \
           AND timestamp > NOW() - INTERVAL '{interval}' \
           AND details->>'client_ip' IS NOT NULL \
           AND details->>'client_ip' != '-' \
         GROUP BY ip \
         ORDER BY total DESC \
         LIMIT {limit}"
    );

    let rows = sqlx::query_as::<_, (String, i64, i64, String)>(&sql)
        .fetch_all(&state.pg_pool)
        .await
        .map_err(|e| ApiError(DomainError::Internal(format!("query top_ips: {e}"))))?;

    let out = rows
        .into_iter()
        .map(|(ip, total, failed, last_seen)| TopIpEntry { client_ip: ip, total, failed, last_seen })
        .collect();
    Ok(Json(out))
}

// ── 2. Echecs d'auth (401/403) recents ──────────────────────────────────

#[derive(Debug, Serialize)]
pub struct AuthFailureEntry {
    pub timestamp: String,
    pub status_code: i64,
    pub method: String,
    pub route: String,
    pub client_ip: String,
    pub user_agent: String,
}

pub async fn auth_failures(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Query(q): Query<WindowQuery>,
) -> Result<Json<Vec<AuthFailureEntry>>, ApiError> {
    gate_admin(&state, &rbac)?;
    let interval = window_to_interval(q.window.as_deref().unwrap_or("24h"));
    let limit = q.limit.unwrap_or(100).clamp(1, 500);

    let sql = format!(
        "SELECT \
            to_char(timestamp, 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS ts, \
            COALESCE((details->>'status_code')::bigint, 0) AS status, \
            COALESCE(details->>'method', '?') AS method, \
            COALESCE(details->>'route', '?') AS route, \
            COALESCE(details->>'client_ip', '-') AS ip, \
            COALESCE(details->>'user_agent', '') AS ua \
         FROM logs \
         WHERE category = 'api' \
           AND timestamp > NOW() - INTERVAL '{interval}' \
           AND (details->>'status_code')::int IN (401, 403) \
         ORDER BY timestamp DESC \
         LIMIT {limit}"
    );

    let rows = sqlx::query_as::<_, (String, i64, String, String, String, String)>(&sql)
        .fetch_all(&state.pg_pool)
        .await
        .map_err(|e| ApiError(DomainError::Internal(format!("query auth_failures: {e}"))))?;

    let out = rows
        .into_iter()
        .map(|(ts, status, method, route, ip, ua)| AuthFailureEntry {
            timestamp: ts,
            status_code: status,
            method,
            route,
            client_ip: ip,
            user_agent: ua,
        })
        .collect();
    Ok(Json(out))
}

// ── 3. IPs bannies (lecture fichier export fail2ban) ───────────────────

#[derive(Debug, Serialize)]
pub struct Fail2banJail {
    pub name: String,
    pub total_banned: i64,
    pub banned_ips: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct BannedIpsResponse {
    pub installed: bool,
    pub updated_at: Option<String>,
    pub message: String,
    pub jails: Vec<Fail2banJail>,
}

const F2B_STATUS_PATH: &str = "/var/lib/sentinel/fail2ban-status.json";

pub async fn banned_ips(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
) -> Result<Json<BannedIpsResponse>, ApiError> {
    gate_admin(&state, &rbac)?;

    // Lit le fichier expose par le cron host /usr/local/bin/fail2ban-export.sh
    let raw = match std::fs::read_to_string(F2B_STATUS_PATH) {
        Ok(s) => s,
        Err(_) => {
            return Ok(Json(BannedIpsResponse {
                installed: false,
                updated_at: None,
                message: format!(
                    "fail2ban status non disponible. Pour activer : 1) installer fail2ban sur l'host (apt install fail2ban) ; 2) creer le script /usr/local/bin/fail2ban-export.sh + cron pour exporter dans {F2B_STATUS_PATH} ; 3) monter /var/lib/sentinel:/var/lib/sentinel:ro dans le conteneur api du compose."
                ),
                jails: vec![],
            }));
        }
    };

    #[derive(serde::Deserialize)]
    struct RawJail {
        name: String,
        total_banned: i64,
        banned_ips: String,
    }
    #[derive(serde::Deserialize)]
    struct RawStatus {
        updated_at: String,
        jails: Vec<RawJail>,
    }

    let parsed: RawStatus = serde_json::from_str(&raw).map_err(|e| {
        ApiError(DomainError::Internal(format!("parse fail2ban-status.json: {e}")))
    })?;

    let jails = parsed
        .jails
        .into_iter()
        .map(|j| Fail2banJail {
            name: j.name,
            total_banned: j.total_banned,
            banned_ips: j
                .banned_ips
                .split_whitespace()
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
                .collect(),
        })
        .collect::<Vec<_>>();

    let total: usize = jails.iter().map(|j| j.banned_ips.len()).sum();
    Ok(Json(BannedIpsResponse {
        installed: true,
        updated_at: Some(parsed.updated_at),
        message: format!("{} IPs actuellement bannies sur {} jail(s)", total, jails.len()),
        jails,
    }))
}

// ── 4. Audit log admin (actions sensibles) ──────────────────────────────

#[derive(Debug, Deserialize)]
pub struct AuditQuery {
    pub guild_id: Option<String>,
    pub event_type_prefix: Option<String>, // ex: "docker." ou "rbac."
    pub limit: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct AuditEntry {
    pub id: String,
    pub guild_id: String,
    pub event_type: String,
    pub actor_id: Option<String>,
    pub actor_name: Option<String>,
    pub target_id: Option<String>,
    pub target_name: Option<String>,
    pub details: serde_json::Value,
    pub created_at: String,
}

pub async fn audit_logs(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Query(q): Query<AuditQuery>,
) -> Result<Json<Vec<AuditEntry>>, ApiError> {
    gate_admin(&state, &rbac)?;
    let limit = q.limit.unwrap_or(100).clamp(1, 500);

    // Construction dynamique safe : seuls les noms de colonnes/tables sont
    // hardcoded, valeurs bindees via $N.
    let mut sql = String::from(
        "SELECT id::text, guild_id, event_type, actor_id, actor_name, \
                target_id, target_name, details, \
                to_char(created_at, 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') \
         FROM audit_logs WHERE 1=1",
    );
    let mut idx = 1;
    if q.guild_id.is_some() {
        sql.push_str(&format!(" AND guild_id = ${idx}"));
        idx += 1;
    }
    if q.event_type_prefix.is_some() {
        sql.push_str(&format!(" AND event_type LIKE ${idx} || '%'"));
        idx += 1;
    }
    sql.push_str(&format!(" ORDER BY created_at DESC LIMIT ${idx}"));

    let mut q_builder = sqlx::query_as::<
        _,
        (
            String,
            String,
            String,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
            serde_json::Value,
            String,
        ),
    >(&sql);
    if let Some(g) = &q.guild_id {
        q_builder = q_builder.bind(g);
    }
    if let Some(p) = &q.event_type_prefix {
        q_builder = q_builder.bind(p);
    }
    q_builder = q_builder.bind(limit);

    let rows = q_builder
        .fetch_all(&state.pg_pool)
        .await
        .map_err(|e| ApiError(DomainError::Internal(format!("query audit_logs: {e}"))))?;

    let out = rows
        .into_iter()
        .map(
            |(id, guild_id, event_type, actor_id, actor_name, target_id, target_name, details, created_at)| {
                AuditEntry {
                    id,
                    guild_id,
                    event_type,
                    actor_id,
                    actor_name,
                    target_id,
                    target_name,
                    details,
                    created_at,
                }
            },
        )
        .collect();
    Ok(Json(out))
}

// ── Ban IP : ajoute une IP a la blocklist host ──────────────────────────

#[derive(Debug, Deserialize)]
pub struct BanIpDto {
    pub ip: String,
    /// Optionnel : raison libre (ex: "trop d'echecs auth")
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct BanIpResponse {
    pub ok: bool,
    pub message: String,
}

const BANS_PENDING_PATH: &str = "/var/lib/sentinel/bans-pending.txt";

/// POST /api/security/ban-ip
/// Pattern fichier-shim : l'API ecrit l'IP dans /var/lib/sentinel/bans-pending.txt,
/// un cron host (apply-bans.sh) lit le fichier, applique `ufw deny from <IP>`,
/// puis vide le fichier.
pub async fn ban_ip(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Json(dto): Json<BanIpDto>,
) -> Result<Json<BanIpResponse>, ApiError> {
    gate_admin(&state, &rbac)?;

    let ip = dto.ip.trim();
    // Validation basique : IPv4 ou IPv6
    let parsed: Result<std::net::IpAddr, _> = ip.parse();
    if parsed.is_err() {
        return Err(ApiError(DomainError::ValidationError(format!(
            "IP invalide : {ip}"
        ))));
    }
    // Refus de banner les IPs LAN/loopback
    let p = parsed.unwrap();
    if p.is_loopback() {
        return Err(ApiError(DomainError::ValidationError(
            "Refus de bannir une IP loopback".into(),
        )));
    }
    if let std::net::IpAddr::V4(v4) = p {
        if v4.is_private() {
            return Err(ApiError(DomainError::ValidationError(
                "Refus de bannir une IP privee LAN".into(),
            )));
        }
    }

    // Append au fichier (cron host le lit + applique + vide)
    use std::fs::OpenOptions;
    use std::io::Write;
    let parent = std::path::Path::new(BANS_PENDING_PATH).parent().unwrap();
    if let Err(e) = std::fs::create_dir_all(parent) {
        return Err(ApiError(DomainError::Internal(format!("mkdir: {e}"))));
    }
    let line = format!(
        "{}\t{}\t{}\n",
        ip,
        chrono::Utc::now().to_rfc3339(),
        dto.reason.as_deref().unwrap_or("")
    );
    let mut f = OpenOptions::new()
        .create(true)
        .append(true)
        .open(BANS_PENDING_PATH)
        .map_err(|e| ApiError(DomainError::Internal(format!("open bans file: {e}"))))?;
    f.write_all(line.as_bytes())
        .map_err(|e| ApiError(DomainError::Internal(format!("write bans: {e}"))))?;

    // Audit
    let actor = rbac
        .as_ref()
        .map(|r| r.0.discord_user_id.clone())
        .unwrap_or_else(|| "unknown".to_string());
    crate::adapters::inbound::http::handlers::system::server_events::record_server_event(
        &state.pg_pool,
        &actor,
        None,
        "security.ban_ip",
        Some(ip),
        "warn",
        serde_json::json!({ "reason": dto.reason, "ip": ip }),
    )
    .await;

    Ok(Json(BanIpResponse {
        ok: true,
        message: format!(
            "IP {} ajoutee a la blocklist (sera appliquee au prochain tick du cron host)",
            ip
        ),
    }))
}

// ── Unban IP : retire une IP de la blocklist ────────────────────────────

const UNBANS_PENDING_PATH: &str = "/var/lib/sentinel/unbans-pending.txt";

pub async fn unban_ip(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Json(dto): Json<BanIpDto>,
) -> Result<Json<BanIpResponse>, ApiError> {
    gate_admin(&state, &rbac)?;

    let ip = dto.ip.trim();
    let parsed: Result<std::net::IpAddr, _> = ip.parse();
    if parsed.is_err() {
        return Err(ApiError(DomainError::ValidationError(format!(
            "IP invalide : {ip}"
        ))));
    }

    use std::fs::OpenOptions;
    use std::io::Write;
    let parent = std::path::Path::new(UNBANS_PENDING_PATH).parent().unwrap();
    if let Err(e) = std::fs::create_dir_all(parent) {
        return Err(ApiError(DomainError::Internal(format!("mkdir: {e}"))));
    }
    let line = format!(
        "{}\t{}\t{}\n",
        ip,
        chrono::Utc::now().to_rfc3339(),
        dto.reason.as_deref().unwrap_or("")
    );
    let mut f = OpenOptions::new()
        .create(true)
        .append(true)
        .open(UNBANS_PENDING_PATH)
        .map_err(|e| ApiError(DomainError::Internal(format!("open unbans file: {e}"))))?;
    f.write_all(line.as_bytes())
        .map_err(|e| ApiError(DomainError::Internal(format!("write unbans: {e}"))))?;

    let actor = rbac
        .as_ref()
        .map(|r| r.0.discord_user_id.clone())
        .unwrap_or_else(|| "unknown".to_string());
    crate::adapters::inbound::http::handlers::system::server_events::record_server_event(
        &state.pg_pool,
        &actor,
        None,
        "security.unban_ip",
        Some(ip),
        "info",
        serde_json::json!({ "reason": dto.reason, "ip": ip }),
    )
    .await;

    Ok(Json(BanIpResponse {
        ok: true,
        message: format!("IP {} retiree de la blocklist (sera applique au prochain tick)", ip),
    }))
}

// ── Lecture de fichiers JSON exposes par les cron host ──────────────────

/// Helper generique : lit un fichier JSON expose par un cron host
/// (pattern fichier-shim documente dans TODO_SECURITY_MONITORING.md).
fn read_host_json<T: for<'de> serde::Deserialize<'de>>(
    path: &str,
    feature: &str,
) -> Result<T, ApiError> {
    let raw = std::fs::read_to_string(path).map_err(|e| {
        ApiError(DomainError::NotFound(format!(
            "{feature} non disponible. Setup : sudo bash infra/scripts/setup-host-security.sh {feature}. (lecture {path}: {e})"
        )))
    })?;
    serde_json::from_str(&raw)
        .map_err(|e| ApiError(DomainError::Internal(format!("parse {path}: {e}"))))
}

// SSH failures
#[derive(Debug, Serialize, Deserialize)]
pub struct SshFailureEntry {
    pub timestamp: String,
    pub user: String,
    pub ip: String,
    pub message: String,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct SshFailuresResponse {
    pub updated_at: String,
    pub total_24h: i64,
    pub entries: Vec<SshFailureEntry>,
}

pub async fn ssh_failures(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
) -> Result<Json<SshFailuresResponse>, ApiError> {
    gate_admin(&state, &rbac)?;
    let data: SshFailuresResponse = read_host_json("/var/lib/sentinel/ssh-failures.json", "ssh-failures")?;
    Ok(Json(data))
}

// Disk trend
#[derive(Debug, Serialize, Deserialize)]
pub struct DiskTrendPoint {
    pub timestamp: String,
    pub mount: String,
    pub used_gb: f64,
    pub total_gb: f64,
    pub usage_pct: f64,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct DiskTrendResponse {
    pub updated_at: String,
    pub points: Vec<DiskTrendPoint>,
}

pub async fn disk_trend(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
) -> Result<Json<DiskTrendResponse>, ApiError> {
    gate_admin(&state, &rbac)?;
    let data: DiskTrendResponse = read_host_json("/var/lib/sentinel/disk-trend.json", "disk-trend")?;
    Ok(Json(data))
}

// Active connections
#[derive(Debug, Serialize, Deserialize)]
pub struct ConnectionEntry {
    pub state: String,
    pub local_addr: String,
    pub remote_addr: String,
    pub process: Option<String>,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct ConnectionsResponse {
    pub updated_at: String,
    pub total: i64,
    pub connections: Vec<ConnectionEntry>,
}

pub async fn active_connections(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
) -> Result<Json<ConnectionsResponse>, ApiError> {
    gate_admin(&state, &rbac)?;
    let data: ConnectionsResponse = read_host_json("/var/lib/sentinel/connections.json", "connections")?;
    Ok(Json(data))
}

// Open ports check
#[derive(Debug, Serialize, Deserialize)]
pub struct OpenPort {
    pub port: i64,
    pub protocol: String,
    pub service: Option<String>,
    pub expected: bool, // true si dans la liste blanche (80,443,22/2222)
}
#[derive(Debug, Serialize, Deserialize)]
pub struct OpenPortsResponse {
    pub updated_at: String,
    pub ports: Vec<OpenPort>,
    pub unexpected_count: i64,
}

pub async fn open_ports(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
) -> Result<Json<OpenPortsResponse>, ApiError> {
    gate_admin(&state, &rbac)?;
    let data: OpenPortsResponse = read_host_json("/var/lib/sentinel/open-ports.json", "open-ports")?;
    Ok(Json(data))
}

// Trivy vulns
#[derive(Debug, Serialize, Deserialize)]
pub struct TrivyVuln {
    pub image: String,
    pub cve: String,
    pub severity: String, // CRITICAL / HIGH / MEDIUM / LOW
    pub package: Option<String>,
    pub fixed_version: Option<String>,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct TrivyResponse {
    pub updated_at: String,
    pub critical: i64,
    pub high: i64,
    pub medium: i64,
    pub low: i64,
    pub vulnerabilities: Vec<TrivyVuln>,
}

pub async fn trivy_vulns(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
) -> Result<Json<TrivyResponse>, ApiError> {
    gate_admin(&state, &rbac)?;
    let data: TrivyResponse = read_host_json("/var/lib/sentinel/trivy.json", "trivy")?;
    Ok(Json(data))
}

// Nginx suspicious patterns
#[derive(Debug, Serialize, Deserialize)]
pub struct SuspiciousEntry {
    pub timestamp: String,
    pub ip: String,
    pub method: String,
    pub url: String,
    pub status: i64,
    pub category: String, // "scanner" | "sqli" | "xss" | "path-traversal"
    pub user_agent: String,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct SuspiciousResponse {
    pub updated_at: String,
    pub total_24h: i64,
    pub by_category: serde_json::Value,
    pub entries: Vec<SuspiciousEntry>,
}

pub async fn nginx_suspicious(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
) -> Result<Json<SuspiciousResponse>, ApiError> {
    gate_admin(&state, &rbac)?;
    let data: SuspiciousResponse = read_host_json("/var/lib/sentinel/nginx-suspicious.json", "nginx-suspicious")?;
    Ok(Json(data))
}

// TLS handshake errors
#[derive(Debug, Serialize, Deserialize)]
pub struct TlsErrorEntry {
    pub timestamp: String,
    pub client: String,
    pub error: String,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct TlsErrorsResponse {
    pub updated_at: String,
    pub total_24h: i64,
    pub entries: Vec<TlsErrorEntry>,
}

pub async fn tls_errors(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
) -> Result<Json<TlsErrorsResponse>, ApiError> {
    gate_admin(&state, &rbac)?;
    let data: TlsErrorsResponse = read_host_json("/var/lib/sentinel/tls-errors.json", "tls-errors")?;
    Ok(Json(data))
}

// File integrity
#[derive(Debug, Serialize, Deserialize)]
pub struct FileIntegrityEntry {
    pub path: String,
    pub sha256: String,
    pub modified_at: String,
    pub status: String, // "ok" | "modified" | "missing"
}
#[derive(Debug, Serialize, Deserialize)]
pub struct FileIntegrityResponse {
    pub updated_at: String,
    pub baseline_at: Option<String>,
    pub modified_count: i64,
    pub files: Vec<FileIntegrityEntry>,
}

pub async fn file_integrity(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
) -> Result<Json<FileIntegrityResponse>, ApiError> {
    gate_admin(&state, &rbac)?;
    let data: FileIntegrityResponse = read_host_json("/var/lib/sentinel/file-integrity.json", "file-integrity")?;
    Ok(Json(data))
}

// Outbound connections
#[derive(Debug, Serialize, Deserialize)]
pub struct OutboundConnection {
    pub local_addr: String,
    pub remote_addr: String,
    pub remote_host: Option<String>,
    pub process: Option<String>,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct OutboundResponse {
    pub updated_at: String,
    pub total: i64,
    pub connections: Vec<OutboundConnection>,
}

pub async fn outbound_connections(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
) -> Result<Json<OutboundResponse>, ApiError> {
    gate_admin(&state, &rbac)?;
    let data: OutboundResponse = read_host_json("/var/lib/sentinel/outbound.json", "outbound")?;
    Ok(Json(data))
}

// ── Last successful logins ──────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct SuccessfulLoginEntry {
    pub timestamp: String,
    pub discord_user_id: String,
    pub username: Option<String>,
    pub client_ip: Option<String>,
    pub user_agent: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct LimitQuery {
    pub limit: Option<i64>,
}

pub async fn last_successful_logins(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Query(q): Query<LimitQuery>,
) -> Result<Json<Vec<SuccessfulLoginEntry>>, ApiError> {
    gate_admin(&state, &rbac)?;
    let limit = q.limit.unwrap_or(20).clamp(1, 200);

    let rows = sqlx::query_as::<_, (String, String, Option<String>, Option<String>, Option<String>)>(
        "SELECT to_char(logged_at, 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"'), \
                discord_user_id, username, client_ip, user_agent \
         FROM successful_logins \
         ORDER BY logged_at DESC \
         LIMIT $1",
    )
    .bind(limit)
    .fetch_all(&state.pg_pool)
    .await
    .map_err(|e| ApiError(DomainError::Internal(format!("query: {e}"))))?;

    let out = rows
        .into_iter()
        .map(|(ts, uid, name, ip, ua)| SuccessfulLoginEntry {
            timestamp: ts,
            discord_user_id: uid,
            username: name,
            client_ip: ip,
            user_agent: ua,
        })
        .collect();
    Ok(Json(out))
}

// ── Trafic anormal : graphe req/s sur N heures ─────────────────────────

#[derive(Debug, Deserialize)]
pub struct TrafficTrendQuery {
    /// Fenetre : "1h", "6h", "24h", "7d"
    pub window: Option<String>,
    /// Bucket : taille en minutes (5 par defaut)
    pub bucket_minutes: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct TrafficDatapoint {
    pub timestamp: String,
    pub total: i64,
    pub errors: i64, // 4xx + 5xx
}

#[derive(Debug, Serialize)]
pub struct TrafficTrendResponse {
    pub datapoints: Vec<TrafficDatapoint>,
    pub baseline_avg: f64,
    pub peak: i64,
    pub peak_at: Option<String>,
    pub alert: bool,
    pub alert_reason: Option<String>,
}

pub async fn traffic_trend(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Query(q): Query<TrafficTrendQuery>,
) -> Result<Json<TrafficTrendResponse>, ApiError> {
    gate_admin(&state, &rbac)?;
    let window = q.window.as_deref().unwrap_or("24h");
    let interval = window_to_interval(window);
    let bucket_min = q.bucket_minutes.unwrap_or(5).max(1).min(60) as i64;

    // Bucket par tranches de N minutes via date_trunc + arithmetique
    let sql = format!(
        "SELECT \
            to_char(date_trunc('hour', timestamp) + \
                INTERVAL '{bucket_min} min' * \
                FLOOR(EXTRACT(MINUTE FROM timestamp) / {bucket_min}), \
                'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS bucket, \
            COUNT(*)::bigint AS total, \
            SUM(CASE WHEN level IN ('warn', 'error') THEN 1 ELSE 0 END)::bigint AS errors \
         FROM logs \
         WHERE category = 'api' \
           AND timestamp > NOW() - INTERVAL '{interval}' \
         GROUP BY bucket \
         ORDER BY bucket ASC"
    );

    let rows = sqlx::query_as::<_, (String, i64, i64)>(&sql)
        .fetch_all(&state.pg_pool)
        .await
        .map_err(|e| ApiError(DomainError::Internal(format!("query traffic: {e}"))))?;

    let datapoints: Vec<TrafficDatapoint> = rows
        .into_iter()
        .map(|(ts, total, errors)| TrafficDatapoint { timestamp: ts, total, errors })
        .collect();

    let totals: Vec<i64> = datapoints.iter().map(|d| d.total).collect();
    let n = totals.len() as f64;
    let sum: i64 = totals.iter().sum();
    let baseline_avg = if n > 0.0 { sum as f64 / n } else { 0.0 };
    let peak = totals.iter().copied().max().unwrap_or(0);
    let peak_at = datapoints
        .iter()
        .max_by_key(|d| d.total)
        .map(|d| d.timestamp.clone());

    // Alerte si pic > 3x moyenne (et data > 10 buckets pour avoir du sens)
    let alert = baseline_avg > 0.0 && datapoints.len() > 10 && (peak as f64) > baseline_avg * 3.0;
    let alert_reason = if alert {
        Some(format!(
            "Pic à {} req sur 1 bucket (3× moyenne {:.1})",
            peak, baseline_avg
        ))
    } else {
        None
    };

    Ok(Json(TrafficTrendResponse {
        datapoints,
        baseline_avg,
        peak,
        peak_at,
        alert,
        alert_reason,
    }))
}

// ── Cleanup : purge des logs de securite ────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CleanupQuery {
    /// Nb de jours a garder. 0 = tout supprimer. Defaut 0.
    #[serde(default)]
    pub older_than_days: Option<i64>,
    /// True = purger aussi audit_logs (events Discord). Defaut false.
    #[serde(default)]
    pub include_audit_logs: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct CleanupResponse {
    pub deleted_api_logs: i64,
    pub deleted_audit_logs: i64,
    pub message: String,
}

/// DELETE /api/security/cleanup
/// Supprime les entrees de logs (table `logs` cat='api') et optionnellement
/// `audit_logs`. Gate superadmin uniquement (operation destructive).
pub async fn cleanup_security_logs(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Query(q): Query<CleanupQuery>,
) -> Result<Json<CleanupResponse>, ApiError> {
    gate_admin(&state, &rbac)?;
    let days = q.older_than_days.unwrap_or(0).max(0);
    let include_audit = q.include_audit_logs.unwrap_or(false);

    let api_logs_deleted: u64 = if days == 0 {
        sqlx::query("DELETE FROM logs WHERE category = 'api'")
            .execute(&state.pg_pool)
            .await
            .map_err(|e| ApiError(DomainError::Internal(format!("delete logs: {e}"))))?
            .rows_affected()
    } else {
        sqlx::query(&format!(
            "DELETE FROM logs WHERE category = 'api' AND timestamp < NOW() - INTERVAL '{days} days'"
        ))
        .execute(&state.pg_pool)
        .await
        .map_err(|e| ApiError(DomainError::Internal(format!("delete logs: {e}"))))?
        .rows_affected()
    };

    let audit_deleted: u64 = if include_audit {
        if days == 0 {
            sqlx::query("DELETE FROM audit_logs")
                .execute(&state.pg_pool)
                .await
                .map_err(|e| ApiError(DomainError::Internal(format!("delete audit: {e}"))))?
                .rows_affected()
        } else {
            sqlx::query(&format!(
                "DELETE FROM audit_logs WHERE created_at < NOW() - INTERVAL '{days} days'"
            ))
            .execute(&state.pg_pool)
            .await
            .map_err(|e| ApiError(DomainError::Internal(format!("delete audit: {e}"))))?
            .rows_affected()
        }
    } else {
        0
    };

    let actor = rbac
        .as_ref()
        .map(|r| r.0.discord_user_id.as_str())
        .unwrap_or("unknown");
    tracing::info!(
        target: "audit::security",
        actor = actor,
        api_logs = api_logs_deleted,
        audit_logs = audit_deleted,
        days_kept = days,
        "security cleanup executed"
    );
    crate::adapters::inbound::http::handlers::system::server_events::record_server_event(
        &state.pg_pool,
        actor,
        None,
        "security.cleanup",
        Some(&format!("days={}", days)),
        if include_audit { "warn" } else { "info" },
        serde_json::json!({
            "deleted_api_logs": api_logs_deleted,
            "deleted_audit_logs": audit_deleted,
            "days_kept": days,
            "include_audit": include_audit,
        }),
    )
    .await;

    Ok(Json(CleanupResponse {
        deleted_api_logs: api_logs_deleted as i64,
        deleted_audit_logs: audit_deleted as i64,
        message: format!(
            "{} logs API + {} audit logs supprimes",
            api_logs_deleted, audit_deleted
        ),
    }))
}

// ── 5. Certificat TLS ───────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct TlsCertInfo {
    pub domain: String,
    pub issuer: String,
    pub subject: String,
    pub not_before: String,
    pub not_after: String,
    pub days_until_expiry: i64,
    pub is_expired: bool,
    pub is_warning: bool, // < 14 jours
}

pub async fn tls_cert(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
) -> Result<Json<TlsCertInfo>, ApiError> {
    gate_admin(&state, &rbac)?;
    let domain = std::env::var("WEB_DOMAIN").unwrap_or_default();
    if domain.is_empty() {
        return Err(ApiError(DomainError::Internal(
            "WEB_DOMAIN non defini en env".into(),
        )));
    }

    // 2 strategies pour recuperer le cert :
    //
    // 1. Lecture fichier /etc/letsencrypt/live/{domain}/cert.pem
    //    -> echoue si volume monte mais perms certbot "live/" en 700 root
    //
    // 2. Fallback : openssl s_client connect web:443 -showcerts
    //    -> recupere le cert directement via TLS handshake interne,
    //       independant des perms fichier
    //
    // On essaye d'abord la lecture (rapide), fallback openssl si KO.
    let path = format!("/etc/letsencrypt/live/{domain}/cert.pem");
    let pem = match std::fs::read_to_string(&path) {
        Ok(p) => p,
        Err(_) => fetch_cert_via_openssl(&domain).map_err(|e| {
            ApiError(DomainError::Internal(format!(
                "lecture cert {path} echouee + fallback openssl echec : {e}"
            )))
        })?,
    };

    let info = parse_cert(&pem)
        .map_err(|e| ApiError(DomainError::Internal(format!("parse cert: {e}"))))?;
    Ok(Json(info))
}

/// Fallback : lance `openssl s_client -connect web:443 -servername {domain}`
/// pour recuperer le cert via TLS handshake. Necessite openssl dans l'image
/// (deja dispo dans les images Debian/Alpine standards).
fn fetch_cert_via_openssl(domain: &str) -> Result<String, String> {
    use std::io::Write;
    use std::process::Command;
    use std::process::Stdio;

    // -connect web:443 = service nginx via DNS interne Docker
    // -servername = SNI pour que nginx serve le bon vhost
    // Stdin "" pour fermer la connexion immediatement apres le handshake
    let mut child = Command::new("openssl")
        .args(["s_client", "-connect", "web:443", "-servername", domain, "-showcerts"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("spawn openssl: {e}"))?;

    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(b"");
    }

    let output = child
        .wait_with_output()
        .map_err(|e| format!("wait openssl: {e}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Extraire le PREMIER bloc -----BEGIN CERTIFICATE----- ... -----END CERTIFICATE-----
    let begin = stdout
        .find("-----BEGIN CERTIFICATE-----")
        .ok_or_else(|| "BEGIN CERTIFICATE marker absent".to_string())?;
    let end_marker = "-----END CERTIFICATE-----";
    let end = stdout[begin..]
        .find(end_marker)
        .ok_or_else(|| "END CERTIFICATE marker absent".to_string())?;
    let pem = &stdout[begin..begin + end + end_marker.len()];
    Ok(pem.to_string())
}

fn parse_cert(pem: &str) -> Result<TlsCertInfo, String> {
    use x509_parser::pem::parse_x509_pem;
    use x509_parser::prelude::*;

    let (_, p) = parse_x509_pem(pem.as_bytes()).map_err(|e| format!("pem: {e}"))?;
    let (_, cert) = X509Certificate::from_der(&p.contents).map_err(|e| format!("der: {e}"))?;

    let issuer = cert.issuer().to_string();
    let subject = cert.subject().to_string();
    let nb = cert.validity().not_before;
    let na = cert.validity().not_after;

    // Conversion via ASN1Time -> chrono (pragmatique : format RFC).
    let not_before = nb.to_rfc2822().unwrap_or_else(|_| nb.to_string());
    let not_after = na.to_rfc2822().unwrap_or_else(|_| na.to_string());

    let now = chrono::Utc::now();
    let na_chrono = chrono::DateTime::<chrono::Utc>::from_timestamp(na.timestamp(), 0)
        .ok_or_else(|| "timestamp invalide".to_string())?;
    let days_until_expiry = (na_chrono - now).num_days();
    let is_expired = days_until_expiry < 0;
    let is_warning = !is_expired && days_until_expiry < 14;

    // Domaine = CN du subject
    let domain = cert
        .subject()
        .iter_common_name()
        .next()
        .and_then(|cn| cn.as_str().ok())
        .unwrap_or("")
        .to_string();

    Ok(TlsCertInfo {
        domain,
        issuer,
        subject,
        not_before,
        not_after,
        days_until_expiry,
        is_expired,
        is_warning,
    })
}
