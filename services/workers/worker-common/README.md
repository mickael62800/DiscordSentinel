# worker-common

Bibliotheque partagee par tous les workers DiscordSentinel. Factorise la
plomberie commune (config env, pg_pool, heartbeat, lifecycle, scheduler,
metrics, helpers Redis/HTTP/gRPC).

## Modules

- `api` : appels HTTP vers l'API (envoi de logs lifecycle, etc.)
- `grpc` : creation de clients gRPC partages
- `metrics` : init `metrics-exporter-prometheus` + tokio-metrics, expose
  `/metrics` sur `METRICS_PORT` (defaut 9100)
- `redis_helpers` : `open_or_exit`, helpers de pub/sub

## Fonctions cles

| Fonction | Role |
|---|---|
| `init_tracing(filter)` | dotenv + tracing-subscriber JSON |
| `create_pg_pool(url)` | sqlx::PgPool avec params PG_MAX_CONNECTIONS / PG_ACQUIRE_TIMEOUT |
| `send_lifecycle_log(api_url, name, level, msg)` | POST `/api/logs` |
| `start_heartbeat(api_url, name)` | task ecrit Redis `heartbeat:{name}` toutes les 30s |
| `shutdown_signal()` | attend Ctrl+C / SIGTERM |
| `run_lifecycle(name, display, api_url, pool, schedule_fn)` | boilerplate de cycle de vie (channels, heartbeat, lifecycle logs, shutdown) — factorise les ~15 lignes 100% identiques entre workers |
| `is_worker_enabled(pool, guild_id, name)` | check `bot_guild_config` (per-guild on/off) |
| `is_worker_globally_enabled(pool, name)` | check `worker_global_config` (kill switch) |
| `load_database_url() / load_api_url() / load_redis_url()` | lecture env avec exit en cas d'absence |
| `load_worker_config(pool, name)` | charge la config DB du worker |
| `config_or_env(...)` | fallback DB -> env -> default |
| `load_env<T>(key, default)` | parse env var avec defaut |
| `spawn_periodic(name, interval, pool, shutdown, api_url, worker_name, fn)` | spawn une tache periodique avec metrics + heartbeat |

## Convention d'appel

Chaque worker appelle dans son `main.rs` :

```rust
use sentinel_worker_common as common;

#[tokio::main]
async fn main() {
    common::init_tracing("sentinel_X_worker=info");
    common::metrics::init_observability("X-worker");

    let mut config = WorkerConfig::from_env();
    let pg_pool = common::create_pg_pool(&config.database_url).await;
    let db_config = common::load_worker_config(&pg_pool, "X-worker").await;
    if !db_config.is_empty() {
        config.apply_db_config(&db_config);
    }

    common::run_lifecycle(
        "X-worker",
        "X Worker",
        &config.api_url,
        &pg_pool,
        |shutdown_rx| scheduler::start(&config, pg_pool.clone(), shutdown_rx),
    ).await;
}
```
