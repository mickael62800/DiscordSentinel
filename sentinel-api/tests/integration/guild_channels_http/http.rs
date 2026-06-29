//! Tests d'integration HTTP pour GET /api/guilds/{guild_id}/channels.

#[path = "../../test_helpers.rs"]
mod test_helpers;

use std::sync::Arc;

use axum::body::Body;
use axum::http::Request;
use axum::http::StatusCode;
use http_body_util::BodyExt;
use tower::ServiceExt;

use sentinel_api::adapters::inbound::http::router;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_text_channels_returns_empty_array_from_mock() {
    let mut state = test_helpers::build_test_state(Arc::new(test_helpers::StubVoiceChannels));
    state.discord_api = Arc::new(test_helpers::MockDiscordApi::new());
    let app = router::build_for_test(state);
    let req = Request::builder()
        .method("GET")
        .uri("/api/guilds/111111111111111111/channels")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let b = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&b).unwrap();
    assert!(json.as_array().unwrap().is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_text_channels_second_call_hits_cache() {
    // Deux appels consecutifs : le second doit lire le cache Redis si pose.
    // On verifie surtout que le code path cache ne panique pas.
    let mut state = test_helpers::build_test_state(Arc::new(test_helpers::StubVoiceChannels));
    state.discord_api = Arc::new(test_helpers::MockDiscordApi::new());
    let app = router::build_for_test(state);

    for _ in 0..2 {
        let req = Request::builder()
            .method("GET")
            .uri("/api/guilds/222222222222222222/channels")
            .body(Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }
}

// DiscordApi local qui retourne une liste configurable pour tester le path
// "serialize + set cache" quand la reponse n'est pas vide.
use async_trait::async_trait;
use sentinel_api::adapters::outbound::discord_api::DiscordApi;
use sentinel_api::adapters::outbound::discord_api::DiscordChannel;
use sentinel_api::adapters::outbound::discord_api::DiscordMember;
use sentinel_api::adapters::outbound::discord_api::DiscordUser;
use sentinel_api::adapters::outbound::discord_api::UserGuild;
use sentinel_core::domain::errors::DomainError;

struct DiscordApiWithChannels(Vec<DiscordChannel>);

#[async_trait]
impl DiscordApi for DiscordApiWithChannels {
    async fn list_text_channels(&self, _: &str) -> Result<Vec<DiscordChannel>, DomainError> {
        Ok(self.0.clone())
    }
    async fn upload_emoji(
        &self,
        _: &str,
        _: &str,
        _: &[u8],
        _: &str,
    ) -> Result<(String, String, bool), DomainError> {
        unimplemented!()
    }
    async fn ban_user(&self, _: &str, _: &str, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn list_members(&self, _: &str, _: u32) -> Result<Vec<DiscordMember>, DomainError> {
        Ok(vec![])
    }
    async fn send_dm(&self, _: &str, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn create_role(
        &self,
        _: &str,
        _: &str,
        _: u32,
        _: Option<&str>,
    ) -> Result<serde_json::Value, DomainError> {
        unimplemented!()
    }
    async fn edit_role(
        &self,
        _: &str,
        _: &str,
        _: Option<&str>,
        _: Option<u32>,
        _: Option<&str>,
        _: Option<bool>,
        _: Option<bool>,
    ) -> Result<serde_json::Value, DomainError> {
        unimplemented!()
    }
    async fn delete_role(&self, _: &str, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn unban_user(&self, _: &str, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn remove_timeout(&self, _: &str, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn apply_timeout(&self, _: &str, _: &str, _: u64) -> Result<(), DomainError> {
        Ok(())
    }
    async fn get_user_guilds(&self, _: &str) -> Result<Vec<UserGuild>, DomainError> {
        Ok(vec![])
    }
    async fn get_user_me(&self, _: &str) -> Result<DiscordUser, DomainError> {
        unimplemented!()
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_text_channels_returns_non_empty_response() {
    let channels = vec![
        DiscordChannel {
            id: "c1".into(),
            name: "general".into(),
            position: 0,
        },
        DiscordChannel {
            id: "c2".into(),
            name: "random".into(),
            position: 0,
        },
    ];
    let mut state = test_helpers::build_test_state(Arc::new(test_helpers::StubVoiceChannels));
    state.discord_api = Arc::new(DiscordApiWithChannels(channels));
    let app = router::build_for_test(state);

    let req = Request::builder()
        .method("GET")
        .uri("/api/guilds/333333333333333333/channels")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let b = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&b).unwrap();
    assert_eq!(json.as_array().unwrap().len(), 2);
    assert_eq!(json[0]["id"], "c1");
    assert_eq!(json[0]["name"], "general");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_text_channels_caches_and_second_call_returns_same() {
    let channels = vec![DiscordChannel {
        id: "cached1".into(),
        name: "salon".into(),
        position: 0,
    }];
    let mut state = test_helpers::build_test_state(Arc::new(test_helpers::StubVoiceChannels));
    state.discord_api = Arc::new(DiscordApiWithChannels(channels));
    let app = router::build_for_test(state);

    let guild_id = "444444444444444444";
    for _ in 0..2 {
        let req = Request::builder()
            .method("GET")
            .uri(format!("/api/guilds/{guild_id}/channels"))
            .body(Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let b = resp.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&b).unwrap();
        assert_eq!(json.as_array().unwrap().len(), 1);
        assert_eq!(json[0]["id"], "cached1");
    }
}
