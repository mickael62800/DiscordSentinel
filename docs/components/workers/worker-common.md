# worker-common — Librairie partagée des workers

**Rôle** : Librairie commune à tous les workers. Fournit l'infrastructure réutilisable : pool PostgreSQL, scheduling périodique, heartbeats, observabilité Prometheus, shutdown gracieux et hiérarchie de config (DB > env > défaut).

Ce n'est **pas un worker** mais une dépendance `path = "../worker-common"` dans le `Cargo.toml` de chaque worker.

## Fonctions exportées (`src/lib.rs`)

### PostgreSQL

- `create_pg_pool(database_url)` — crée un `PgPool` avec `PG_MAX_CONNECTIONS` (défaut 5) et `PG_ACQUIRE_TIMEOUT` (défaut 30s). Exit le process si la connexion échoue.

### Scheduling

- `spawn_periodic(name, interval_secs, pool, shutdown_rx, api_url, worker_name, job_fn)` — lance une task Tokio qui exécute `job_fn` toutes les `interval_secs`. Break sur shutdown signal. Log les erreurs vers l'API via `send_lifecycle_log`.

### Heartbeat

- `start_heartbeat(api_url, worker_name)` — boucle background qui POST `/api/bots/heartbeat` avec le `worker_name` toutes les 30 s (`HEARTBEAT_INTERVAL`).

### Lifecycle

- `send_lifecycle_log(api_url, worker_name, level, message)` — envoie un log typé (`info` / `warn` / `error`) vers `/api/logs`.
- `shutdown_signal()` — attend Ctrl+C ou SIGTERM de manière cross-platform.

### Config hiérarchique

- `load_worker_config(pool, worker_name)` — charge la config depuis `bot_guild_config WHERE bot_name=worker_name` et retourne un `HashMap<String, String>`.
- `is_worker_enabled(pool, guild_id, worker_name)` — vérifie si le worker est activé pour une guild (via `bot_guild_config.enabled`).
- `load_env(key, default)` / `load_env_bool(key, default)` — helpers pour lire les env vars avec typage.
- `config_or_env(db_config, db_key, env_key, default)` / `config_or_env_bool(...)` — hiérarchie **DB > env > défaut** unifiée.
- `load_database_url()` / `load_redis_url()` / `load_api_url()` — helpers prédéfinis.

### Constantes

- `SECS_PER_MINUTE = 60`, `SECS_PER_HOUR = 3600`, `DEFAULT_PG_MAX_CONNECTIONS = 5`, etc.

### Observabilité (`src/metrics.rs`)

- `init_observability(worker_name)` — one-liner qui :
  1. Initialise le registry Prometheus
  2. Spawn un serveur HTTP sur `:9100` exposant `/metrics`
  3. Démarre un sampler `tokio-metrics` qui publie toutes les 10s des gauges sur l'état du runtime Tokio (`tokio_busy_ratio`, `tokio_live_tasks_count`, `tokio_global_queue_depth`, etc. — uniquement les champs **stables** de tokio-metrics, pas besoin de `tokio_unstable`)

## Dépendances externes

- PostgreSQL (config + heartbeat logs)
- Redis (optionnel, chaque worker décide)
- API interne (heartbeat + lifecycle logs)
- Prometheus (exposition via `:9100`)

## Modules clés

- `src/lib.rs` — toutes les fonctions ci-dessus
- `src/metrics.rs` — observabilité Prometheus + sampler tokio

## Variables d'env (partagées par tous les workers)

- `DATABASE_URL` (requis)
- `REDIS_URL` (défaut `redis://localhost:6379`)
- `API_URL` (défaut `http://localhost:3000`)
- `PG_MAX_CONNECTIONS` (défaut 5)
- `PG_ACQUIRE_TIMEOUT` (défaut 30s)
- `HEARTBEAT_INTERVAL` (défaut 30s)

## Tables DB touchées

- `bot_guild_config` (SELECT pour config dynamique)
- `logs` (INSERT via `/api/logs` pour les lifecycle events)
- `bot_heartbeats` ou équivalent (via `/api/bots/heartbeat`)
