# Applications — Index

| App | Stack | Description |
|---|---|---|
| [desktop](./desktop.md) | Tauri 2 + Rust + Vue 3 + Pinia | Interface d'administration DiscordSentinel |

## Flux d'auth et multi-tenant (Phase 2 B)

1. L'utilisateur lance l'app → écran `Login` → OAuth2 Discord sur port local `19836`
2. Discord redirige avec `code` → `AuthService::exchange_code` → obtient `access_token`
3. `AuthService` propage le token à `ApiAdapter::set_discord_token`
4. Toutes les requêtes API suivantes envoient `X-Discord-Token: <token>` en header
5. Le backend (`api`) valide l'accès guild par guild via le middleware `guild_auth_middleware` (cache Redis 5 min des guilds autorisées)

Côté stockage local : la config (URL API, clé API, bot tokens) est chiffrée en AES-256-GCM dans un store LMDB local (voir `config_store.rs`).
