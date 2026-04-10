# gateway — WebSocket relay

**Rôle** : Proxy WebSocket stateless. Relaie les événements publiés sur Redis pub/sub vers les clients desktop connectés en temps réel. Permet d'afficher les events live dans le desktop sans polling.

## Architecture

**Pattern Broadcaster** (channel Tokio capacity-limited). Stack : **Rust / Axum WebSocket / Redis client / `tokio::sync::broadcast`**.

## Structure du code

```
services/gateway/
├── Cargo.toml
└── src/
    ├── main.rs              (setup, CORS, routes /ws et /health)
    ├── broadcaster.rs       (EventBroadcaster : atomic count, broadcast channel)
    ├── handler.rs           (ws_handler, relais Redis → WebSocket par client)
    ├── redis_subscriber.rs  (connexion Redis SUBSCRIBE + reconnect exponential backoff)
    ├── config.rs            (Config from_env)
    ├── health.rs            (endpoint /health)
    └── logger.rs            (logging structuré)
```

## Endpoints

| Route | Méthode | Description |
|---|---|---|
| `/ws` | GET (upgrade) | WebSocket endpoint (query param `?token=` pour auth) |
| `/health` | GET | Health check |

## Flux de données

```
Bot/Worker ──PUBLISH─► Redis (canal "sentinel:events")
                          │
                          ▼
                  gateway (SUBSCRIBE)
                          │
                          ▼
             EventBroadcaster (broadcast channel)
                          │
            ┌─────────────┼─────────────┐
            ▼             ▼             ▼
       Desktop #1    Desktop #2    Desktop #N  (via WebSocket)
```

## Auth

Le client passe `?token=<API_KEY>` dans l'URL WebSocket. Si `API_KEY` est vide côté serveur, l'auth est bypassée (dev mode). Sinon comparaison string exacte avec la config.

## Dépendances externes

- **Redis** (pub/sub, reconnect avec exponential backoff)
- **Tokio** (runtime async)
- **Axum** (HTTP + WebSocket upgrade)

## Variables d'env

| Variable | Défaut | Rôle |
|---|---|---|
| `HOST` | `0.0.0.0` | Adresse bind |
| `PORT` | 3001 | Port d'écoute |
| `REDIS_URL` | — | Connexion Redis |
| `API_KEY` | (vide) | Token d'auth WebSocket (facultatif en dev) |
| `REDIS_CHANNEL` | `sentinel:events` | Canal Redis à subscribe |
| `MAX_CONNECTIONS` | 1000 | Limite clients simultanés |
| `BROADCAST_CHANNEL_CAPACITY` | 512 | Buffer interne Tokio broadcast |
| `REDIS_RECONNECT_DELAY_SECS` | — | Délai initial reconnect |
| `REDIS_RECONNECT_MAX_DELAY_SECS` | — | Délai max reconnect (exponential) |

## Observabilité

- **TraceLayer** — request_id, method, uri, status, latency
- Logs : connect/disconnect avec IP + total de clients
- Logs : events relayed/skipped (si broadcast channel plein)
- Warn sur auth failure et max connections atteintes
- Graceful shutdown avec timeout configurable
