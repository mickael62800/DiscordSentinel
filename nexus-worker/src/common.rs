//! Helpers partages du worker Nexus.
//!
//! Porte minimal de `sentinel-worker/src/common`. Sentinel poussait les logs
//! worker vers `POST /api/logs` ; nexus-api n'expose pas (encore) d'ingestion
//! de logs, donc on emet en local via `tracing`. Le jour ou l'endpoint
//! existera, seule cette fonction change.

/// Remonte un evenement de job worker. Sortie `tracing` locale.
pub fn send_worker_log(
    worker_name: &str,
    level: &str,
    job_name: &str,
    message: &str,
    details: serde_json::Value,
) {
    match level {
        "error" => tracing::error!(worker = worker_name, job = job_name, ?details, "{message}"),
        "warn" => tracing::warn!(worker = worker_name, job = job_name, ?details, "{message}"),
        _ => tracing::info!(worker = worker_name, job = job_name, ?details, "{message}"),
    }
}
