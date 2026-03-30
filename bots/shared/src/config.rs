/// Configuration de base commune a tous les bots.
/// Les bots etendent cette struct avec leurs champs specifiques.
#[derive(Clone)]
pub struct BaseConfig {
    pub discord_token: String,
    pub api_base_url: String,
    pub api_key: String,
}

impl BaseConfig {
    /// Charge la config de base depuis les variables d'environnement.
    /// `token_var` est le nom de la variable pour le token Discord
    /// (ex: "DISCORD_TOKEN", "AUDIT_DISCORD_TOKEN").
    pub fn from_env(token_var: &str) -> Self {
        Self {
            discord_token: std::env::var(token_var)
                .unwrap_or_else(|_| panic!("{token_var} manquant dans .env")),
            api_base_url: std::env::var("API_BASE_URL")
                .unwrap_or_else(|_| "http://localhost:3000".to_string()),
            api_key: std::env::var("API_KEY").unwrap_or_default(),
        }
    }
}

/// Trait que chaque Config de bot doit implementer pour acceder aux champs de base.
pub trait BotConfig {
    fn base(&self) -> &BaseConfig;

    fn discord_token(&self) -> &str {
        &self.base().discord_token
    }

    fn api_base_url(&self) -> &str {
        &self.base().api_base_url
    }

    fn api_key(&self) -> &str {
        &self.base().api_key
    }
}
