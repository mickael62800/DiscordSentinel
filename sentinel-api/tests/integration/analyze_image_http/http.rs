//! Tests d'integration HTTP pour POST /analyze/image.

#[path = "../../test_helpers.rs"]
mod test_helpers;

use std::sync::Arc;

use async_trait::async_trait;
use axum::body::Body;
use axum::http::Request;
use axum::http::StatusCode;
use base64::Engine;
use http_body_util::BodyExt;
use tower::ServiceExt;

use sentinel_api::adapters::inbound::http::router;
use sentinel_core::ports::inbound::ai::analyze_image::AnalyzeImageCommand;
use sentinel_core::ports::inbound::ai::analyze_image::AnalyzeImageUseCase;
use sentinel_core::domain::entities::ai::image_analysis::ImageAnalysis;
use sentinel_core::domain::entities::ai::image_analysis::ImageClassification;
use sentinel_core::domain::enums::moderation::action::Action;
use sentinel_core::domain::errors::DomainError;
struct OkAnalyzeImage;
#[async_trait]
impl AnalyzeImageUseCase for OkAnalyzeImage {
    async fn analyze_image(&self, _: AnalyzeImageCommand) -> Result<ImageAnalysis, DomainError> {
        Ok(ImageAnalysis {
            action: Action::None,
            reason: String::new(),
            score: 0.0,
            duration: None,
            classifications: vec![ImageClassification {
                label: "safe".into(),
                confidence: 0.99,
            }],
        })
    }
}

async fn post(app: axum::Router, body: serde_json::Value) -> (StatusCode, serde_json::Value) {
    let req = Request::builder()
        .method("POST")
        .uri("/analyze/image")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&body).unwrap()))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    let s = resp.status();
    let b = resp.into_body().collect().await.unwrap().to_bytes();
    (
        s,
        serde_json::from_slice(&b).unwrap_or(serde_json::Value::Null),
    )
}

fn payload(image_data: &str, content_type: &str) -> serde_json::Value {
    serde_json::json!({
        "guild_id": "111111111111111111",
        "channel_id": "222222222222222222",
        "user_id": "333333333333333333",
        "username": "bob",
        "message_id": "444444444444444444",
        "image_data": image_data,
        "content_type": content_type,
        "filename": "x.png",
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn analyze_image_happy_path() {
    let mut state = test_helpers::build_test_state(Arc::new(test_helpers::StubVoiceChannels));
    state.ai.analyze_image_uc = Arc::new(OkAnalyzeImage);
    let app = router::build_for_test(state);
    let data = base64::engine::general_purpose::STANDARD.encode(b"fake-png-bytes");
    let (status, json) = post(app, payload(&data, "image/png")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["action"], "none");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn analyze_image_rejects_oversized() {
    let mut state = test_helpers::build_test_state(Arc::new(test_helpers::StubVoiceChannels));
    state.ai.analyze_image_uc = Arc::new(OkAnalyzeImage);
    let app = router::build_for_test(state);
    let data = "A".repeat(14_000_001);
    let (status, _) = post(app, payload(&data, "image/png")).await;
    // Axum peut rejeter avec 413 (body-limit) avant meme d'atteindre le handler,
    // sinon c'est le handler qui repond 422 via MAX_IMAGE_BASE64_LEN.
    assert!(
        status == StatusCode::UNPROCESSABLE_ENTITY || status == StatusCode::PAYLOAD_TOO_LARGE,
        "status inattendu: {status}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn analyze_image_rejects_invalid_content_type() {
    let mut state = test_helpers::build_test_state(Arc::new(test_helpers::StubVoiceChannels));
    state.ai.analyze_image_uc = Arc::new(OkAnalyzeImage);
    let app = router::build_for_test(state);
    let data = base64::engine::general_purpose::STANDARD.encode(b"x");
    let (status, json) = post(app, payload(&data, "application/pdf")).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(json["error"].as_str().unwrap().contains("Content-type"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn analyze_image_rejects_invalid_base64() {
    let mut state = test_helpers::build_test_state(Arc::new(test_helpers::StubVoiceChannels));
    state.ai.analyze_image_uc = Arc::new(OkAnalyzeImage);
    let app = router::build_for_test(state);
    let (status, json) = post(app, payload("not!!valid!!base64", "image/png")).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(json["error"].as_str().unwrap().contains("Base64"));
}

struct FlaggedAnalyzeImage;
#[async_trait]
impl AnalyzeImageUseCase for FlaggedAnalyzeImage {
    async fn analyze_image(&self, _: AnalyzeImageCommand) -> Result<ImageAnalysis, DomainError> {
        Ok(ImageAnalysis {
            action: Action::Delete,
            reason: "NSFW detecte".into(),
            score: 0.95,
            duration: None,
            classifications: vec![ImageClassification {
                label: "nsfw".into(),
                confidence: 0.95,
            }],
        })
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn analyze_image_with_action_broadcasts_infraction() {
    // Exerce la branche `action != "none"` du handler (broadcast infraction_new).
    let mut state = test_helpers::build_test_state(Arc::new(test_helpers::StubVoiceChannels));
    state.ai.analyze_image_uc = Arc::new(FlaggedAnalyzeImage);
    let app = router::build_for_test(state);
    let data = base64::engine::general_purpose::STANDARD.encode(b"png");
    let (status, json) = post(app, payload(&data, "image/jpeg")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["action"], "delete");
    assert_eq!(json["reason"], "NSFW detecte");
    assert_eq!(json["classifications"][0]["label"], "nsfw");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn analyze_image_accepts_webp_and_gif() {
    // Couvre plusieurs branches de is_allowed_image_content_type.
    let mut state = test_helpers::build_test_state(Arc::new(test_helpers::StubVoiceChannels));
    state.ai.analyze_image_uc = Arc::new(OkAnalyzeImage);
    let app = router::build_for_test(state);
    let data = base64::engine::general_purpose::STANDARD.encode(b"x");
    for ct in ["image/webp", "image/gif", "image/jpeg"] {
        let (status, _) = post(app.clone(), payload(&data, ct)).await;
        assert_eq!(
            status,
            StatusCode::OK,
            "content_type {ct} devrait etre accepte"
        );
    }
}
