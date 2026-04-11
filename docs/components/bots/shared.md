# shared — Librairie commune des bots

**Rôle** : Librairie Rust partagée par tous les bots. Fournit le client HTTP et **le client gRPC (Phase 7A)** vers l'API, le circuit breaker, la config, les cache settings Serenity, les embeds, le heartbeat, les parsers et le listener Redis.

Ce n'est **pas un bot** mais une dépendance `path = "../shared"` dans le `Cargo.toml` de chaque bot.

## Modules

### `api_client.rs` — client HTTP legacy

- `BaseApiClient` — client HTTP `reqwest` singleton avec pool tuné (Phase 1 : `pool_max_idle_per_host=64`, `pool_idle_timeout=300s`, `tcp_keepalive=60s`). Méthodes : `heartbeat()`, `register_guild()`, `send_log()`, `get_guild_config()`, helpers `get_json/post_json/patch_fire_and_forget/delete_*`.
- Toujours utilisé par les bots non/partiellement migrés et pour les endpoints qui n'ont pas (encore) d'équivalent gRPC.

### `grpc_client.rs` — client gRPC (Phase 7A) 🆕

- `SentinelGrpcClient` — client tonic singleton, **un seul `Channel` HTTP/2** partagé par tous les services (multiplexage natif gRPC). `connect_lazy` au démarrage, keep-alive 30s, timeout 30s.
- **Auth** : interceptor injecte `authorization: Bearer <API_KEY>` à chaque RPC.
- **Helpers de service** : `progression()`, `stats()`, `tickets()`, `moderation()`, `blackjack()`, `coude_players()`, `role_panels()`, `members()`, `security()`, `automod()`, `voice_channels()`, `images()` — chacun renvoie un client tonic prêt à l'emploi avec l'interceptor déjà appliqué.
- **`guarded()`** : wrapper qui passe l'appel par le `CircuitBreaker`. À utiliser par défaut pour bénéficier de la dégradation gracieuse.
- **`GrpcClientKey`** : `TypeMapKey` pour stocker l'`Arc<SentinelGrpcClient>` dans le `data` Serenity.
- **`from_env()`** : lit `GRPC_API_URL` (défaut `http://127.0.0.1:50051`) et `API_KEY`.

### `circuit_breaker.rs` — Phase 7A 🆕

Circuit breaker minimaliste à 3 états (Closed/Open/HalfOpen), implémenté sans `Mutex` (atomics lock-free). Default : 5 échecs consécutifs → ouvert 10s → half-open → 1 essai → referme ou ré-ouvre.

Compté comme « échec » : `tonic::Code::Unavailable`, `DeadlineExceeded`, `Internal`. Les autres erreurs (NotFound, InvalidArgument, etc.) **ne déclenchent pas** le breaker — ce sont des erreurs métier normales.

Tests unitaires inclus : `closed_then_opens_after_threshold`, `half_open_after_cooldown`.

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

### `event_bus.rs` / `redis_listener.rs`

Helper pour publier sur Redis Streams (Phase 5B) et souscrire à `sentinel:events`. Utilisé par audit-bot, moderation-bot et ticket-bot pour la communication asynchrone fire-and-forget et le replay differé.

### `heartbeat.rs`

Task background : envoie `POST /api/bots/heartbeat` toutes les 10s, appelle `register_guild()` au `ready` event, publie des events Redis quand nécessaire.

### `parsers.rs`

Parseurs pour mentions Discord (`<@USER_ID>`, `<#CHANNEL_ID>`, `<@&ROLE_ID>`) → `UserId`/`ChannelId`/`RoleId`.

### `shard_launcher.rs`

Helper pour démarrer un bot en mode mono-shard ou multi-shard selon les variables d'env `SHARD_MODE`/`SHARD_ID`/`SHARD_TOTAL`.

## Dépendance proto

Depuis la Phase 7A, `bots/shared` dépend de `services/proto` (path `../../services/proto`) qui contient les définitions `.proto` compilées par `tonic-build` au build. Les types générés sont accessibles via `sentinel_proto::<service>::v1::*`.

## Variables d'env (partagées)

- `API_BASE_URL`, `API_KEY`
- `GRPC_API_URL` (Phase 7A, défaut `http://127.0.0.1:50051`)
- `REDIS_URL` (optionnel, active l'EventPublisher)
- `<NOM>_DISCORD_TOKEN` (spécifique à chaque bot)
