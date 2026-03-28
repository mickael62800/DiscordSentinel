use std::sync::Arc;
use std::time::Instant;

use axum::extract::{Request, State};
use axum::middleware::Next;
use axum::response::Response;

use crate::domain::entities::LogEntry;
use crate::ports::outbound::LogRepository;

pub async fn api_logger_middleware(
    State(log_repo): State<Arc<dyn LogRepository>>,
    request: Request,
    next: Next,
) -> Response {
    let method = request.method().to_string();
    let uri = request.uri().path().to_string();
    let start = Instant::now();

    let response = next.run(request).await;

    let latency = start.elapsed();
    let status = response.status().as_u16();

    let skip = uri.contains("/heartbeat") || uri == "/health";

    if !skip && (status >= 400 || latency.as_secs() >= 2 || is_mutation(&method)) {
        let level = if status >= 500 {
            "error"
        } else if status >= 400 {
            "warn"
        } else {
            "info"
        };

        let status_text = status_label(status);

        let message = format!("[{}] {} {} — {} {}", level.to_uppercase(), method, uri, status, status_text);

        let details = serde_json::json!({
            "method": method,
            "route": uri,
            "status_code": status,
            "status_text": status_text,
            "latency_ms": latency.as_millis() as u64,
        });

        let entry = LogEntry {
            id: uuid::Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
            level: level.to_string(),
            bot: "sentinel-api".to_string(),
            server: String::new(),
            message,
            category: "api".to_string(),
            details,
        };

        let repo = log_repo.clone();
        tokio::spawn(async move {
            let _ = repo.save(&entry).await;
        });
    }

    response
}

fn is_mutation(method: &str) -> bool {
    matches!(method, "POST" | "PUT" | "PATCH" | "DELETE")
}

fn status_label(status: u16) -> &'static str {
    match status {
        200 => "OK",
        201 => "Created",
        204 => "No Content",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        422 => "Unprocessable Entity",
        429 => "Too Many Requests",
        500 => "Internal Server Error",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        _ => "Unknown",
    }
}
