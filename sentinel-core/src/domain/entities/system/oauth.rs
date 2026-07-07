//! Entites du flux OAuth2 Discord web : session persistante (refresh token cote
//! serveur) et trace de login reussi (page Securite serveur).

use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Trace best-effort d'un login OAuth reussi (journal `successful_logins`).
#[derive(Debug, Clone)]
pub struct LoginTrace {
    pub discord_user_id: String,
    pub username: String,
    pub client_ip: String,
    pub user_agent: String,
}

/// Donnees de creation d'une session web persistante.
#[derive(Debug, Clone)]
pub struct NewOAuthSession {
    pub id: Uuid,
    pub discord_user_id: String,
    pub username: String,
    pub global_name: Option<String>,
    pub avatar: Option<String>,
    pub access_token: String,
    pub refresh_token: String,
    pub access_expires_at: DateTime<Utc>,
}

/// Session web persistante telle que relue depuis le stockage.
#[derive(Debug, Clone)]
pub struct OAuthSession {
    pub discord_user_id: String,
    pub username: String,
    pub global_name: Option<String>,
    pub avatar: Option<String>,
    pub access_token: String,
    pub refresh_token: String,
    pub access_expires_at: DateTime<Utc>,
}

/// Mise a jour des tokens d'une session apres un refresh Discord.
#[derive(Debug, Clone)]
pub struct SessionTokenUpdate {
    pub id: Uuid,
    pub access_token: String,
    pub refresh_token: String,
    pub access_expires_at: DateTime<Utc>,
}
