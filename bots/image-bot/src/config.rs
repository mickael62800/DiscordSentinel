/// Configuration du bot chargée depuis les variables d'environnement.
pub struct Config {
    pub discord_token: String,
    pub api_base_url: String,
    pub api_key: String,
    /// Taille max d'image acceptée en bytes (défaut: 10 MB)
    pub max_image_size: u64,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            discord_token: std::env::var("DISCORD_TOKEN")
                .expect("DISCORD_TOKEN manquant dans .env"),
            api_base_url: std::env::var("API_BASE_URL")
                .unwrap_or_else(|_| "http://localhost:3000".to_string()),
            api_key: std::env::var("API_KEY").unwrap_or_default(),
            max_image_size: std::env::var("MAX_IMAGE_SIZE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(10 * 1024 * 1024),
        }
    }
}
