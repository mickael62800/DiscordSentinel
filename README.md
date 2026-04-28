# DiscordSentinel

Plateforme de modération distribuée pour serveurs Discord. Architecture microservices : **un bot Discord unifié** (interface Serenity), **API centrale** (intelligence + IA), **gateway WebSocket** (temps réel), **app web** (administration), **13 workers** périodiques, **inférence ONNX** embarquée.

---

## Architecture globale

```
Discord Messages / Events / Images
       │
       ▼
┌─────────────────────────────────────────────────────────────┐
│      Bot Discord unifié (Serenity 0.12)  — bots/sentinel-bot │
│   13 modules : audit · automod · blackjack · cleanup ·       │
│   community · coude · games · moderation · progression ·     │
│   security · tickets · voice · welcome                       │
└────────────┬─────────────────────────────────┬──────────────┘
             │ HTTP (BaseApiClient keep-alive)  │ Redis pub/sub
             ▼                                  ▼
┌─────────────────────────┐         ┌──────────────────────────┐
│  API backend (Axum 0.8) │◄────────┤  Gateway WebSocket       │
│  services/api           │         │  services/gateway        │
│  - 150 migrations       │         │  (relay Redis → clients) │
│  - ONNX inference       │         └─────────┬────────────────┘
│  - ~40 handlers HTTP    │                   │
│  - Hexagonal            │                   │
│  - guild_auth multi-    │                   │
│    tenant (OAuth2)      │                   │
│  - OAuth Discord        │                   │
└──────┬──────────────┬───┘                   │
       │              │                       │
       │ PostgreSQL   │ Redis                 │
       ▼              ▼                       ▼
┌───────────┐  ┌───────────┐        ┌────────────────────────┐
│ Postgres  │  │  Redis    │        │ apps/web (Vue 3)       │
│(PgBouncer)│  │ (cache +  │        │ OAuth2 Discord + WS    │
│ 150 migs  │  │  pub/sub) │        └────────────────────────┘
└─────┬─────┘  └─────┬─────┘
      │              │
      └──────┬───────┘
             ▼
┌─────────────────────────────────────────┐
│ 13 Workers périodiques (Tokio)          │
│ ai · analytics · appeal-sla · audit-    │
│ cache · blackjack-cleanup · cache ·     │
│ cleanup · coude · discord-audit-sync ·  │
│ export · moderation · monitoring ·      │
│ temp-roles                              │
└─────────────────────────────────────────┘
```

**Philosophie** : Bot = interface légère (multi-module dans un seul process) · API = cerveau (décisions + IA + persistance) · Gateway = temps réel découplé · Workers = jobs DB-bound périodiques · Web = admin remote.

---

## Stack technique

| Composant | Technologie | Détails |
|---|---|---|
| API backend | Rust / Axum 0.8 / Tokio / sqlx 0.8 | Hexagonal, ~40 handlers, 150 migrations, ONNX inference, OAuth Discord |
| Gateway WebSocket | Rust / Axum 0.8 / Redis pub/sub | Service dédié temps réel, auto-reconnect exponential backoff |
| Bot Discord unifié | Rust / Serenity 0.12 / lib `sentinel-shared` | Process unique, 13 modules chargés dynamiquement selon config per-guild |
| 13 Workers | Rust / Tokio / sqlx / lib `worker-common` | `spawn_periodic` + heartbeat + métriques Prometheus |
| gRPC (Phase 7) | `tonic` 0.12 + `prost` 0.13 | Crate `services/proto` (amorce scaling horizontal) |
| PostgreSQL | Postgres 16 + **PgBouncer** | 150 migrations, partitionnement RANGE mensuel, vues matérialisées |
| Cache | Redis 7 | `maxmemory=2gb allkeys-lru`, pub/sub events, cache `user_guilds` multi-tenant |
| Inférence IA | ONNX Runtime 2.0 / ndarray / tokenizers | Vision (NSFW/illicite) + Text (sentiments multilingues) |
| Web dashboard | Vue 3 + TS + Vite + Pinia + Chart.js | `apps/web` — servi par Nginx (Dockerfile + nginx.conf) |
| Observabilité | Prometheus + Grafana + tokio-metrics | Middleware Axum metrics, dashboards provisionnés |
| Containerisation | Docker Alpine multi-stage + Compose | Infra + API + gateway + bot + workers + web + monitoring |

**Dépendances workspace** : `tokio`, `serde`, `reqwest 0.12` (rustls, pool tuné), `sqlx 0.8`, `chrono`, `uuid`, `tracing`, `async-trait`, `tower-http` (CORS, compression zstd/gzip, rate limit, request-id), `ort` (ONNX), `tokenizers`, `ndarray`, `redis 0.27` (streams), `tonic`/`prost` (gRPC), `metrics-exporter-prometheus`, `tokio-metrics`, `tikv-jemallocator` (Linux/macOS). Profil release : `lto = "thin"`, `codegen-units = 16`, `strip = true`.

---

## Structure du projet

```
DiscordSentinel/
├── apps/
│   └── web/                     # Vue 3 web dashboard (Pinia, vue-router, Chart.js, Nginx)
│
├── bots/
│   ├── sentinel-bot/            # Bot Discord unifié (single process)
│   │   └── src/modules/         # audit · automod · blackjack · cleanup · community ·
│   │                            # coude · games · moderation · progression · security ·
│   │                            # tickets · voice · welcome (13 modules)
│   └── shared/                  # Lib `sentinel-shared` (api_client, cache_settings, embeds, ...)
│
├── services/
│   ├── api/
│   │   ├── src/
│   │   │   ├── adapters/inbound/http/   # handlers, middlewares (auth, guild_auth, rate_limit, api_logger, metrics)
│   │   │   ├── adapters/outbound/       # repositories postgres, redis_cache
│   │   │   ├── application/             # use case services
│   │   │   ├── domain/                  # entities, value_objects, services (ONNX, Discord API)
│   │   │   └── ports/                   # traits inbound/outbound
│   │   └── migrations/                  # 001 → 150 (partitions, MV, enums, ai_jobs, welcome rich embeds, ...)
│   │
│   ├── gateway/                 # WebSocket relay (Redis pub/sub → clients)
│   ├── proto/                   # Définitions gRPC (Phase 7 — `tonic` + `prost`)
│   │
│   └── workers/
│       ├── worker-common/       # Lib partagée (pg_pool, spawn_periodic, heartbeat, observability)
│       ├── ai-worker/           # Queue async IA (drain ai_jobs → API → Redis)
│       ├── analytics-worker/    # Snapshots quotidiens/horaires
│       ├── appeal-sla-worker/   # SLA appels de sanction
│       ├── audit-cache-worker/  # Cache des audit-logs Discord
│       ├── blackjack-cleanup-worker/  # Nettoyage tables blackjack
│       ├── cache-worker/        # Warm caches + refresh MV + sync user_cache + manage_partitions
│       ├── cleanup-worker/      # Rétention DB + VACUUM
│       ├── coude-worker/        # Expiration combats + résolution paris
│       ├── discord-audit-sync-worker/  # Ingest continu des audit-logs Discord
│       ├── export-worker/       # Exports asynchrones (CSV, JSON, ...)
│       ├── moderation-worker/   # Conduct regen + cleanup bans + rappels sanctions
│       ├── monitoring-worker/   # Détection offline/online via Redis
│       └── temp-roles-worker/   # Scan temp_roles → publish Redis
│
├── ai/                          # Configs d'entraînement (YAML) + dossiers d'exports ONNX (montés par Docker)
│
├── infra/                       # prometheus.yml + grafana provisioning
├── scripts/                     # build-all.sh, dev.sh, health-check.sh, seed-rules.sh, start-all.sh
├── docs/                        # COUP_DE_COUDE_*.md, commandes-utilisateur.md, cmd_discord/, amélioration/
│
├── docker-compose.yml           # Stack complète (infra + API + bot + 13 workers + gateway + web + monitoring)
├── docker-compose.test.yml      # Stack de tests
├── Cargo.toml                   # Workspace Rust (20+ membres)
└── README.md                    # ← ce fichier
```

---

## Base de données

**PostgreSQL 16** derrière **PgBouncer** (transaction pooling). **150 migrations** versionnées (`services/api/migrations/001 → 150`).

### Optimisations structurelles

- **Vues matérialisées** (`mv_coude_leaderboard`, `mv_wallet_leaderboard`, `mv_level_leaderboard`) refreshées toutes les 5 min par `cache-worker`. Gain 100–1000× sur les endpoints leaderboard.
- **Partitionnement RANGE mensuel** sur 4 tables hot : `infractions`, `audit_logs`, `user_activity_log`, `logs`. Génération automatique M+1/M+2 par `cache-worker`.
- **Enums Postgres** : `coude_class`, `moderation_gravity`, `voice_channel_kind` (wrappers Rust `#[derive(sqlx::Type)]`). `discord_roles.permissions` en `BIGINT`.
- **Index GIN** sur `infractions.flags`, `security_events.user_ids`, `bot_definitions.config_schema`. Partials soft-delete sur `voice_channels` + `tickets`. Discord IDs typés `VARCHAR(20)`.
- **Table `user_cache`** : source de vérité des usernames Discord, alimentée par agrégation périodique depuis 4 tables hot.
- **Table `ai_jobs`** : file d'attente asynchrone pour l'inférence IA (consommée par `ai-worker`).

### Tables principales (extrait)

| Table | Description |
|---|---|
| `rules` / `infractions` | Règles de modération + infractions (flags JSONB + GIN) |
| `tickets` / `ticket_messages` | Tickets support + SLA |
| `moderation_actions` | Historique modération manuelle (enum `moderation_gravity`) |
| `security_events` | Détection raid / alt accounts |
| `audit_logs` **(partitionné)** | Logs d'audit Discord (ingestés par `discord-audit-sync-worker`) |
| `user_activity_log` **(partitionné)** | Activité utilisateur pour surveillance |
| `logs` **(partitionné)** | Logs applicatifs bot/workers |
| `user_stats` / `user_levels` / `user_wallets` | Stats, XP, wallets (3 vues matérialisées) |
| `coude_*` | 12 tables du jeu Coup de Coude |
| `voice_channels` + sub-tables | Salons vocaux temporaires (enum `voice_channel_kind`) |
| `bot_guild_config` / `bot_definitions` | Config per-guild + schéma de config par module bot |
| `sanction_reminders` | Rappels 24 h avant expiration mute/ban |
| `temp_roles` | Rôles temporaires (scan par `temp-roles-worker`) |
| `ai_jobs` | Queue IA async |
| `welcome_config` | Config bienvenue + rich embeds (migrations 148–150) |

### Flag types supportés (10)

| Type | Source | Poids défaut |
|---|---|---|
| `spam` / `insult` / `link` / `phishing` | Module automod | 3.0 / 5.0 / 1.0 / 7.0 |
| `nsfw` / `illicit` | IA Vision ONNX | 8.0 / 9.0 |
| `anger` / `rage` / `threat` / `harassment` | IA Text ONNX | 3.0 / 6.0 / 8.0 / 7.0 |

---

## Endpoints API (résumé)

**Authentification** : `Authorization: Bearer <API_KEY>` obligatoire (sauf `/health` et `/metrics`). Le middleware `guild_auth_middleware` filtre en plus par `X-Discord-Token` si présent (multi-tenant OAuth2).

| Préfixe | Handler | Description |
|---|---|---|
| `/health`, `/metrics` | health / metrics / system | Publics (Prometheus + healthcheck) |
| `/analyze`, `/analyze/image` | analyze*.rs | Inférence IA **synchrone** (rate limit strict) |
| `/api/ai/jobs` | ai_jobs.rs | Queue **async** (POST = 202 + job_id, GET = status) |
| `/api/analytics/*`, `/api/dashboard/*`, `/api/stats/*` | analytics / dashboard / dashboard_charts / stats | Heatmap, trends, top infractors, KPIs |
| `/api/rules/*` | rules.rs | Règles de modération |
| `/api/infractions/*`, `/api/strikes/*` | infractions / strikes | Infractions + escalade |
| `/api/tickets/*` | tickets.rs | Tickets support |
| `/api/moderation/*`, `/api/purge/*` | moderation / purge | `log_action`, history, bans, purges |
| `/api/security/*` | security.rs | Events de détection |
| `/api/notes/*`, `/api/reminders/*` | notes / reminders | Notes mod + rappels |
| `/api/voice-channels/*` | voice_channels.rs | Salons vocaux dynamiques |
| `/api/conduct/*`, `/api/levels/*` | conduct / levels | Conduite + XP/niveaux |
| `/api/coude/*` | coude/* | Jeu Coup de Coude (hexagonal, 8 sous-handlers) |
| `/api/blackjack/*`, `/api/games/*`, `/api/wallet/*` | blackjack/* / games / wallet | Jeux + porte-monnaie |
| `/api/audit-logs/*`, `/api/watched-users/*`, `/api/user-activity/*` | audit_logs / watched_users / user_activity | Audit + dossiers surveillés |
| `/api/discord-roles/*`, `/api/role-panels/*` | discord_roles / role_panels | Sync rôles + panels réactionnels |
| `/api/members/*`, `/api/guilds/*`, `/api/guild-channels/*` | guild_members / guild_channels | Membres + guilds + salons |
| `/api/welcome/*` | welcome.rs | Config bienvenue + rich embeds |
| `/api/models/*`, `/api/cache/*`, `/api/bots/*` | models_status / cache_stats / bot_config / bot_persistence | Monitoring ONNX + cache + config per-guild |
| `/api/oauth/*`, `/api/rbac/*` | oauth / rbac | OAuth2 Discord + rôles applicatifs |
| `/api/exports/*` | exports.rs | Exports async (délégués à `export-worker`) |

---

## Inférence IA (ONNX)

### Modèle Vision

| Propriété | Valeur |
|---|---|
| Architecture | EfficientNetV2-S |
| Classes | `safe`, `nsfw`, `illicit` |
| Input | 224×224 normalisé ImageNet |
| Format | ONNX (opset 17) |

### Modèle Text

| Propriété | Valeur |
|---|---|
| Architecture | DistilBERT multilingual |
| Classes | `neutral`, `anger`, `rage`, `threat`, `harassment` |
| Input | Tokens (max 256) + attention mask |
| Tokenizer | HuggingFace tokenizers (Rust) |

Les modèles sont chargés au démarrage de l'API. **Mode dégradé** automatique si absents (scoring règles seulement). Les configs d'entraînement vivent dans `ai/training/{text,vision}/configs/`, les exports ONNX sont attendus dans `ai/training/{text,vision}/exports/` (montés en `/models/*` par `docker-compose.yml`). Le pipeline d'entraînement lui-même est externe au repo.

### Config IA per-guild

Centralisée dans `bot_guild_config` (bot_name = `automod-bot`) : `text_enabled`, `text_threshold`, `vision_enabled`, `vision_threshold`, `context_dampening`, `context_format`, `context_max_messages`, `context_max_chars`. Configurable depuis le dashboard web. La table historique `ia_config` a été fusionnée (migration 146).

### Mode async

Alternative à `POST /analyze` (synchrone, timeout 5 s côté bot) : **`POST /api/ai/jobs`** retourne `202 Accepted` + `job_id` immédiatement. L'**ai-worker** dépile la file et publie le résultat sur Redis `ai_result:{job_id}` (+ `SET` TTL 600 s).

---

## Multi-tenant

Middleware **`guild_auth_middleware`** : filtre les requêtes selon l'appartenance Discord du user appelant.

1. Le web fait OAuth2 Discord (scopes `identify` + `guilds`) → `access_token`.
2. L'adaptateur client envoie ce token dans `X-Discord-Token` sur toutes les requêtes.
3. Le middleware extrait le `guild_id` de l'URI, interroge `/users/@me/guilds` (cache Redis 5 min par hash de token), et refuse `403` si le guild n'est pas dans la liste autorisée.
4. Si `X-Discord-Token` est absent (appel bot/worker interne), le middleware est **pass-through** — `auth_middleware` (Bearer API key) reste obligatoire.

---

## Gateway WebSocket

| Propriété | Valeur |
|---|---|
| Port | 3001 |
| Auth | `?token=<api_key>` |
| Max connexions | 1000 (configurable) |
| Reconnexion Redis | Exponential backoff |
| Healthcheck | `GET /health` |

**Events broadcastés** : `infraction_new`, `ticket_new/message/closed/assigned`, `security_event`, `moderation_action`, `sanction_expiry_reminder`, `temp_role_expire`, `bot_log`, etc.

---

## Middleware API (ordre de traversée)

```
Request
  → CORS
  → SetRequestId + TraceLayer
  → BodyLimit (10 MB par défaut)
  → CompressionLayer (zstd + gzip)
  → metrics_middleware (Prometheus)
  → api_logger (→ table logs)
  → [si route protégée]
      → rate_limit (token bucket IP)
      → auth (Bearer API key)
      → guild_auth (X-Discord-Token)
  → Handler
```

**Rate limit inférence ONNX** : semaphore (`INFERENCE_MAX_CONCURRENT=4`) + token bucket (`INFERENCE_MAX_PER_SEC=20`). HTTP 429 si dépassement.

---

## Observabilité

- **Prometheus** — endpoint `/metrics` sur l'API et chaque worker (port 9100). Compteurs `http_requests_total{route,method,status}`, histogrammes `http_request_duration_seconds`, gauges `tokio_busy_ratio`, `tokio_live_tasks_count`, `tokio_global_queue_depth`.
- **Grafana** — dashboards auto-provisionnés dans `infra/grafana/`. Démarrage : `docker compose --profile monitoring up -d prometheus grafana`. UI sur `http://localhost:3002` (admin/admin).
- **pg_stat_statements** — extension activée (migration 099). `SELECT * FROM pg_stat_statements ORDER BY total_exec_time DESC`.
- **Tracing structuré** — `tracing-subscriber` JSON en prod, correlation IDs `X-Request-ID` propagés via `tower_http::request_id`.

---

## Déploiement

### Docker Compose

```bash
# Stack complète (infra + API + bot unifié + 13 workers + gateway + web)
docker compose up -d

# Avec Prometheus + Grafana
docker compose --profile monitoring up -d
```

**Services infra** : `postgres` (tuning RAM : `shared_buffers=4GB`, `work_mem=64MB`, WAL tuning), `pgbouncer` (transaction pooling), `redis` (`maxmemory=2gb allkeys-lru`).

### Variables d'environnement (.env)

```env
# Infrastructure
POSTGRES_PASSWORD=sentinel_secret
REDIS_PASSWORD=sentinel_redis

# API
API_KEY=your_api_key_here
REQUIRE_API_KEY=true

# IA / Inference ONNX
VISION_MODEL_PATH=/models/vision/vision_sentinel.onnx
TEXT_MODEL_PATH=/models/text/text_sentinel.onnx
TEXT_TOKENIZER_PATH=/models/text/tokenizer.json
TEXT_MAX_LENGTH=256

# Bot Discord unifié (réutilisé par l'API et certains workers pour audit-sync, coude, etc.)
SENTINEL_DISCORD_TOKEN=...

# Voice module
VOICE_GUILD_ID=...
VOICE_PUBLIC_CREATOR_CHANNEL_ID=...
VOICE_PRIVATE_CREATOR_CHANNEL_ID=...
VOICE_LOG_CHANNEL_ID=...
```

### Développement local

```bash
bash scripts/dev.sh              # Lance API + bot + web
bash scripts/build-all.sh        # Build release de tous les crates
bash scripts/health-check.sh     # Vérifie que tous les services répondent
bash scripts/seed-rules.sh       # Seed de règles de dev
bash scripts/start-all.sh        # Démarre la stack complète

# Ou composant par composant :
cd services/api && cargo run
cd services/workers/ai-worker && cargo run
cd bots/sentinel-bot && cargo run
cd apps/web && npm run dev
```

---

## Tests

Tests `cargo test --lib` côté API + unitaires workers/modules bot. Couverture principale :

- **API** : ScoringService, InferenceService, ValueObjects, MV repos, middleware (`guild_auth`), handlers (welcome, infractions postgres, repos outbound).
- **Bot unifié** : détecteurs automod, security, tickets, audit, progression, community.
- **Workers** : helpers `worker-common`.
- **Gateway** : broadcaster.

Stack de tests dédiée via `docker-compose.test.yml`.

---

## Bonnes pratiques du projet

- **Bot = interface légère** — logique métier centralisée dans l'API, jamais dans les modules du bot.
- **Architecture hexagonale** côté API — séparation stricte domain / ports / adapters.
- **Workers = jobs périodiques DB-bound** — via `spawn_periodic` + Redis pub/sub, pas de Discord gateway direct (sauf `discord-audit-sync-worker` qui utilise le token bot pour lire les audit-logs).
- **Gateway découplé** — absorbe les bursts WebSocket indépendamment de l'API.
- **Inférence IA gracieuse** — si modèles absents, fallback scoring règles.
- **Multi-tenant** — filtre `guild_auth` avec pass-through pour appels internes.
- **Observabilité first** — métriques Prometheus + traces JSON + `pg_stat_statements`.
- **Workspace Rust partagé** — deps communes (tokio, serde, reqwest, tonic, serenity, redis) déclarées une seule fois dans le `Cargo.toml` racine, `lto = "thin"` pour garder les builds rapides.
