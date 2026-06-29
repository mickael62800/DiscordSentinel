use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::Instant;

use axum::extract::ConnectInfo;
use axum::extract::Request;
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::IntoResponse;
use axum::response::Response;
use tokio::sync::Mutex;

/// Simple in-memory token bucket rate limiter per IP address.
#[derive(Clone)]
pub struct RateLimiter {
    inner: Arc<Mutex<RateLimiterInner>>,
    max_tokens: u64,
    refill_per_sec: u64,
}

struct RateLimiterInner {
    buckets: HashMap<IpAddr, Bucket>,
}

struct Bucket {
    tokens: u64,
    last_refill: Instant,
}

impl RateLimiter {
    pub fn new(requests_per_sec: u64) -> Self {
        Self {
            inner: Arc::new(Mutex::new(RateLimiterInner {
                buckets: HashMap::new(),
            })),
            max_tokens: requests_per_sec * 10, // burst = 10x per-second rate
            refill_per_sec: requests_per_sec,
        }
    }

    async fn check(&self, ip: IpAddr) -> bool {
        let mut inner = self.inner.lock().await;
        let now = Instant::now();

        // Cap: refuse les nouvelles IPs au-dela de 50 000 entrees pour eviter l'OOM
        if !inner.buckets.contains_key(&ip) && inner.buckets.len() >= 50_000 {
            // Nettoyage d'urgence des entrees expirees
            inner
                .buckets
                .retain(|_, b| now.duration_since(b.last_refill).as_secs() < 120);
            if inner.buckets.len() >= 50_000 {
                return false;
            }
        }

        let bucket = inner.buckets.entry(ip).or_insert(Bucket {
            tokens: self.max_tokens,
            last_refill: now,
        });

        // Refill tokens based on elapsed time
        let elapsed = now.duration_since(bucket.last_refill).as_secs_f64();
        let refill = (elapsed * self.refill_per_sec as f64) as u64;
        if refill > 0 {
            bucket.tokens = (bucket.tokens + refill).min(self.max_tokens);
            bucket.last_refill = now;
        }

        // Consume a token
        if bucket.tokens > 0 {
            bucket.tokens -= 1;
            true
        } else {
            false
        }
    }

    /// Periodically clean up stale entries (call every ~60s)
    pub async fn cleanup(&self) {
        let mut inner = self.inner.lock().await;
        let now = Instant::now();
        inner
            .buckets
            .retain(|_, b| now.duration_since(b.last_refill).as_secs() < 120);
    }
}

/// Extrait l'IP cliente en priorisant X-Forwarded-For (derriere un reverse
/// proxy) puis X-Real-IP, puis la socket directe. Sans ca, derriere un proxy
/// toutes les requetes viennent de 127.0.0.1 et le rate limit s'applique
/// globalement au lieu de par-client.
fn client_ip(request: &Request, fallback: IpAddr) -> IpAddr {
    if let Some(xff) = request.headers().get("x-forwarded-for") {
        if let Ok(s) = xff.to_str() {
            // X-Forwarded-For peut contenir plusieurs IPs "client, proxy1, proxy2"
            if let Some(first) = s.split(',').next() {
                if let Ok(ip) = first.trim().parse::<IpAddr>() {
                    return ip;
                }
            }
        }
    }
    if let Some(xri) = request.headers().get("x-real-ip") {
        if let Ok(s) = xri.to_str() {
            if let Ok(ip) = s.trim().parse::<IpAddr>() {
                return ip;
            }
        }
    }
    fallback
}

pub async fn rate_limit_middleware(
    ConnectInfo(addr): ConnectInfo<std::net::SocketAddr>,
    axum::extract::State(limiter): axum::extract::State<RateLimiter>,
    request: Request,
    next: Next,
) -> Response {
    let ip = client_ip(&request, addr.ip());
    if limiter.check(ip).await {
        next.run(request).await
    } else {
        (
            StatusCode::TOO_MANY_REQUESTS,
            [("Retry-After", "1")],
            "Rate limit exceeded",
        )
            .into_response()
    }
}

#[cfg(test)]
#[path = "tests/rate_limit.rs"]
mod tests;
