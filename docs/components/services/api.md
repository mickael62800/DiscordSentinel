# api — Backend central

**Rôle** : Le cerveau de DiscordSentinel. Persiste toutes les données en PostgreSQL, expose ~34 familles de routes HTTP, fait l'inférence IA (ONNX texte/image), sert les bots et le desktop, applique l'auth + rate limiting + multi-tenant.

## Architecture

**Hexagonale (Ports & Adapters)**. Stack : **Rust / Axum 0.8 / Tokio / sqlx / Redis / ONNX Runtime / PostgreSQL 16**. ~105 migrations SQL versionnées.

## Structure du code

```
services/api/
├── Cargo.toml                      (jemalloc + LTO + strip en release)
├── migrations/                     (001 → 105 — schémas versionnés)
└── src/
    ├── main.rs                     (startup, pool sqlx, Redis, ONNX load, router)
    ├── lib.rs                      (re-exports pour les tests)
    │
    ├── adapters/
    │   ├── inbound/http/
    │   │   ├── router.rs           (routes + layers dans l'ordre)
    │   │   ├── state.rs            (AppState : use cases, repos, inference, cache)
    │   │   ├── errors.rs           (ApiError + From<DomainError>)
    │   │   ├── helpers.rs          (map_to_dtos, ok_response, etc.)
    │   │   ├── validation.rs       (helpers de validation input)
    │   │   ├── metrics.rs          (init Prometheus + middleware)
    │   │   │
    │   │   ├── handlers/           (34 familles de routes)
    │   │   │   ├── analyze.rs           # POST /analyze (texte)
    │   │   │   ├── analyze_image.rs     # POST /analyze/image
    │   │   │   ├── ai_jobs.rs           # Phase 4 A — queue async
    │   │   │   ├── rules.rs             # /api/rules
    │   │   │   ├── infractions.rs       # /api/infractions
    │   │   │   ├── tickets.rs           # /api/tickets
    │   │   │   ├── moderation.rs        # /api/moderation
    │   │   │   ├── security.rs          # /api/security
    │   │   │   ├── strikes.rs           # /api/strikes
    │   │   │   ├── notes.rs, reminders.rs, voice_channels.rs, ...
    │   │   │   ├── coude/               # sous-dossier 8 fichiers (Phase 3)
    │   │   │   ├── blackjack/           # sous-dossier 4 fichiers (Phase 3)
    │   │   │   └── ... (34 au total)
    │   │   │
    │   │   ├── middleware/
    │   │   │   ├── auth.rs              # Bearer API key
    │   │   │   ├── guild_auth.rs        # Phase 2 B — multi-tenant
    │   │   │   ├── rate_limit.rs        # Token bucket par IP
    │   │   │   └── api_logger.rs        # Log request/response → table logs
    │   │   │
    │   │   └── dto/                 (DTO HTTP — request/response Serde)
    │   │
    │   └── outbound/
    │       ├── postgres/            (~14 repos : PgRuleRepository, ...)
    │       ├── redis_cache.rs       (wrapper Redis async multiplexé)
    │       └── job_client.rs        (client vers worker queue)
    │
    ├── application/                 (Use cases : Manage*Service)
    │   ├── analyze_message_service.rs
    │   ├── analyze_image_service.rs
    │   ├── manage_rules_service.rs
    │   ├── manage_moderation_service.rs
    │   ├── manage_coude_{players,combats,bets,economy,inventory,social}_service.rs
    │   └── ... (20+ services)
    │
    ├── domain/
    │   ├── entities/                (Rule, Infraction, Ticket, CoudePlayer, etc.)
    │   ├── services/                (InferenceService, DiscordApiService, TextTokenizer)
    │   └── value_objects/           (DetectionFlags, Action, CoudeClass, ModerationGravity, VoiceChannelKind)
    │
    └── ports/
        ├── inbound/                 (traits use case : ManageRulesUseCase, ...)
        └── outbound/                (traits repo : RuleRepository, ...)
```

## Familles de routes HTTP

| Préfixe | Handler | Description |
|---|---|---|
| `/health` | health.rs | Health check (public) |
| `/metrics` | metrics.rs | Prometheus (public, firewall en prod) |
| `/analyze`, `/analyze/image` | analyze*.rs | Inférence IA (heavy rate limit) |
| `/api/analytics/*` | analytics.rs | Stats hot (heavy rate limit) |
| `/api/ai/jobs`, `/api/ai/jobs/:id` | ai_jobs.rs | **Phase 4** — queue async IA |
| `/api/rules/*` | rules.rs | Règles de modération |
| `/api/infractions/*` | infractions.rs | Historique infractions |
| `/api/tickets/*` | tickets.rs | Support tickets |
| `/api/security/*` | security.rs | Événements détection |
| `/api/moderation/*` | moderation.rs | Actions bans/mutes/warns |
| `/api/strikes/*` | strikes.rs | Système strikes |
| `/api/voice-channels/*` | voice_channels.rs | Salons vocaux temporaires |
| `/api/conduct/*` | conduct.rs | Points de conduite |
| `/api/levels/*` | levels.rs | XP / niveaux |
| `/api/coude/*` | coude/*.rs | Jeu Coup de Coude (8 sous-handlers) |
| `/api/blackjack/*` | blackjack/*.rs | Blackjack |
| `/api/games/*` | games.rs | Mini-jeux |
| `/api/wallet/*` | wallet.rs | Porte-monnaie partagé |
| `/api/audit-logs/*` | audit_logs.rs | Logs d'audit |
| `/api/watched-users/*` | watched_users.rs | Dossiers surveillés |
| `/api/discord-roles/*` | discord_roles.rs | Sync rôles Discord |
| `/api/members/*`, `/api/guilds/*` | guild_members.rs | Membres guilds |
| `/api/welcome/*` | welcome.rs | Config bienvenue |
| `/api/models/*` | models_status.rs | Status ONNX + reload |
| `/api/cache/*` | cache_stats.rs | Monitoring cache hit/miss |
| `/api/bots/*` | bot_config.rs | Config des bots par guild |
| `/api/name-history`, `/api/sponsorships`, `/api/temp-roles`, ... | bot_persistence.rs | Persistance fire-and-forget |

## Layers Axum (ordre de traversée)

```
Request → CORS → SetRequestId → Trace → BodyLimit → PropagateRequestId
        → Compression (zstd+gzip) → Metrics middleware → ApiLogger
        → [Route match]
           protected: rate_limit → auth (Bearer) → guild_auth (X-Discord-Token) → handler
           public: handler (health, metrics)
        → Response
```

## Dépendances externes

- **PostgreSQL** (persistance — via PgBouncer depuis Phase 2 A.6)
- **Redis** (cache + pub/sub)
- **ONNX Runtime** (`ort` crate) — inférence texte + vision
- **Discord API** (via `DiscordApiService` pour bans/unbans/get_user_guilds)
- **Tokio** (runtime async)

## Variables d'env clés

| Variable | Défaut | Rôle |
|---|---|---|
| `DATABASE_URL` | — | Connexion PostgreSQL (via PgBouncer en prod) |
| `REDIS_URL` | — | Connexion Redis |
| `API_KEY` | — | Bearer token exigé sur toutes les routes protégées |
| `REQUIRE_API_KEY` | `true` | Active/désactive l'auth (dev only) |
| `PORT` | 3000 | Port d'écoute |
| `RATE_LIMIT_PER_SEC` | 50 | Limit standard (token bucket par IP) |
| `HEAVY_RATE_LIMIT_PER_SEC` | 5 | Limit pour `/analyze*` et `/api/analytics/*` |
| `MAX_BODY_SIZE` | 10 MB | Limite taille de requête |
| `ALLOWED_ORIGINS` | `*` | CORS |
| `VISION_MODEL_PATH` | — | Path vers `vision_sentinel.onnx` |
| `TEXT_MODEL_PATH` | — | Path vers `text_sentinel.onnx` |
| `TEXT_TOKENIZER_PATH` | — | Path vers `tokenizer.json` |
| `AUTOMOD_DISCORD_TOKEN` | — | Token du bot utilisé pour les appels Discord API |

## Observabilité (Phase 0)

- **Prometheus** — endpoint `/metrics` public, compteurs `http_requests_total{route,method,status}`, histogrammes `http_request_duration_seconds{...}`. Cardinalité bornée via `MatchedPath` (les paramètres du path partagent un seul label).
- **Tracing structuré** — `tracing-subscriber` JSON en prod, correlation IDs `X-Request-ID` propagés
- **Middleware `api_logger`** — enregistre chaque requête/réponse dans la table `logs`
- **TraceLayer** — spans Axum avec request_id, method, uri, status, latency
- **tokio-metrics** — sampler runtime toutes les 10 s (utilise uniquement les champs stables, pas besoin de `tokio_unstable`)

## Phase 2 — optimisations appliquées

- Vues matérialisées : `mv_coude_leaderboard`, `mv_wallet_leaderboard`, `mv_level_leaderboard` (3 repos lisent depuis les MV, refresh 5 min par cache-worker)
- Enums Postgres : `coude_class`, `moderation_gravity`, `voice_channel_kind` + value_objects Rust `#[derive(sqlx::Type)]`
- `discord_roles.permissions` en BIGINT (était TEXT)
- 4 tables event-heavy partitionnées RANGE mensuel
- Middleware `guild_auth_middleware` pour le filtrage multi-tenant par guild Discord
- API key pool sqlx à 20, compression zstd+gzip, pool de reqwest tuné
