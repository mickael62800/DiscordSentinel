use axum::http::header;
use axum::http::HeaderValue;
use axum::http::Method;
use axum::middleware;
use axum::routing::get;
use axum::routing::post;
use axum::Router;
use tower_http::compression::CompressionLayer;
use tower_http::cors::AllowOrigin;
use tower_http::cors::CorsLayer;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::request_id::MakeRequestUuid;
use tower_http::request_id::PropagateRequestIdLayer;
use tower_http::request_id::SetRequestIdLayer;
use tower_http::set_header::SetResponseHeaderLayer;
use tower_http::trace::TraceLayer;
use tracing::Span;

use super::handlers;
use super::metrics::metrics_handler;
use super::metrics::metrics_middleware;
use super::middleware::api_logger::api_logger_middleware;
use super::middleware::auth::auth_middleware;
use super::middleware::rate_limit::rate_limit_middleware;
use super::middleware::rate_limit::RateLimiter;
use super::routes;
use super::state::AppState;

fn build_cors(allowed_origins: &str) -> CorsLayer {
    let allow_origin = if allowed_origins == "*" {
        AllowOrigin::any()
    } else if allowed_origins.is_empty() {
        // Default securise : uniquement les origines Tauri + localhost dev
        tracing::info!("ALLOWED_ORIGINS non configure — utilisation des origines par defaut (Tauri + localhost)");
        AllowOrigin::list([
            "https://tauri.localhost".parse::<HeaderValue>().unwrap(),
            "http://tauri.localhost".parse::<HeaderValue>().unwrap(),
            "http://localhost:1420".parse::<HeaderValue>().unwrap(),
            "http://localhost:3000".parse::<HeaderValue>().unwrap(),
        ])
    } else {
        let origins: Vec<HeaderValue> = allowed_origins
            .split(',')
            .filter_map(|o| o.trim().parse().ok())
            .collect();
        AllowOrigin::list(origins)
    };

    CorsLayer::new()
        .allow_origin(allow_origin)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([
            header::CONTENT_TYPE,
            header::AUTHORIZATION,
            header::HeaderName::from_static("x-request-id"),
            header::HeaderName::from_static("x-discord-token"),
            header::HeaderName::from_static("x-api-key"),
        ])
        .max_age(std::time::Duration::from_secs(3600))
}

/// Compose toutes les routes protegees par auth (hors endpoints lourds).
fn protected_domain_routes() -> Router<AppState> {
    Router::new()
        // Bot-facing routes (scoring, rules, infractions) — sans /analyze (deplace dans heavy)
        .merge(routes::bot::routes())
        // App-facing routes (nested by domain)
        .merge(routes::ticket::routes())
        .merge(routes::security::routes())
        .merge(routes::automod::routes())
        .merge(routes::moderation::routes())
        .merge(routes::voice_channels::routes())
        .merge(routes::progression::routes())
        .merge(routes::stats::routes())
        // Dashboard & config routes + charts
        .merge(routes::dashboard::routes())
        // Audit logs + watched users + discord roles
        .merge(routes::audit::routes())
        // Bot persistence (fire-and-forget)
        .merge(routes::bot_persistence::routes())
        // Members + guild direct API
        .merge(routes::members::routes())
        // Coup de coude
        .merge(routes::coude::routes())
        // Games
        .merge(routes::games::routes())
        // Système + jobs async + RBAC + welcome
        .merge(routes::system::routes())
}

/// Construit le router sans rate limiter ni ConnectInfo — pour les tests d'integration.
pub fn build_for_test(state: AppState) -> Router {
    let protected = Router::new()
        // Endpoints lourds (sans rate limit en test)
        .route("/analyze", post(handlers::ai::analyze::analyze))
        .route("/analyze/image", post(handlers::ai::analyze_image::analyze_image))
        .merge(routes::analytics::routes())
        // Routes standard
        .merge(protected_domain_routes())
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ));

    let public = Router::new()
        .route("/health", get(handlers::system::health::health))
        .route("/auth/discord/authorize", get(handlers::system::oauth::authorize))
        .route("/auth/discord/callback", get(handlers::system::oauth::callback));

    Router::new()
        .merge(protected)
        .merge(public)
        .with_state(state)
}

pub fn build(state: AppState, max_body_size: usize, rate_limit_per_sec: u64, allowed_origins: &str) -> Router {
    let limiter = RateLimiter::new(rate_limit_per_sec);

    // Limiter strict pour les endpoints lourds (inference IA, analytics)
    // Par defaut : 5 req/s (burst 50) vs standard qui est typiquement 50-100 req/s
    let heavy_rate: u64 = std::env::var("HEAVY_RATE_LIMIT_PER_SEC")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(5);
    let heavy_limiter = RateLimiter::new(heavy_rate);

    // Spawn cleanup tasks (purge stale IP buckets every 60s)
    let limiter_cleanup = limiter.clone();
    let heavy_cleanup = heavy_limiter.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
            limiter_cleanup.cleanup().await;
            heavy_cleanup.cleanup().await;
        }
    });

    // Routes lourdes avec rate limit strict (inference IA + analytics)
    let heavy_routes = Router::new()
        .route("/analyze", post(handlers::ai::analyze::analyze))
        .route("/analyze/image", post(handlers::ai::analyze_image::analyze_image))
        .merge(routes::analytics::routes())
        .route_layer(middleware::from_fn_with_state(
            heavy_limiter,
            rate_limit_middleware,
        ));

    // Routes protegees par auth + rate limit standard
    let protected = Router::new()
        // Routes lourdes (limiter strict)
        .merge(heavy_routes)
        // Toutes les routes de domaine protegees
        .merge(protected_domain_routes())
        // Phase 7 B — RBAC fin : enrichit la requete avec le role du user
        // sur la guild courante (extension RoleContext). Doit tourner apres
        // guild_auth pour reutiliser le meme flow de token.
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            crate::adapters::inbound::http::middleware::rbac::rbac_middleware,
        ))
        // Phase 2 B — Multi-tenant : filtre les requetes par appartenance
        // Discord du user appelant (header X-Discord-Token). Pass-through si
        // header absent (appel bot/internal). Doit etre apres auth_middleware
        // pour beneficier de la validation API key d'abord.
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            crate::adapters::inbound::http::middleware::guild_auth::guild_auth_middleware,
        ))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ))
        .route_layer(middleware::from_fn_with_state(
            limiter,
            rate_limit_middleware,
        ));

    // Routes publiques (health + métriques Prometheus pour scraping)
    //
    // ⚠️ `/metrics` est volontairement public — Prometheus scrape sans auth.
    // Pour restreindre en prod, faire un firewall sur l'IP du Prometheus ou
    // ajouter une couche basic auth via reverse proxy.
    let public = Router::new()
        .route("/health", get(handlers::system::health::health))
        .route("/metrics", get(metrics_handler))
        // OAuth Discord web : publiques car pas de token prealable.
        // Le state CSRF + l'echange code cote serveur protegent le flux.
        .route("/auth/discord/authorize", get(handlers::system::oauth::authorize))
        .route("/auth/discord/callback", get(handlers::system::oauth::callback));

    // Helper : true pour les endpoints bruyants (heartbeat des bots toutes
    // les 1-3s, /health du frontend toutes les 90s). On veut les voir en
    // DEBUG pour ne pas polluer les logs INFO.
    fn is_low_verbosity_path(p: &str) -> bool {
        p.contains("/heartbeat") || p == "/health"
    }

    // TraceLayer configure pour inclure le request_id dans chaque span
    let trace_layer = TraceLayer::new_for_http().make_span_with(|request: &axum::http::Request<_>| {
        let request_id = request
            .headers()
            .get("x-request-id")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("-");
        let path = request.uri().path();
        let low = is_low_verbosity_path(path);

        if low {
            tracing::debug_span!(
                "http_request",
                method = %request.method(),
                uri = %request.uri(),
                request_id = %request_id,
                low_verbosity = true,
            )
        } else {
            tracing::info_span!(
                "http_request",
                method = %request.method(),
                uri = %request.uri(),
                request_id = %request_id,
            )
        }
    }).on_response(|response: &axum::http::Response<_>, latency: std::time::Duration, span: &Span| {
        let status = response.status().as_u16();
        let latency_ms = latency.as_millis() as u64;
        // Si la span est marquee low_verbosity (heartbeat/health), on emet en DEBUG.
        // Sinon INFO. tracing-subscriber filtre selon RUST_LOG.
        if span.field("low_verbosity").is_some() {
            tracing::debug!(status = status, latency_ms = latency_ms, "response");
        } else {
            tracing::info!(status = status, latency_ms = latency_ms, "response");
        }
    });

    let log_repo = state.log_repo.clone();

    Router::new()
        .merge(protected)
        .merge(public)
        .layer(middleware::from_fn_with_state(log_repo, api_logger_middleware))
        // Métriques Prometheus : enregistre count + latency par (route, method, status).
        // Doit s'appliquer APRÈS le matching de route pour récupérer le `MatchedPath`.
        .layer(middleware::from_fn(metrics_middleware))
        // Phase 1 — Quick wins : compression HTTP (zstd préféré, gzip fallback).
        // S'applique sur toutes les réponses dont le client envoie un Accept-Encoding
        // compatible. Gain typique : -60 % de bande passante sur les payloads JSON
        // (la plupart de nos endpoints retournent du JSON très répétitif). Le coût
        // CPU côté serveur est négligeable à zstd niveau 3.
        .layer(CompressionLayer::new().zstd(true).gzip(true))
        .layer(PropagateRequestIdLayer::x_request_id())
        .layer(RequestBodyLimitLayer::new(max_body_size))
        .layer(trace_layer)
        .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
        // Security headers
        .layer(SetResponseHeaderLayer::overriding(
            header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::X_FRAME_OPTIONS,
            HeaderValue::from_static("DENY"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::HeaderName::from_static("x-xss-protection"),
            HeaderValue::from_static("1; mode=block"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::HeaderName::from_static("referrer-policy"),
            HeaderValue::from_static("strict-origin-when-cross-origin"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::HeaderName::from_static("permissions-policy"),
            HeaderValue::from_static("camera=(), microphone=(), geolocation=()"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::CACHE_CONTROL,
            HeaderValue::from_static("no-store"),
        ))
        // Content-Security-Policy strict : l'API ne sert que du JSON, aucune
        // execution de script / chargement de ressource n'est legitime sur ce
        // domaine. Bloque tout XSS reflechi residuel.
        .layer(SetResponseHeaderLayer::overriding(
            header::CONTENT_SECURITY_POLICY,
            HeaderValue::from_static(
                "default-src 'none'; frame-ancestors 'none'; base-uri 'none'; form-action 'none'",
            ),
        ))
        .layer(build_cors(allowed_origins))
        .with_state(state)
}
