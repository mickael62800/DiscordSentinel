use std::sync::Arc;

use serde::{Deserialize, Serialize};
use crate::shared::api_client::BaseApiClient;

/// URL-encode un segment de path pour eviter qu'un nom de jeu avec `/` ou
/// caracteres speciaux ne casse le routing ou ne permette une injection.
fn encode_segment(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{:02X}", b),
        })
        .collect()
}

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct Game {
    pub id: String,
    pub guild_id: String,
    pub game_name: String,
    pub created_by: String,
    #[serde(default)]
    pub emoji: Option<String>,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub role_id: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct GamePanel {
    pub id: String,
    pub guild_id: String,
    pub channel_id: String,
    pub message_id: String,
    #[serde(default)]
    pub category: Option<String>,
}

#[derive(Debug, Serialize)]
struct SavePanelReq<'a> {
    channel_id: &'a str,
    message_id: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    category: Option<&'a str>,
}

pub struct GameApiClient {
    pub base: Arc<BaseApiClient>,
}

impl GameApiClient {
    pub fn new(base: Arc<BaseApiClient>) -> Self {
        Self { base }
    }

    /// Liste tous les jeux d'un serveur.
    pub async fn list_games(&self, guild_id: &str) -> Result<Vec<Game>, String> {
        self.base
            .get_json(&format!("/api/games/{}", encode_segment(guild_id)))
            .await
    }

    /// Liste les jeux d'une categorie (None => jeux sans categorie).
    pub async fn list_games_by_category(&self, guild_id: &str, category: Option<&str>) -> Result<Vec<Game>, String> {
        let mut url = format!("/api/games/{}/by-category", encode_segment(guild_id));
        if let Some(cat) = category {
            url.push_str(&format!("?category={}", encode_segment(cat)));
        }
        self.base.get_json(&url).await
    }

    /// Cree un jeu. Le bot cree d'abord le role Discord puis passe son ID ici.
    pub async fn create_game(
        &self,
        guild_id: &str,
        game_name: &str,
        created_by: &str,
        role_id: Option<&str>,
        emoji: Option<&str>,
        category: Option<&str>,
    ) -> Result<Game, String> {
        self.base.post_json("/api/games", &serde_json::json!({
            "guild_id": guild_id,
            "game_name": game_name,
            "created_by": created_by,
            "emoji": emoji,
            "category": category,
            "role_id": role_id,
        })).await
    }

    /// Supprime un jeu.
    pub async fn delete_game(&self, guild_id: &str, game_id: &str) -> Result<(), String> {
        let req = self.base.client().delete(format!(
            "{}/api/games/{}/{}",
            self.base.base_url(),
            encode_segment(guild_id),
            encode_segment(game_id),
        ));
        let resp = self.base.auth(req).send().await.map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            return Err(format!("API error: {}", resp.status()));
        }
        Ok(())
    }

    /// Associe un role Discord a un jeu (PATCH role_id).
    #[allow(dead_code)]
    pub async fn set_role_id(
        &self,
        guild_id: &str,
        game_id: &str,
        role_id: Option<&str>,
    ) -> Result<Game, String> {
        let url = format!(
            "{}/api/games/{}/{}/role",
            self.base.base_url(),
            encode_segment(guild_id),
            encode_segment(game_id),
        );
        let req = self.base.client().patch(url).json(&serde_json::json!({
            "role_id": role_id,
        }));
        let resp = self.base.auth(req).send().await.map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            return Err(format!("API error: {}", resp.status()));
        }
        resp.json::<Game>().await.map_err(|e| e.to_string())
    }

    pub async fn get_game_by_name(&self, guild_id: &str, game_name: &str) -> Result<Option<Game>, String> {
        self.base
            .get_json(&format!(
                "/api/games/{}/by-name/{}",
                encode_segment(guild_id),
                encode_segment(game_name)
            ))
            .await
    }

    // ── Panels ──

    pub async fn save_panel(
        &self,
        guild_id: &str,
        channel_id: &str,
        message_id: &str,
        category: Option<&str>,
    ) -> Result<GamePanel, String> {
        let url = format!(
            "{}/api/games/{}/panels",
            self.base.base_url(),
            encode_segment(guild_id),
        );
        let body = SavePanelReq { channel_id, message_id, category };
        let req = self.base.client().post(url).json(&body);
        let resp = self.base.auth(req).send().await.map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            return Err(format!("API error: {}", resp.status()));
        }
        resp.json::<GamePanel>().await.map_err(|e| e.to_string())
    }

    #[allow(dead_code)]
    pub async fn find_panel_by_message(
        &self,
        guild_id: &str,
        message_id: &str,
    ) -> Result<Option<GamePanel>, String> {
        self.base
            .get_json(&format!(
                "/api/games/{}/panels/by-message/{}",
                encode_segment(guild_id),
                encode_segment(message_id)
            ))
            .await
    }

    pub async fn list_panels(&self, guild_id: &str) -> Result<Vec<GamePanel>, String> {
        self.base
            .get_json(&format!(
                "/api/games/{}/panels",
                encode_segment(guild_id)
            ))
            .await
    }
}
