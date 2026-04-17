# DiscordSentinel

Plateforme de moderation distribuee pour serveurs Discord. Architecture microservices : bots legers (interfaces), API centrale (intelligence), gateway WebSocket (temps reel), app desktop (administration), inference IA (ONNX), workers periodiques.

> 📖 **Documentation par composant** : [`docs/components/`](./docs/components/) — 1 fichier MD par bot/worker/service/app (29 au total).
> 🗺️ **Roadmap** : [`docs/ROADMAP.md`](./docs/ROADMAP.md) — 7 phases, avec tableau `État d'avancement`.
> 📋 **Différés phases 0-2** : [`docs/PHASES_0_2_DIFFERES.md`](./docs/PHASES_0_2_DIFFERES.md).

---

## 📊 État d'avancement (au 2026-04-10)

| Phase | Status | Description |
|---|---|---|
| **0** Observabilité | ✅ | pg_stat_statements, Prometheus + Grafana, metrics Axum, tokio-metrics |
| **1** Quick wins | ✅ | jemalloc + LTO, cache Serenity tiers, compression zstd/gzip, keep-alive reqwest |
| **2** Fondations DB + multi-tenant | ✅ *(partielle)* | Migrations 100-104, MV leaderboards, enums Postgres, partitionnement, PgBouncer, middleware `guild_auth` (X-Discord-Token) |
| **3** Refactor god files | ✅ | Split hexagonal de `coude.rs` (2370L) en 8 sous-fichiers |
| **4** ai-worker + workers prio | ✅ *(partielle)* | Queue async IA (mig 105), `ai-worker`, `temp-roles-worker`, enrichissement sanction-expiry |
| **5** Cache + Streams + Batch writes | ⏸️ à faire | — |
| **6** Features moderation + workers 2 | ⏸️ à faire | — |
| **7** gRPC + scaling horizontal | ⏸️ à faire | — |

**Tests : 237/237 passent** côté API (`cargo test --lib`). Tous les workers et bots compilent clean.

---

## Architecture globale

```
Discord Messages / Events / Images
       │
       ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                       15 Bots Discord (Serenity)                         │
│  audit automod blackjack cleanup community coude game image             │
│  moderation progression roles security ticket voice welcome             │
└────────────┬──────────────────────────────────┬────────────────────────┘
             │ HTTP (BaseApiClient keep-alive)   │ Redis pub/sub
             ▼                                   ▼
┌────────────────────────┐          ┌────────────────────────────┐
│  API backend (Axum)    │◄─────────┤  Gateway WebSocket         │
│  services/api          │          │  (relay Redis → desktop)   │
│  - ~105 migrations     │          └──────────┬─────────────────┘
│  - ONNX inference      │                     │
│  - 34 handlers HTTP    │                     │
│  - Hexagonal ports/adpt│                     │
│  - guild_auth (tenant) │                     │
└────┬──────────────┬────┘                     │
     │              │                          │
     │ PostgreSQL   │ Redis                    │
     ▼              ▼                          ▼
┌───────────┐  ┌───────────┐        ┌──────────────────────┐
│ Postgres  │  │  Redis    │        │  Desktop Tauri       │
│(PgBouncer)│  │ (cache +  │        │  Vue 3 + Pinia       │
│ + MV +    │  │  pub/sub) │        │  OAuth2 Discord      │
│ partitions│  │           │        │  WebSocket live      │
└─────┬─────┘  └─────┬─────┘        └──────────────────────┘
      │              │
      │    ┌─────────┴──────────────────────────────┐
      ▼    ▼                                         ▼
┌────────────────────────────────────┐      ┌──────────────────┐
│  8 Workers periodiques (Tokio)     │      │  AI Training API │
│  ai analytics cache cleanup        │      │  (Python FastAPI)│
│  coude moderation monitoring       │      │  PyTorch + ONNX  │
│  temp-roles                        │      │  fine-tuning     │
└────────────────────────────────────┘      └──────────────────┘
```

**Philosophie** : Bots = interfaces legeres | API = cerveau (decisions + IA + persistance) | Gateway = temps reel | Workers = jobs periodiques DB-bound | Desktop = controle admin | ai-api = training ML a la demande.

---

## Stack technique

| Composant | Technologie | Détails |
|---|---|---|
| API Backend | Rust / Axum 0.8 / Tokio / sqlx 0.8 | Architecture hexagonale, 34 handlers, ~105 migrations, ONNX inference |
| Gateway WebSocket | Rust / Axum 0.8 / Redis pub/sub | Service dédié temps-réel, auto-reconnect exponential backoff |
| 15 Bots Discord | Rust / Serenity 0.12 / librairie `shared` | Cache tiers (minimal/small/medium/full) — Phase 1 |
| 8 Workers | Rust / Tokio / sqlx / librairie `worker-common` | `spawn_periodic` + heartbeat + observabilité Prometheus |
| Base de données | PostgreSQL 16 + **PgBouncer** | ~105 migrations, 4 tables partitionnées RANGE mensuel, 3 vues matérialisées |
| Cache | Redis 7 | `maxmemory=2gb allkeys-lru`, pub/sub events, cache `user_guilds` pour multi-tenant |
| Inference IA | ONNX Runtime 2.0 / ndarray / tokenizers | Vision (NSFW/illicite) + Text (sentiments) |
| Desktop Frontend | Vue 3 + TypeScript + Vite + Pinia + Chart.js | 29 pages, composants atomic design |
| Desktop Backend | Tauri 2 + Rust | Hexagonal, LMDB chiffré AES-256-GCM, WebSocket client |
| Entrainement IA | Python + PyTorch + Transformers + ONNX | 2 modèles : vision EfficientNetV2-S + text DistilBERT multilingual |
| Observabilité | Prometheus + Grafana + tokio-metrics | Middleware Axum metrics, dashboards provisionnés |
| Containerisation | Docker Alpine multi-stage + Compose | ~20 services orchestrés |

**Dépendances Rust clés** : `serde`, `reqwest 0.12` (pool tuné Phase 1), `sqlx 0.8`, `chrono`, `uuid`, `tracing`, `async-trait`, `tower-http` (CORS, compression zstd/gzip, rate limit, request-id), `ort` (ONNX), `tokenizers`, `ndarray`, `redis`, `metrics-exporter-prometheus`, `tokio-metrics`, `tikv-jemallocator` (Linux/macOS).

---

## Structure du projet

```
DiscordSentinel/
├── apps/
│   └── desktop/                       → docs/components/apps/desktop.md
│
├── services/
│   ├── api/                           → docs/components/services/api.md
│   │   ├── src/
│   │   │   ├── adapters/inbound/http/    # 34 handlers, 4 middlewares (auth, guild_auth, rate_limit, api_logger)
│   │   │   ├── adapters/outbound/        # 14 repos postgres, redis_cache
│   │   │   ├── application/              # 20+ use case services
│   │   │   ├── domain/                   # entities, value_objects, services (ONNX, Discord API)
│   │   │   └── ports/                    # traits inbound/outbound
│   │   └── migrations/                   # 001 → 105 (pg_stat_statements, quick wins, MV, enums, partitions, ai_jobs)
│   │
│   ├── gateway/                       → docs/components/services/gateway.md
│   │
│   └── workers/
│       ├── worker-common/             # Librairie partagée (pg_pool, spawn_periodic, observability)
│       ├── ai-worker/                 # Phase 4 A — queue async IA
│       ├── analytics-worker/          # Snapshots quotidiens/horaires
│       ├── cache-worker/              # 6 jobs : warm caches + MV refresh + user_cache sync + partitions
│       ├── cleanup-worker/            # Rétention DB + VACUUM
│       ├── coude-worker/              # Expiration combats + résolution paris
│       ├── moderation-worker/         # Conduct regen + cleanup bans + rappels sanctions (Redis)
│       ├── monitoring-worker/         # Détection offline/online via Redis
│       └── temp-roles-worker/         # Phase 4 B — scan temp_roles → publish Redis
│
├── bots/                              → docs/components/bots/
│   ├── shared/                        # Librairie partagée (api_client, cache_settings, embeds, etc.)
│   ├── audit-bot/    automod-bot/    blackjack-bot/   cleanup-bot/    community-bot/
│   ├── coude-bot/    game-bot/       image-bot/       moderation-bot/  progression-bot/
│   └── roles-bot/    security-bot/   ticket-bot/      voice-bot/       welcome-bot/
│
├── ai/                                → docs/components/services/ai-api.md
│   ├── api/                           # FastAPI Python (training ML)
│   ├── training/                      # text/ + vision/ : datasets, configs, checkpoints, exports
│   └── shared/
│
├── infra/                             # Phase 0 — observabilité
│   ├── prometheus/prometheus.yml
│   └── grafana/                       # datasources + dashboards provisionnés
│
├── scripts/                           # build-all.sh, dev.sh, health-check.sh, seed-rules.sh, start-all.sh
│
├── docs/
│   ├── ROADMAP.md                     # 7 phases + état d'avancement
│   ├── PHASES_0_2_DIFFERES.md         # Ce qui a été reporté
│   ├── BASELINE_METRICS.md            # Template baseline (Phase 0)
│   ├── DB_OPTIMISATIONS.md, OPTIMISATIONS_PERFORMANCES.md, WORKERS_PROPOSES.md
│   ├── MULTI_TENANT_AUTH.md, ESTIMATION_RAM_PROD.md
│   └── components/                    # ✅ Doc par composant (29 fichiers MD)
│       ├── README.md                  # Index + diagramme archi
│       ├── bots/                      # 15 bots + shared
│       ├── workers/                   # 8 workers + worker-common
│       ├── services/                  # api, gateway, ai-api
│       └── apps/                      # desktop
│
├── docker-compose.yml                 # ~20 services (infra + bots + workers + monitoring profil)
├── .env.example
└── README.md                          # ← ce fichier
```

---

## Schéma base de données

**PostgreSQL 16** derrière **PgBouncer** (transaction pooling depuis Phase 2 A.6). **~105 migrations** versionnées.

### Optimisations Phase 2

- **Index** : GIN sur `infractions.flags` / `security_events.user_ids` / `bot_definitions.config_schema` ; partials soft-delete sur `voice_channels` + `tickets` ; ~30 colonnes `TEXT → VARCHAR(20)` pour les Discord IDs (migration 101 idempotente PL/pgSQL).
- **Vues matérialisées** (`mv_coude_leaderboard`, `mv_wallet_leaderboard`, `mv_level_leaderboard`) refreshées toutes les 5 min par `cache-worker`. Gain 100-1000× sur les endpoints leaderboard.
- **Enums Postgres** : `coude_class`, `moderation_gravity`, `voice_channel_kind` avec wrappers Rust `#[derive(sqlx::Type)]`. `discord_roles.permissions` passé de `TEXT` à `BIGINT`.
- **Partitionnement RANGE mensuel** sur 4 tables hot : `infractions`, `audit_logs`, `user_activity_log`, `logs`. 12 partitions pré-créées + `DEFAULT`, génération automatique M+1/M+2 par `cache-worker` job `manage_partitions`.
- **Table `user_cache`** (Phase 2 A.2) : source de vérité des usernames Discord, alimentée par un job périodique qui agrège depuis 4 tables hot.
- **Table `ai_jobs`** (Phase 4 A, migration 105) : file d'attente asynchrone pour l'inférence IA.

### Tables principales (extrait)

| Table | Description |
|---|---|
| `rules` / `infractions` | Règles de modération + infractions enregistrées |
| `tickets` / `ticket_messages` | Système de tickets support |
| `moderation_actions` | Historique modération manuelle (enum `moderation_gravity`) |
| `security_events` | Détection raid / alt accounts (JSONB user_ids + GIN) |
| `audit_logs` **(partitionné)** | Logs d'audit Discord |
| `user_activity_log` **(partitionné)** | Activité utilisateur pour surveillance |
| `logs` **(partitionné)** | Logs applicatifs bots/workers |
| `user_stats` / `user_levels` / `user_wallets` | Stats, XP, wallets (3 vues matérialisées) |
| `coude_*` | 12 tables du jeu Coup de Coude (refactor hexagonal Phase 3) |
| `voice_channels` + sub-tables | Salons vocaux temporaires (enum `voice_channel_kind`) |
| `bot_guild_config` / `bot_definitions` | Config per-guild + schéma de config par bot |
| `sanction_reminders` | Rappels 24h avant expiration mute/ban (enriched Phase 4 B) |
| `temp_roles` | Rôles temporaires (scan par `temp-roles-worker`) |
| `ai_jobs` | Queue IA async (Phase 4 A) |

### Flag types supportés (10)

| Type | Source | Poids défaut |
|---|---|---|
| `spam` / `insult` / `link` / `phishing` | Bot automod | 3.0 / 5.0 / 1.0 / 7.0 |
| `nsfw` / `illicit` | IA Vision ONNX | 8.0 / 9.0 |
| `anger` / `rage` / `threat` / `harassment` | IA Text ONNX | 3.0 / 6.0 / 8.0 / 7.0 |

---

## Endpoints API

**Authentification** : `Authorization: Bearer <API_KEY>` obligatoire (sauf `/health` et `/metrics`). Le middleware `guild_auth_middleware` (Phase 2 B) filtre en plus par `X-Discord-Token` si présent.

| Préfixe | Handler | Description |
|---|---|---|
| `/health`, `/metrics` | health / metrics | Publics (Prometheus + healthcheck) |
| `/analyze`, `/analyze/image` | analyze*.rs | Inférence IA **synchrone** (rate limit strict) |
| `/api/ai/jobs`, `/api/ai/jobs/:id` | ai_jobs.rs | **Phase 4** — queue **async** (POST = 202 + job_id, GET = status) |
| `/api/analytics/*` | analytics.rs | Heatmap, action distribution, top infractors, trends |
| `/api/rules/*` | rules.rs | Règles de modération |
| `/api/infractions/*` | infractions.rs | Historique infractions |
| `/api/tickets/*` | tickets.rs | Support tickets |
| `/api/moderation/*` | moderation.rs | log_action, history, bans |
| `/api/security/*` | security.rs | Events de détection |
| `/api/strikes/*` | strikes.rs | Système de strikes (escalade) |
| `/api/notes/*`, `/api/reminders/*` | notes / reminders | Notes mod + rappels temporaires |
| `/api/voice-channels/*` | voice_channels.rs | Salons vocaux dynamiques |
| `/api/conduct/*`, `/api/levels/*` | conduct / levels | Points de conduite + XP/niveaux |
| `/api/coude/*` | coude/ (8 sous-handlers) | Jeu Coup de Coude (refactor hexagonal Phase 3) |
| `/api/blackjack/*`, `/api/games/*`, `/api/wallet/*` | blackjack / games / wallet | Jeux + porte-monnaie partagé |
| `/api/audit-logs/*`, `/api/watched-users/*` | audit_logs / watched_users | Logs d'audit + dossiers surveillés |
| `/api/discord-roles/*` | discord_roles.rs | Sync rôles Discord (permissions BIGINT depuis Phase 2) |
| `/api/members/*`, `/api/guilds/*` | guild_members.rs | Membres + guilds |
| `/api/welcome/*` | welcome.rs | Config bienvenue |
| `/api/models/*`, `/api/cache/*` | models_status / cache_stats | Monitoring ONNX + cache |
| `/api/bots/*` | bot_config.rs | Config per-guild des bots |

> Détail complet des handlers : [`docs/components/services/api.md`](./docs/components/services/api.md).

---

## Bots Discord (15)

Chaque bot a sa fiche dans [`docs/components/bots/`](./docs/components/bots/). Résumé :

| Bot | Rôle | Cache tier |
|---|---|---|
| [automod](./docs/components/bots/automod-bot.md) | Auto-modération (spam, phishing, insultes, unicode) | small |
| [moderation](./docs/components/bots/moderation-bot.md) | `/ban /mute /warn /history /note /call /appeal` + templates | full |
| [security](./docs/components/bots/security-bot.md) | Anti-raid, alt detection, quarantaine, lockdown | medium |
| [audit](./docs/components/bots/audit-bot.md) | Logs d'audit + anomaly detection + rapport hebdo | medium |
| [ticket](./docs/components/bots/ticket-bot.md) | Tickets + SLA + transcripts + satisfaction | small |
| [image](./docs/components/bots/image-bot.md) | Analyse images NSFW/illicite + hash cache | small |
| [voice](./docs/components/bots/voice-bot.md) | Salons vocaux temporaires + AFK + vote-kick + themes | full |
| [progression](./docs/components/bots/progression-bot.md) | XP, niveaux, badges, streaks, multipliers | small |
| [community](./docs/components/bots/community-bot.md) | Rôles temporaires, panels, parrainage | minimal |
| [roles](./docs/components/bots/roles-bot.md) | Sync rôles Discord ↔ API + panels réactionnels | small |
| [coude](./docs/components/bots/coude-bot.md) | Jeu RPG PvP (combat, inventaire, shop, classes) | minimal |
| [blackjack](./docs/components/bots/blackjack-bot.md) | Tables blackjack interactives + wallet | minimal |
| [game](./docs/components/bots/game-bot.md) | Mini-jeux textuels (devinettes, trivia) | minimal |
| [welcome](./docs/components/bots/welcome-bot.md) | Messages de bienvenue templés | minimal |
| [cleanup](./docs/components/bots/cleanup-bot.md) | Purges manuelles staff (DB + messages Discord) | minimal |
| [shared](./docs/components/bots/shared.md) | **Librairie partagée** (api_client, cache_settings, embeds) | — |

**Tiers cache Serenity (Phase 1)** : `minimal` (aucun msg) / `small` (channels) / `medium` (100 msg/ch) / `full` (défaut). Gain RAM typique −30 à −50 %.

---

## Workers (8)

Chaque worker a sa fiche dans [`docs/components/workers/`](./docs/components/workers/). Résumé :

| Worker | Jobs périodiques |
|---|---|
| [ai-worker](./docs/components/workers/ai-worker.md) | `drain_ai_jobs` (2s) — dépile `ai_jobs`, dispatch vers API, publie Redis |
| [analytics-worker](./docs/components/workers/analytics-worker.md) | `daily_snapshot` (1h), `hourly_snapshot` (1h) |
| [cache-worker](./docs/components/workers/cache-worker.md) | **6 jobs** : warm analytics/dashboard/voice stats, refresh MV leaderboards (5min), sync user_cache (15min), manage_partitions (24h) |
| [cleanup-worker](./docs/components/workers/cleanup-worker.md) | `cleanup_old_data` (1h), `vacuum_tables` (24h) |
| [coude-worker](./docs/components/workers/coude-worker.md) | `expire_combats` (24h), `resolve_betting` (30s) |
| [moderation-worker](./docs/components/workers/moderation-worker.md) | `conduct_regen` (1h), `cleanup_bans` (1min), `sync_ban_proposals` (2min), `send_reminders` (30s) — enrichi Phase 4 pour publier `sanction_expiry_reminder` sur Redis |
| [monitoring-worker](./docs/components/workers/monitoring-worker.md) | Boucle manuelle (30s) — détection offline/online via Redis |
| [temp-roles-worker](./docs/components/workers/temp-roles-worker.md) | `expire_temp_roles` (1min) — scan `temp_roles`, publie event Redis |
| [worker-common](./docs/components/workers/worker-common.md) | **Librairie partagée** (pg_pool, `spawn_periodic`, heartbeat, observability Prometheus) |

---

## Inférence IA (ONNX)

### Modèle Vision — détection images

| Propriété | Valeur |
|---|---|
| Architecture | EfficientNetV2-S |
| Classes | `safe`, `nsfw`, `illicit` |
| Input | 224×224 normalisé ImageNet |
| Format | ONNX (opset 17) |

### Modèle Text — détection sentiments

| Propriété | Valeur |
|---|---|
| Architecture | DistilBERT multilingual |
| Classes | `neutral`, `anger`, `rage`, `threat`, `harassment` |
| Input | Tokens (max 256) + attention mask |
| Tokenizer | HuggingFace tokenizers (Rust) |

Les modèles sont chargés au démarrage de l'API. Mode dégradé automatique si absents (scoring règles seulement). L'**ai-api** Python ([`ai/api/`](./ai/api/)) permet de fine-tuner et exporter de nouveaux ONNX à la demande depuis la page `IaTrainingPage` du desktop.

### Config IA per-guild

Table `ia_config` : `text_enabled`, `text_threshold`, `vision_enabled`, `vision_threshold` — sliders temps-réel dans le desktop. Endpoints `GET/PUT /api/ia-config/{guild_id}`.

### Mode async (Phase 4)

Alternative à `POST /analyze` (synchrone, timeout 5s côté bots) : **`POST /api/ai/jobs`** qui retourne `202 Accepted` + `job_id` immédiatement. L'**ai-worker** dépile la file et publie le résultat sur Redis `ai_result:{job_id}` (+ `SET` avec TTL 600s). Les bots peuvent opt-in progressivement.

---

## Multi-tenant (Phase 2 B)

L'API expose un middleware **`guild_auth_middleware`** qui filtre les requêtes selon l'appartenance Discord du user appelant :

1. Le desktop fait OAuth2 Discord (scope `identify` + `guilds`) → `access_token`
2. L'`ApiAdapter` envoie ce token dans un header `X-Discord-Token` sur toutes les requêtes
3. Le middleware extrait le `guild_id` de l'URI, interroge Discord `/users/@me/guilds` (cache Redis 5 min par hash de token), et refuse `403` si le guild n'est pas dans la liste autorisée
4. Si `X-Discord-Token` est absent (appel bot/internal), le middleware est **pass-through** — l'`auth_middleware` (Bearer API key) reste obligatoire

Cela permet de partager l'app desktop à un admin externe en toute sécurité.

---

## Gateway WebSocket

Service dédié au temps réel, séparé de l'API. Relay Redis pub/sub → WebSocket.

| Propriété | Valeur |
|---|---|
| Port | 3001 |
| Auth | `?token=<api_key>` |
| Max connexions | 1000 (configurable) |
| Reconnexion Redis | Exponential backoff |
| Healthcheck | `GET /health` |

**Events broadcasts** : `infraction_new`, `ticket_new/message/closed/assigned`, `security_event`, `moderation_action`, `sanction_expiry_reminder`, `temp_role_expire`, `bot_log`, etc.

---

## Middleware API (ordre de traversée)

```
Request
  → CORS
  → SetRequestId + TraceLayer
  → BodyLimit (10MB par défaut)
  → CompressionLayer (zstd + gzip)  ← Phase 1
  → metrics_middleware (Prometheus)
  → api_logger (→ table logs)
  → [si route protégée]
      → rate_limit (token bucket IP)
      → auth (Bearer API key)
      → guild_auth (X-Discord-Token) ← Phase 2 B
  → Handler
```

| Middleware | Rôle |
|---|---|
| `auth` | Bearer `API_KEY`, mode dev si vide |
| `guild_auth` | **Phase 2 B** — multi-tenant par Discord OAuth2 |
| `rate_limit` | Token bucket par IP, standard 50 req/s, heavy 5 req/s sur `/analyze*` |
| `api_logger` | Enregistre chaque requête/réponse dans la table `logs` |
| `metrics` | Prometheus counter + histogram, cardinalité bornée via `MatchedPath` |
| Compression | zstd + gzip, négociation `Accept-Encoding` |

**Rate limit inférence ONNX** : semaphore (`INFERENCE_MAX_CONCURRENT=4`) + token bucket (`INFERENCE_MAX_PER_SEC=20`). HTTP 429 si dépassement.

---

## Observabilité (Phase 0)

- **Prometheus** — endpoint `/metrics` sur l'API et chaque worker (port 9100). Compteurs `http_requests_total{route,method,status}`, histogrammes `http_request_duration_seconds`, gauges `tokio_busy_ratio`, `tokio_live_tasks_count`, `tokio_global_queue_depth` (champs stables uniquement).
- **Grafana** — dashboards auto-provisionnés dans `infra/grafana/`. Démarrage : `docker compose --profile monitoring up -d prometheus grafana`. UI sur `http://localhost:3002` (admin/admin).
- **pg_stat_statements** — extension activée (migration 099), permet d'identifier les queries lentes via `SELECT * FROM pg_stat_statements ORDER BY total_exec_time DESC`.
- **Tracing structuré** — `tracing-subscriber` JSON en prod, correlation IDs `X-Request-ID` propagés via `tower_http::request_id`.
- **Baseline** — template dans [`docs/BASELINE_METRICS.md`](./docs/BASELINE_METRICS.md) avec requêtes PromQL/SQL prêtes.

---

## Déploiement

### Docker Compose

```bash
# Stack complète (infra + API + 8 workers + 15 bots + gateway + desktop)
docker compose up -d

# Avec Prometheus + Grafana
docker compose --profile monitoring up -d
```

**Services infra** : `postgres` (tuning RAM Phase 2 A.5 : `shared_buffers=4GB`, `work_mem=64MB`, WAL tuning), `pgbouncer` (transaction pooling, max_prepared_statements=100), `redis` (`maxmemory=2gb allkeys-lru`).

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

# Bot Discord unifie (fusion des 15 anciens bots en un seul sentinel-bot)
# Reutilise aussi par l'API et les workers (audit-sync, coude-worker).
SENTINEL_DISCORD_TOKEN=...

# Voice module config
VOICE_GUILD_ID=...
VOICE_PUBLIC_CREATOR_CHANNEL_ID=...
VOICE_PRIVATE_CREATOR_CHANNEL_ID=...
VOICE_LOG_CHANNEL_ID=...
```

### Développement local

```bash
bash scripts/dev.sh              # Lance API + bots + desktop
bash scripts/build-all.sh        # Build release de tous les crates
bash scripts/health-check.sh     # Vérifie que tous les services répondent
bash scripts/seed-rules.sh       # Seed de règles de dev
bash scripts/start-all.sh        # Démarre la stack complète

# Ou composant par composant :
cd services/api && cargo run
cd services/workers/ai-worker && cargo run
cd bots/moderation-bot && cargo run
cd apps/desktop && npm run tauri dev
```

---

## Tests

**237/237 tests** passent côté API (`cd services/api && cargo test --lib`). Tous les crates (API, 8 workers, 15 bots, librairies `shared` + `worker-common`) compilent clean.

Couverture principale :
- API : ScoringService, InferenceService, ValueObjects, MV repos, middleware (`guild_auth` 5 tests)
- Bots : détecteurs automod (214 tests), security (64), ticket (59), audit (29), progression (42), community (40)
- Workers : worker-common helpers
- Gateway : broadcaster (6 tests)

---

## Bonnes pratiques du projet

- **Bots = interfaces légères** — jamais de logique métier dans les bots
- **Toujours passer par l'API** — centralisation des décisions
- **Architecture hexagonale** — separation stricte domain / ports / adapters côté API
- **Workers = jobs périodiques DB-bound** — via `spawn_periodic` + Redis pub/sub, pas de Discord gateway direct
- **Gateway dédié** — découplé de l'API pour absorber les bursts WebSocket
- **Inference IA gracieuse** — si modèles absents, fallback scoring règles
- **Cache Serenity par tier** — RAM minimale par défaut, opt-in par cas d'usage
- **Multi-tenant** — filtre `guild_auth` avec pass-through pour bots/internal
- **Observabilité first** — Phase 0 pose la baseline avant toute optimisation
- **Migrations cross-breaking** — différées et documentées ([`docs/PHASES_0_2_DIFFERES.md`](./docs/PHASES_0_2_DIFFERES.md))

---

## Liens rapides

| Document | Sujet |
|---|---|
| [`docs/ROADMAP.md`](./docs/ROADMAP.md) | Roadmap 7 phases + état d'avancement |
| [`docs/PHASES_0_2_DIFFERES.md`](./docs/PHASES_0_2_DIFFERES.md) | Ce qui a été reporté (phases 0-2) |
| [`docs/components/`](./docs/components/) | Doc détaillée par composant (29 fichiers) |
| [`docs/BASELINE_METRICS.md`](./docs/BASELINE_METRICS.md) | Template baseline Prometheus/PromQL |
| [`docs/DB_OPTIMISATIONS.md`](./docs/DB_OPTIMISATIONS.md) | 12 optimisations schéma Postgres |
| [`docs/OPTIMISATIONS_PERFORMANCES.md`](./docs/OPTIMISATIONS_PERFORMANCES.md) | 12 optimisations perf/scalabilité |
| [`docs/WORKERS_PROPOSES.md`](./docs/WORKERS_PROPOSES.md) | Workers proposés (dont déjà créés) |
| [`docs/MULTI_TENANT_AUTH.md`](./docs/MULTI_TENANT_AUTH.md) | Isolation par guild (OAuth2 + RBAC) |
| [`docs/ESTIMATION_RAM_PROD.md`](./docs/ESTIMATION_RAM_PROD.md) | Tuning build + jemalloc |
