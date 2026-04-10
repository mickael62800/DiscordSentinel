# shared — Librairie commune des bots

**Rôle** : Librairie Rust partagée par tous les bots. Fournit le client HTTP vers l'API, la config, les cache settings Serenity, les embeds, le heartbeat, les parsers et le listener Redis.

Ce n'est **pas un bot** mais une dépendance `path = "../shared"` dans le `Cargo.toml` de chaque bot.

## Modules

### `api_client.rs`

- `BaseApiClient` — client HTTP `reqwest` singleton avec pool tuné (Phase 1 : `pool_max_idle_per_host=64`, `pool_idle_timeout=300s`, `tcp_keepalive=60s`). Méthodes : `heartbeat()`, `register_guild()`, `send_log()`, `get_guild_config()`, helpers `get_json/post_json/patch_fire_and_forget/delete_*`.
- `EventPublisher` — publie sur Redis (canal `sentinel:events` par défaut) pour les events temps-réel qui seront relayés par le `gateway` vers les desktops connectés.

### `cache_settings.rs` (Phase 1)

4 presets de cache Serenity :

| Preset | max_messages | cache_channels | cache_users | cache_guilds |
|---|---|---|---|---|
| `minimal()` | 0 | ❌ | ❌ | ✅ |
| `small()` | 0 | ✅ | ❌ | ✅ |
| `medium()` | 100 | ✅ | ✅ | ✅ |
| `full()` | défaut Serenity | ✅ | ✅ | ✅ |

### `config.rs`

- Trait `BotConfig` implémenté par chaque bot (accès au token + API config)
- `BaseConfig` avec les champs communs : `discord_token`, `api_base_url`, `api_key`
- Helpers `load_env(key, default)`, `load_env_bool(key, default)`

### `discord_helpers.rs`

Utilitaires Serenity : `reply_ephemeral_embed`, mentions, lookup channel/role, parsing de snowflakes, etc.

### `embeds.rs`

Constructeurs d'embeds typés : `info_embed`, `success_embed`, `critical_embed`, `moderate_embed`. Codes couleur et formatage unifiés.

### `heartbeat.rs`

Task background : envoie `POST /api/bots/heartbeat` toutes les 10s, appelle `register_guild()` au `ready` event, publie des events Redis quand nécessaire.

### `parsers.rs`

Parseurs pour mentions Discord (`<@USER_ID>`, `<#CHANNEL_ID>`, `<@&ROLE_ID>`) → `UserId`/`ChannelId`/`RoleId`.

### `redis_listener.rs`

Helper pour souscrire à `sentinel:events` et dispatcher les events vers des closures. Utilisé par `moderation-bot` et `ticket-bot` pour recevoir les rappels et autres events asynchrones publiés par les workers.

## Variables d'env (partagées)

- `API_BASE_URL`, `API_KEY`
- `REDIS_URL` (optionnel, active l'EventPublisher)
- `<NOM>_DISCORD_TOKEN` (spécifique à chaque bot)
