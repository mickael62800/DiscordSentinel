use std::sync::Arc;

use serde::Deserialize;
use sentinel_shared::api_client::BaseApiClient;

/// URL-encode un segment de path pour eviter qu'un nom de jeu avec `/` ou
/// caracteres speciaux ne casse le routing ou ne permette une injection.
fn encode_segment(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            // RFC 3986 unreserved
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{:02X}", b),
        })
        .collect()
}

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
        self.base
            .get_json(&format!("/api/games/{}", encode_segment(guild_id)))
            .await
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

    /// Inscrit un joueur a un jeu. Verifie explicitement le code HTTP de la
    /// reponse : auparavant on utilisait post_fire_and_forget qui ignorait
    /// silencieusement les erreurs API, donc le bot confirmait "inscrit" meme
    /// en cas de 500.
    pub async fn subscribe(&self, guild_id: &str, game_id: &str, user_id: &str) -> Result<(), String> {
        let url = format!(
            "{}/api/games/{}/{}/subscribe",
            self.base.base_url(),
            encode_segment(guild_id),
            encode_segment(game_id),
        );
        let req = self.base.client().post(url).json(&serde_json::json!({ "user_id": user_id }));
        let resp = self.base.auth(req).send().await.map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            return Err(format!("API error: {}", resp.status()));
        }
        Ok(())
    }

    /// Desinscrit un joueur d'un jeu. Meme remarque que subscribe : on
    /// verifie le status HTTP au lieu d'ignorer l'erreur.
    pub async fn unsubscribe(&self, guild_id: &str, game_id: &str, user_id: &str) -> Result<(), String> {
        let req = self.base.client().delete(format!(
            "{}/api/games/{}/{}/subscribe/{}",
            self.base.base_url(),
            encode_segment(guild_id),
            encode_segment(game_id),
            encode_segment(user_id),
        ));
        let resp = self.base.auth(req).send().await.map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            return Err(format!("API error: {}", resp.status()));
        }
        Ok(())
    }

    /// Recupere les abonnes d'un jeu.
    pub async fn get_subscribers(&self, guild_id: &str, game_id: &str) -> Result<Vec<Subscriber>, String> {
        self.base
            .get_json(&format!(
                "/api/games/{}/{}/subscribers",
                encode_segment(guild_id),
                encode_segment(game_id)
            ))
            .await
    }

    /// Trouve un jeu par nom (case-insensitive).
    pub async fn get_game_by_name(&self, guild_id: &str, game_name: &str) -> Result<Option<Game>, String> {
        self.base
            .get_json(&format!(
                "/api/games/{}/by-name/{}",
                encode_segment(guild_id),
                encode_segment(game_name)
            ))
            .await
    }

    /// Jeux auxquels un joueur est inscrit.
    pub async fn get_user_games(&self, guild_id: &str, user_id: &str) -> Result<Vec<Game>, String> {
        self.base
            .get_json(&format!(
                "/api/games/{}/user/{}",
                encode_segment(guild_id),
                encode_segment(user_id)
            ))
            .await
    }
}
