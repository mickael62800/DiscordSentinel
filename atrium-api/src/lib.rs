//! Contrat HTTP et client DeepSeek de l'accueil IA Atrium.
//!
//! L'API ne parle pas a Discord : `atrium-bot` lui transmet un message deja
//! filtre. Cela garde les permissions Discord et les actions sensibles hors du
//! perimetre du modele.

use std::{net::SocketAddr, sync::Arc};

use async_trait::async_trait;
use atrium_core::{
    application::WelcomeService,
    domain::{ConversationScope, WelcomeError, WelcomePrompt, WelcomeRequest},
    ports::{
        inbound::GenerateWelcomeReplyUseCase,
        outbound::{AiProviderError, WelcomeAiGateway},
    },
};
use axum::{
    extract::Extension,
    http::{header::AUTHORIZATION, HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use platform_common_api::{rate_limit_middleware, RateLimiter};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

pub mod rag;

const DEEPSEEK_URL: &str = "https://api.deepseek.com/chat/completions";

#[derive(Clone)]
pub struct AppState {
    pub config: AppConfig,
    pub welcome: Arc<dyn GenerateWelcomeReplyUseCase>,
    pub rag: Option<Arc<rag::RagService>>,
}

#[derive(Clone)]
pub struct AppConfig {
    pub bind_addr: SocketAddr,
    pub grpc_addr: SocketAddr,
    pub rag_database_url: String,
    pub embeddings_base_url: String,
    pub embeddings_api_key: Option<String>,
    pub embeddings_model: String,
    api_token: String,
    deepseek_api_key: String,
    model: String,
}

impl AppConfig {
    pub fn dummy() -> Self {
        Self {
            bind_addr: "127.0.0.1:8090".parse().unwrap(),
            grpc_addr: "127.0.0.1:8091".parse().unwrap(),
            rag_database_url: "postgres://localhost/test".into(),
            embeddings_base_url: "http://127.0.0.1:11434/v1".into(),
            embeddings_api_key: None,
            embeddings_model: "nomic-embed-text".into(),
            api_token: "test-token".into(),
            deepseek_api_key: "test-ds-key".into(),
            model: "deepseek-v4-flash".into(),
        }
    }

    pub fn from_env() -> Result<Self, String> {
        let bind_addr = std::env::var("ATRIUM_API_BIND_ADDR")
            .unwrap_or_else(|_| "0.0.0.0:8090".into())
            .parse()
            .map_err(|_| "ATRIUM_API_BIND_ADDR invalide".to_owned())?;
        let required =
            |key: &str| std::env::var(key).map_err(|_| format!("variable {key} manquante"));
        Ok(Self {
            bind_addr,
            grpc_addr: std::env::var("ATRIUM_GRPC_BIND_ADDR")
                .unwrap_or_else(|_| "0.0.0.0:8091".into())
                .parse()
                .map_err(|_| "ATRIUM_GRPC_BIND_ADDR invalide".to_owned())?,
            rag_database_url: required("ATRIUM_RAG_DATABASE_URL")?,
            embeddings_base_url: std::env::var("ATRIUM_EMBEDDINGS_BASE_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:11434/v1".into()),
            embeddings_api_key: std::env::var("ATRIUM_EMBEDDINGS_API_KEY")
                .ok()
                .filter(|value| !value.trim().is_empty()),
            embeddings_model: std::env::var("ATRIUM_EMBEDDINGS_MODEL")
                .unwrap_or_else(|_| "nomic-embed-text".into()),
            api_token: required("ATRIUM_API_TOKEN")?,
            deepseek_api_key: required("DEEPSEEK_API_KEY")?,
            model: std::env::var("DEEPSEEK_MODEL").unwrap_or_else(|_| "deepseek-v4-flash".into()),
        })
    }
}

/// Applique les migrations Atrium avant de rendre l'API disponible.
///
/// La base doit etre dediee a Atrium : SQLx partage sa table
/// `_sqlx_migrations` par base et Sentinel possede deja une migration `001`.
pub async fn run_migrations(config: &AppConfig) -> Result<(), sqlx::Error> {
    let pool = PgPool::connect(&config.rag_database_url).await?;
    sqlx::migrate!("./migrations").run(&pool).await?;
    tracing::info!("Migrations Atrium appliquees");
    Ok(())
}

pub fn router(config: AppConfig, rag: Arc<rag::RagService>) -> Router {
    let state = Arc::new(AppState {
        welcome: welcome_use_case(&config),
        rag: Some(rag),
        config,
    });
    router_with_state(state)
}

pub fn router_with_state(state: Arc<AppState>) -> Router {
    let rate_limiter = RateLimiter::new(
        std::env::var("ATRIUM_API_RATE_LIMIT_PER_SEC")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(5),
    );
    let router = Router::new()
        .route("/health", get(health))
        .route("/v1/welcome/reply", post(welcome_reply))
        .layer(Extension(state))
        .layer(axum::middleware::from_fn(
            platform_common_api::metrics::metrics_middleware,
        ))
        .route_layer(axum::middleware::from_fn_with_state(
            rate_limiter.clone(),
            rate_limit_middleware,
        ));

    platform_common_api::http::security_headers(router).with_state(rate_limiter)
}

pub fn welcome_use_case(config: &AppConfig) -> Arc<dyn GenerateWelcomeReplyUseCase> {
    let ai: Arc<dyn WelcomeAiGateway> = Arc::new(DeepSeekGateway {
        client: Client::new(),
        api_key: config.deepseek_api_key.clone(),
        model: config.model.clone(),
    });
    Arc::new(WelcomeService::new(ai))
}

pub mod grpc;

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
}

#[derive(Debug, Deserialize)]
pub struct WelcomeReplyRequest {
    pub guild_id: String,
    pub member: NewMember,
    pub channel: WelcomeChannel,
    /// Message facultatif du nouveau membre. Il est borne avant envoi au modele.
    #[serde(default)]
    pub message: String,
    /// Contexte configure par les administrateurs : regles, salons, FAQ.
    #[serde(default)]
    pub server_context: String,
}

#[derive(Debug, Deserialize)]
pub struct NewMember {
    pub id: String,
    pub display_name: String,
}

#[derive(Debug, Deserialize)]
pub struct WelcomeChannel {
    pub id: String,
    pub kind: ChannelKind,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ChannelKind {
    General,
    Direct,
}

#[derive(Serialize)]
pub struct WelcomeReplyResponse {
    pub reply: String,
    pub model: String,
    pub generated_by_ai: bool,
}

async fn welcome_reply(
    Extension(state): Extension<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<WelcomeReplyRequest>,
) -> Result<Json<WelcomeReplyResponse>, ApiError> {
    authorize(&headers, &state.config)?;
    let retrieved = match &state.rag {
        Some(rag) => rag
            .context_for(&request.guild_id, &request.message)
            .await
            .map_err(|error| {
                tracing::warn!(%error, "Recherche RAG indisponible");
                ApiError::bad_request("recherche de connaissances indisponible")
            })?,
        None => String::new(),
    };
    let reply = state
        .welcome
        .reply(WelcomeRequest {
            guild_id: request.guild_id,
            member_id: request.member.id,
            member_display_name: request.member.display_name,
            channel_id: request.channel.id,
            scope: request.channel.kind.into(),
            member_message: request.message,
            server_context: merge_context(&request.server_context, &retrieved),
        })
        .await
        .map_err(ApiError::from)?;
    Ok(Json(WelcomeReplyResponse {
        reply: reply.content,
        model: state.config.model.clone(),
        generated_by_ai: reply.generated_by_ai,
    }))
}

pub fn merge_context(admin_context: &str, retrieved: &str) -> String {
    format!("{}\n\n{}", admin_context.trim(), retrieved.trim())
        .chars()
        .take(12_000)
        .collect()
}

fn authorize(headers: &HeaderMap, config: &AppConfig) -> Result<(), ApiError> {
    let expected = format!("Bearer {}", config.api_token);
    let supplied = headers.get(AUTHORIZATION).and_then(|v| v.to_str().ok());
    if supplied == Some(expected.as_str()) {
        Ok(())
    } else {
        Err(ApiError::unauthorized())
    }
}

impl From<ChannelKind> for ConversationScope {
    fn from(value: ChannelKind) -> Self {
        match value {
            ChannelKind::General => Self::General,
            ChannelKind::Direct => Self::Direct,
        }
    }
}

#[derive(Serialize)]
struct DeepSeekRequest {
    model: String,
    messages: Vec<DeepSeekMessage>,
    max_tokens: u16,
    temperature: f32,
}
#[derive(Serialize)]
struct DeepSeekMessage {
    role: &'static str,
    content: String,
}
#[derive(Deserialize)]
struct DeepSeekResponse {
    choices: Vec<DeepSeekChoice>,
}
#[derive(Deserialize)]
struct DeepSeekChoice {
    message: DeepSeekResponseMessage,
}
#[derive(Deserialize)]
struct DeepSeekResponseMessage {
    content: Option<String>,
}

struct DeepSeekGateway {
    client: Client,
    api_key: String,
    model: String,
}

#[async_trait]
impl WelcomeAiGateway for DeepSeekGateway {
    async fn generate(&self, prompt: WelcomePrompt) -> Result<String, AiProviderError> {
        let body = DeepSeekRequest {
            model: self.model.clone(),
            messages: vec![
                DeepSeekMessage {
                    role: "system",
                    content: prompt.system,
                },
                DeepSeekMessage {
                    role: "user",
                    content: prompt.user,
                },
            ],
            max_tokens: 350,
            temperature: 0.4,
        };
        let response = self
            .client
            .post(DEEPSEEK_URL)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|_| AiProviderError)?;
        if !response.status().is_success() {
            return Err(AiProviderError);
        }
        let payload: DeepSeekResponse = response.json().await.map_err(|_| AiProviderError)?;
        payload
            .choices
            .into_iter()
            .next()
            .and_then(|choice| choice.message.content)
            .filter(|content| !content.trim().is_empty())
            .ok_or(AiProviderError)
    }
}

pub struct ApiError {
    status: StatusCode,
    message: &'static str,
}
impl ApiError {
    fn bad_request(message: &'static str) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message,
        }
    }
    fn unauthorized() -> Self {
        Self {
            status: StatusCode::UNAUTHORIZED,
            message: "jeton API invalide",
        }
    }
}
impl From<WelcomeError> for ApiError {
    fn from(_: WelcomeError) -> Self {
        Self::bad_request("requete d'accueil invalide")
    }
}
impl axum::response::IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        (
            self.status,
            Json(serde_json::json!({"error": self.message})),
        )
            .into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn maps_channel_kinds_to_business_scopes() {
        assert_eq!(
            ConversationScope::from(ChannelKind::General),
            ConversationScope::General
        );
        assert_eq!(
            ConversationScope::from(ChannelKind::Direct),
            ConversationScope::Direct
        );
    }
}
