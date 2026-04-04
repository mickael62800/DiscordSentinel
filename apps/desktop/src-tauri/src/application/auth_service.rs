use std::sync::Mutex;

use reqwest::Client;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;

use crate::domain::entities::{ApiConfig, AuthSession, AuthToken, DiscordConfig, DiscordUser};
use crate::infrastructure::config_store::ConfigStore;

const DISCORD_AUTH_URL: &str = "https://discord.com/api/oauth2/authorize";
const DISCORD_TOKEN_URL: &str = "https://discord.com/api/oauth2/token";
const DISCORD_USER_URL: &str = "https://discord.com/api/users/@me";
const REDIRECT_PORT: u16 = 19836;

pub struct AuthService {
    client: Client,
    config_store: ConfigStore,
    session: Mutex<Option<AuthSession>>,
}

impl AuthService {
    pub fn new(config_store: ConfigStore) -> Self {
        Self {
            client: Client::new(),
            config_store,
            session: Mutex::new(None),
        }
    }

    pub fn get_current_user(&self) -> Option<DiscordUser> {
        self.session
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .as_ref()
            .map(|s| s.user.clone())
    }

    pub fn logout(&self) {
        *self.session.lock().unwrap_or_else(|p| p.into_inner()) = None;
    }

    pub fn get_discord_config(&self) -> Result<Option<DiscordConfig>, String> {
        self.config_store.get_discord_config()
    }

    pub fn save_discord_config(&self, config: DiscordConfig) -> Result<(), String> {
        self.config_store.save_discord_config(&config)
    }

    pub fn clear_discord_config(&self) -> Result<(), String> {
        self.config_store.clear_discord_config()
    }

    pub fn get_api_config(&self) -> Result<Option<ApiConfig>, String> {
        self.config_store.get_api_config()
    }

    pub fn save_api_config(&self, config: ApiConfig) -> Result<(), String> {
        self.config_store.save_api_config(&config)
    }

    pub fn save_bot_token(&self, bot_name: &str, token: &str) -> Result<(), String> {
        self.config_store.save_bot_token(bot_name, token)
    }

    pub fn get_bot_token(&self, bot_name: &str) -> Result<Option<String>, String> {
        self.config_store.get_bot_token(bot_name)
    }

    pub fn get_all_bot_tokens(&self) -> Result<Vec<(String, bool)>, String> {
        self.config_store.get_all_bot_tokens()
    }

    pub fn delete_bot_token(&self, bot_name: &str) -> Result<(), String> {
        self.config_store.delete_bot_token(bot_name)
    }

    pub async fn start_oauth_flow(&self) -> Result<DiscordUser, String> {
        let config = self
            .config_store
            .get_discord_config()?
            .ok_or("Discord credentials not configured. Please set up Client ID and Secret first.")?;

        let redirect_uri = format!("http://localhost:{}/callback", REDIRECT_PORT);

        let auth_url = format!(
            "{}?client_id={}&redirect_uri={}&response_type=code&scope=identify",
            DISCORD_AUTH_URL,
            config.client_id,
            encode_uri_component(&redirect_uri),
        );

        // Async TCP listener — does NOT block the Tokio runtime
        let listener = TcpListener::bind(format!("127.0.0.1:{}", REDIRECT_PORT))
            .await
            .map_err(|e| format!("Failed to start local server: {}", e))?;

        // Open browser
        open::that(&auth_url).map_err(|e| format!("Failed to open browser: {}", e))?;

        // Async accept
        let (mut stream, _) = listener
            .accept()
            .await
            .map_err(|e| format!("Failed to accept connection: {}", e))?;

        let (reader, mut writer) = stream.split();
        let mut buf_reader = BufReader::new(reader);
        let mut request_line = String::new();
        buf_reader
            .read_line(&mut request_line)
            .await
            .map_err(|e| format!("Failed to read request: {}", e))?;

        // Parse code from: GET /callback?code=XXXXX HTTP/1.1
        let code = extract_code(&request_line)?;

        // Send success response to browser
        let response_body = r#"<!DOCTYPE html><html><body style="background:#1a1b2e;color:#e8e8f0;font-family:sans-serif;display:flex;align-items:center;justify-content:center;height:100vh;margin:0"><div style="text-align:center"><h1>Connected!</h1><p>You can close this tab and return to DiscordSentinel.</p></div></body></html>"#;
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            response_body.len(),
            response_body
        );
        let _ = writer.write_all(response.as_bytes()).await;
        drop(writer);
        drop(buf_reader);
        drop(listener);

        // Exchange code for token
        let token = self.exchange_code(&code, &redirect_uri, &config).await?;

        // Fetch user info
        let user = self.fetch_user(&token.access_token).await?;

        // Store session
        *self.session.lock().unwrap_or_else(|p| p.into_inner()) = Some(AuthSession {
            user: user.clone(),
            token,
        });

        Ok(user)
    }

    async fn exchange_code(
        &self,
        code: &str,
        redirect_uri: &str,
        config: &DiscordConfig,
    ) -> Result<AuthToken, String> {
        let params = [
            ("client_id", config.client_id.as_str()),
            ("client_secret", config.client_secret.as_str()),
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", redirect_uri),
        ];

        self.client
            .post(DISCORD_TOKEN_URL)
            .form(&params)
            .send()
            .await
            .map_err(|e| format!("Token exchange failed: {}", e))?
            .json::<AuthToken>()
            .await
            .map_err(|e| format!("Failed to parse token: {}", e))
    }

    async fn fetch_user(&self, access_token: &str) -> Result<DiscordUser, String> {
        self.client
            .get(DISCORD_USER_URL)
            .header("Authorization", format!("Bearer {}", access_token))
            .send()
            .await
            .map_err(|e| format!("Failed to fetch user: {}", e))?
            .json::<DiscordUser>()
            .await
            .map_err(|e| format!("Failed to parse user: {}", e))
    }
}

fn extract_code(request_line: &str) -> Result<String, String> {
    let parts: Vec<&str> = request_line.split_whitespace().collect();
    if parts.len() < 2 {
        return Err("Invalid request".into());
    }

    let path = parts[1];
    if let Some(query_start) = path.find('?') {
        let query = &path[query_start + 1..];
        for param in query.split('&') {
            if let Some(value) = param.strip_prefix("code=") {
                return Ok(value.to_string());
            }
        }
    }

    Err("No authorization code in callback".into())
}

/// Proper percent-encoding for URI components (RFC 3986)
fn encode_uri_component(s: &str) -> String {
    let mut encoded = String::with_capacity(s.len() * 3);
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(byte as char);
            }
            _ => {
                encoded.push_str(&format!("%{:02X}", byte));
            }
        }
    }
    encoded
}
