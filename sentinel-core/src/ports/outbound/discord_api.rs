use async_trait::async_trait;
use serde::Deserialize;

use crate::domain::errors::DomainError;

/// Trait pour les appels a l'API Discord. Permet de mocker le service
/// dans les tests d'integration HTTP sans taper la vraie API.
#[async_trait]
pub trait DiscordApi: Send + Sync {
    async fn list_text_channels(&self, guild_id: &str) -> Result<Vec<DiscordChannel>, DomainError>;
    /// Liste tous les salons utiles d'une guild (texte + voice + stage),
    /// chacun annote avec son `kind`. Utilise par les pickers config qui
    /// s'appliquent aux deux types (xp_channel_multipliers).
    async fn list_all_channels(&self, guild_id: &str) -> Result<Vec<DiscordChannel>, DomainError>;
    async fn upload_emoji(
        &self,
        guild_id: &str,
        name: &str,
        image_bytes: &[u8],
        mime: &str,
    ) -> Result<(String, String, bool), DomainError>;
    async fn ban_user(
        &self,
        guild_id: &str,
        user_id: &str,
        reason: &str,
    ) -> Result<(), DomainError>;
    async fn list_members(
        &self,
        guild_id: &str,
        limit: u32,
    ) -> Result<Vec<DiscordMember>, DomainError>;
    async fn send_dm(&self, user_id: &str, content: &str) -> Result<(), DomainError>;
    async fn create_role(
        &self,
        guild_id: &str,
        name: &str,
        color: u32,
        permissions: Option<&str>,
    ) -> Result<serde_json::Value, DomainError>;
    async fn edit_role(
        &self,
        guild_id: &str,
        role_id: &str,
        name: Option<&str>,
        color: Option<u32>,
        permissions: Option<&str>,
        mentionable: Option<bool>,
        hoist: Option<bool>,
    ) -> Result<serde_json::Value, DomainError>;
    async fn delete_role(&self, guild_id: &str, role_id: &str) -> Result<(), DomainError>;
    async fn unban_user(&self, guild_id: &str, user_id: &str) -> Result<(), DomainError>;
    async fn remove_timeout(&self, guild_id: &str, user_id: &str) -> Result<(), DomainError>;
    async fn apply_timeout(
        &self,
        guild_id: &str,
        user_id: &str,
        duration_seconds: u64,
    ) -> Result<(), DomainError>;
    async fn get_user_guilds(&self, access_token: &str) -> Result<Vec<UserGuild>, DomainError>;
    async fn get_user_me(&self, access_token: &str) -> Result<DiscordUser, DomainError>;
}

#[derive(Debug, Clone, serde::Serialize, Deserialize)]
pub struct DiscordMember {
    pub id: String,
    pub username: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
}

/// Phase 2 B — Subset des champs Discord renvoyes par GET /users/@me/guilds
/// dont on a besoin pour l'auth multi-tenant. On capture juste l'id pour
/// minimiser la deserialization (Discord renvoie name/icon/permissions etc.).
#[derive(Debug, Clone, Deserialize)]
pub struct UserGuild {
    pub id: String,
}

/// Phase 7 B — Info minimal d'un user Discord recupere via `/users/@me`.
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct DiscordUser {
    pub id: String,
    pub username: String,
    #[serde(default)]
    pub avatar: Option<String>,
}

/// Phase 9 Part E — Salon d'une guild (pour channel picker web).
/// `kind` : "text" | "announcement" | "voice" | "stage". Permet aux
/// pickers web d'afficher l'icone correcte (# pour le texte, 🔊 pour le
/// voice) et sert aussi de filtre.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DiscordChannel {
    pub id: String,
    pub name: String,
    pub position: i64,
    #[serde(default = "default_text_kind")]
    pub kind: String,
}

fn default_text_kind() -> String {
    "text".to_string()
}
