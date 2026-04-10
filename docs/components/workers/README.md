# Workers — Index

8 workers Tokio périodiques + 1 librairie partagée (`worker-common`).

| Worker | Rôle | Phase |
|---|---|---|
| [ai-worker](./ai-worker.md) | Dépile `ai_jobs`, dispatch vers l'inférence API, publie sur Redis | 4 |
| [analytics-worker](./analytics-worker.md) | Snapshots quotidiens/horaires (messages, voix, membres) | baseline |
| [cache-worker](./cache-worker.md) | 6 jobs : warm caches, refresh MV, sync user_cache, partitions | 2 + 4 |
| [cleanup-worker](./cleanup-worker.md) | Purge data rétention + VACUUM optionnel | baseline |
| [coude-worker](./coude-worker.md) | Expiration combats + résolution paris (jeu Coude) | baseline |
| [moderation-worker](./moderation-worker.md) | Regen conduct + cleanup bans + rappels sanctions (Redis) | baseline + 4 |
| [monitoring-worker](./monitoring-worker.md) | Détecte offline/online des bots/workers via Redis | baseline |
| [temp-roles-worker](./temp-roles-worker.md) | Scanne temp_roles expirés → publie events Redis | 4 |
| [worker-common](./worker-common.md) | **Librairie commune** (pg pool, spawn_periodic, observability) | — |

## Pattern commun

Tous les workers (sauf `monitoring-worker` qui boucle manuellement) suivent le même modèle :

```rust
// main.rs
common::init_tracing();
common::metrics::init_observability(WORKER_NAME);
let pool = common::create_pg_pool(&config.database_url).await;
let (shutdown_tx, shutdown_rx) = watch::channel(false);
scheduler::start(&config, pool, [redis], shutdown_rx);
common::start_heartbeat(config.api_url, WORKER_NAME);
common::shutdown_signal().await;
```

Chaque job périodique est enregistré via `spawn_periodic(name, interval, pool, shutdown, api_url, worker_name, closure)`. Le helper gère :
- Le tick `tokio::time::sleep` avec break sur shutdown
- Le logging structuré (`tracing`)
- Le report automatique des erreurs vers l'API (log table)

## Variables d'env communes

Tous les workers héritent de `worker-common` les variables :
- `DATABASE_URL` (requis)
- `REDIS_URL` (optionnel selon worker)
- `API_URL` (heartbeat + log lifecycle)
- `PG_MAX_CONNECTIONS` (défaut 5)
- `PG_ACQUIRE_TIMEOUT` (défaut 30s)
- `HEARTBEAT_INTERVAL` (défaut 30s)
