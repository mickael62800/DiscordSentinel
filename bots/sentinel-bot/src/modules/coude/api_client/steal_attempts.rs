//! Phase 5 — Methodes ApiClient pour persistance des tentatives /voler.
//!
//! Le bot appelait `tokio::spawn(sleep 60s)` pour gerer le timeout, ce
//! qui mourrait au redemarrage du process. Maintenant le worker
//! `expire_steals` (sentinel-worker) scanne les rows en `pending` et
//! publie un event Redis `coude:steal_expired` que le bot consomme
//! pour effectuer la resolution AFK.

use serde::Deserialize;
use uuid::Uuid;

use super::ApiClient;

#[derive(Deserialize)]
pub struct StealAttemptCreated {
    #[allow(dead_code)]
    pub id: Uuid,
    #[allow(dead_code)]
    pub expires_at: String,
}

impl ApiClient {
    /// Cree une row `coude_steal_attempts` (status=pending). Le bot
    /// genere l'UUID client-side pour pouvoir l'inclure dans le custom_id
    /// du bouton "Se defendre" sans round-trip supplementaire.
    pub async fn create_steal_attempt(
        &self,
        attempt_id: Uuid,
        guild_id: &str,
        thief_id: &str,
        target_id: &str,
        message_id: &str,
        channel_id: &str,
        window_secs: i64,
    ) -> Result<StealAttemptCreated, String> {
        let body = serde_json::json!({
            "id": attempt_id,
            "guild_id": guild_id,
            "thief_id": thief_id,
            "target_id": target_id,
            "message_id": message_id,
            "channel_id": channel_id,
            "window_secs": window_secs,
        });
        self.base.post_json("/api/coude/steals", &body).await
    }

    /// Marque le row defended apres clic du bouton (idempotent).
    /// Fire-and-forget : meme si l'API rate, on continue la resolution.
    pub async fn mark_steal_defended(&self, attempt_id: Uuid) {
        let body = serde_json::json!({});
        self.base
            .patch_fire_and_forget(
                &format!("/api/coude/steals/{attempt_id}/defend"),
                &body,
            )
            .await;
    }

    /// Marque le row resolved apres post-resolution (idempotent).
    /// Fire-and-forget.
    pub async fn mark_steal_resolved(&self, attempt_id: Uuid) {
        let body = serde_json::json!({});
        self.base
            .patch_fire_and_forget(
                &format!("/api/coude/steals/{attempt_id}/resolved"),
                &body,
            )
            .await;
    }
}
