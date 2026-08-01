pub struct AppConfig {
    pub database_url: String,
    pub redis_url: String,
    pub api_key: String,
    pub host: String,
    pub port: u16,
    /// Phase 7A — port d'ecoute du serveur gRPC interne (tonic).
    /// Coexiste avec le port HTTP/Axum. Defaut : 50051.
    pub grpc_port: u16,
    pub rate_limit_per_sec: u64,
    pub max_body_size: usize,
    pub shutdown_timeout_secs: u64,
    /// Comma-separated list of allowed origins. Empty or "*" = permissive (dev mode).
    pub allowed_origins: String,
    /// Discord bot token pour executer des bans (optionnel).
    pub discord_bot_token: String,
    /// Phase 7 B — Liste de Discord user IDs "superadmin" autorises sur les
    /// endpoints globaux (non scoped par guild). Format : comma-separated.
    /// Ex: `SUPERADMIN_USER_IDS=123456789012345678,234567890123456789`
    pub superadmin_user_ids: Vec<String>,
    /// OAuth Discord — client_id de l'application Discord (visible cote serveur).
    pub discord_oauth_client_id: String,
    /// OAuth Discord — client_secret (jamais expose au front).
    pub discord_oauth_client_secret: String,
    /// OAuth Discord — URI de callback enregistree dans le portail Discord.
    /// Ex: `http://192.168.1.15:3000/auth/discord/callback`.
    pub discord_oauth_redirect_uri: String,
    /// URL de base du front web (ou rediriger apres callback OAuth).
    /// Ex: `http://192.168.1.15:5180`.
    pub web_front_url: String,
    /// Feature flag — active le `global_rbac_gate` (gate RBAC global
    /// fail-closed sur les mutations web) en mode ENFORCE (refuse reellement).
    /// Default `false` (no-op). Voir `middleware/global_rbac.rs`.
    pub rbac_global_gate: bool,
    /// Token optionnel protégeant `/metrics`. Vide (défaut) = endpoint ouvert
    /// (comportement historique : Prometheus scrape sans auth sur le réseau
    /// interne). Si défini, `/metrics` exige `Authorization: Bearer <token>`
    /// (comparaison constant-time) et Prometheus doit être configuré avec.
    pub metrics_token: String,
    /// Mode AUDIT du gate RBAC global : exécute toute la logique de décision et
    /// journalise ce qui SERAIT refusé, mais laisse passer les requêtes. Permet
    /// de valider la table de routes en prod (repérer les 403 potentiels sur des
    /// routes légitimes non mappées) AVANT de basculer en enforce. Activé quand
    /// `RBAC_GLOBAL_GATE=audit`. Mutuellement exclusif avec l'enforce.
    pub rbac_global_gate_audit: bool,
    /// Serveur Discord unique servi par cette installation.
    ///
    /// L'application est mono-serveur : toute requete portant un autre
    /// `guild_id` est refusee par `single_guild_middleware`. Le modele de
    /// donnees garde sa colonne `guild_id` — la retirer serait un refactor
    /// enorme pour aucun gain, la colonne valant simplement toujours la meme
    /// chose — mais la surface HTTP, elle, n'accepte qu'une valeur.
    ///
    /// Vide = verrou desactive (toutes les guildes passent). Utile en
    /// developpement et pour ne pas bloquer une installation existante qui
    /// n'aurait pas encore renseigne la variable.
    pub guild_id: String,
}

impl AppConfig {
    pub fn from_env() -> Self {
        Self {
            database_url: std::env::var("DATABASE_URL").expect("DATABASE_URL manquant"),
            redis_url: std::env::var("REDIS_URL").expect("REDIS_URL manquant"),
            api_key: {
                let key = std::env::var("API_KEY").unwrap_or_default();
                let require = std::env::var("REQUIRE_API_KEY")
                    .map(|v| v != "false" && v != "0")
                    .unwrap_or(true);
                if key.is_empty() && require {
                    tracing::error!("API_KEY non configuree. Definir API_KEY ou REQUIRE_API_KEY=false pour le dev.");
                    std::process::exit(1);
                }
                if !key.is_empty() && key.len() < 16 {
                    // Securite : une API_KEY courte est bruteforçable. On refuse
                    // de demarrer avec une cle < 16 chars quand elle est requise,
                    // plutot que d'emettre un simple warning ignore en pratique.
                    tracing::error!(
                        "API_KEY trop courte ({} chars). Minimum 16 chars (32+ recommande en prod).",
                        key.len()
                    );
                    std::process::exit(1);
                }
                key
            },
            host: std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".into()),
            port: std::env::var("PORT")
                .unwrap_or_else(|_| "3000".into())
                .parse()
                .expect("PORT invalide"),
            grpc_port: std::env::var("GRPC_PORT")
                .unwrap_or_else(|_| "50051".into())
                .parse()
                .unwrap_or(50051),
            rate_limit_per_sec: std::env::var("RATE_LIMIT_PER_SEC")
                .unwrap_or_else(|_| "200".into())
                .parse()
                .unwrap_or(50),
            max_body_size: std::env::var("MAX_BODY_SIZE")
                .unwrap_or_else(|_| "1048576".into())
                .parse()
                .unwrap_or(1_048_576),
            shutdown_timeout_secs: std::env::var("SHUTDOWN_TIMEOUT")
                .unwrap_or_else(|_| "30".into())
                .parse()
                .unwrap_or(30),
            allowed_origins: std::env::var("ALLOWED_ORIGINS").unwrap_or_default(),
            metrics_token: std::env::var("METRICS_TOKEN").unwrap_or_default(),
            // Meme variable que celle lue par le conteneur web : une seule
            // source de verite pour « de quel serveur parle cette
            // installation ». `GUILD_ID` reste accepte en repli.
            guild_id: std::env::var("PUBLIC_GUILD_ID")
                .or_else(|_| std::env::var("GUILD_ID"))
                .unwrap_or_default()
                .trim()
                .to_string(),
            // Token Discord : priorite SENTINEL_DISCORD_TOKEN (bot unifie),
            // fallback sur DISCORD_TOKEN. Les anciens noms par bot
            // (AUTOMOD_DISCORD_TOKEN, MODERATION_DISCORD_TOKEN) sont abandonnes.
            discord_bot_token: std::env::var("SENTINEL_DISCORD_TOKEN")
                .or_else(|_| std::env::var("DISCORD_TOKEN"))
                .unwrap_or_default(),
            superadmin_user_ids: std::env::var("SUPERADMIN_USER_IDS")
                .unwrap_or_default()
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect(),
            discord_oauth_client_id: std::env::var("DISCORD_CLIENT_ID").unwrap_or_default(),
            discord_oauth_client_secret: std::env::var("DISCORD_CLIENT_SECRET").unwrap_or_default(),
            discord_oauth_redirect_uri: std::env::var("DISCORD_REDIRECT_URI").unwrap_or_default(),
            web_front_url: std::env::var("WEB_FRONT_URL").unwrap_or_default(),
            // Tri-etat via RBAC_GLOBAL_GATE :
            //   - "true"/"1"     -> enforce (refuse reellement)
            //   - "audit"/"dryrun" -> audit (log-only, laisse passer)
            //   - autre / absent -> off (no-op total)
            // Default OFF : zero changement de comportement tant que non configure.
            rbac_global_gate: std::env::var("RBAC_GLOBAL_GATE")
                .map(|v| v == "true" || v == "1")
                .unwrap_or(false),
            rbac_global_gate_audit: std::env::var("RBAC_GLOBAL_GATE")
                .map(|v| v == "audit" || v == "dryrun")
                .unwrap_or(false),
        }
    }

    pub fn bind_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    pub fn grpc_bind_addr(&self) -> String {
        format!("{}:{}", self.host, self.grpc_port)
    }
}

#[cfg(test)]
#[path = "tests/config.rs"]
mod tests;
