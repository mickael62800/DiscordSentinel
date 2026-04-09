use std::sync::Arc;

use serde::Deserialize;
use sentinel_shared::api_client::BaseApiClient;

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct Game {
    pub id: String,
    pub guild_id: String,
    pub game_name: String,
    pub created_by: String,
}

#[derive(Debug, Deserialize)]
pub struct Subscriber {
    pub user_id: String,
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
        self.base.get_json(&format!("/api/games/{guild_id}")).await
    }

    /// Cree un jeu.
    pub async fn create_game(&self, guild_id: &str, game_name: &str, created_by: &str) -> Result<Game, String> {
        self.base.post_json("/api/games", &serde_json::json!({
            "guild_id": guild_id,
            "game_name": game_name,
            "created_by": created_by,
        })).await
    }

    /// Supprime un jeu.
    pub async fn delete_game(&self, guild_id: &str, game_id: &str) -> Result<(), String> {
        let req = self.base.client().delete(format!(
            "{}/api/games/{}/{}",
            self.base.base_url(), guild_id, game_id
        ));
        let resp = self.base.auth(req).send().await.map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            return Err(format!("API error: {}", resp.status()));
        }
        Ok(())
    }

    /// Inscrit un joueur a un jeu.
    pub async fn subscribe(&self, guild_id: &str, game_id: &str, user_id: &str) -> Result<(), String> {
        self.base.post_fire_and_forget(
            &format!("/api/games/{guild_id}/{game_id}/subscribe"),
            &serde_json::json!({ "user_id": user_id }),
        ).await;
        Ok(())
    }

    /// Desinscrit un joueur d'un jeu.
    pub async fn unsubscribe(&self, guild_id: &str, game_id: &str, user_id: &str) -> Result<(), String> {
        let req = self.base.client().delete(format!(
            "{}/api/games/{}/{}/subscribe/{}",
            self.base.base_url(), guild_id, game_id, user_id
        ));
        let _ = self.base.auth(req).send().await;
        Ok(())
    }

    /// Recupere les abonnes d'un jeu.
    pub async fn get_subscribers(&self, guild_id: &str, game_id: &str) -> Result<Vec<Subscriber>, String> {
        self.base.get_json(&format!("/api/games/{guild_id}/{game_id}/subscribers")).await
    }

    /// Trouve un jeu par nom (case-insensitive).
    pub async fn get_game_by_name(&self, guild_id: &str, game_name: &str) -> Result<Option<Game>, String> {
        self.base.get_json(&format!("/api/games/{guild_id}/by-name/{game_name}")).await
    }

    /// Jeux auxquels un joueur est inscrit.
    pub async fn get_user_games(&self, guild_id: &str, user_id: &str) -> Result<Vec<Game>, String> {
        self.base.get_json(&format!("/api/games/{guild_id}/user/{user_id}")).await
    }
}
