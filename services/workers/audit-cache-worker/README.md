# audit-cache-worker

Refresh du cache `watched_users` pour le module audit du bot. Multi-tenant
scaling : centralise la lecture en DB pour eviter que chaque replica audit-bot
fasse ses propres queries.

## Role

1. Scan periodique de `watched_users` (filtre actifs)
2. Aggrege par guild + user
3. Publie le snapshot dans `audit_cache:watched:{guild_id}` (Redis, TTL 5 min)
4. Le bot lit ce cache au lieu de faire des `SELECT` directs

Permet de scaler horizontalement le bot sans pression DB.

## Variables d'environnement

| Var | Defaut | Role |
|---|---|---|
| `DATABASE_URL` | (requis) | Connexion Postgres |
| `REDIS_URL` | (requis) | Cache des snapshots |
| `API_URL` | (requis) | URL de l'API (heartbeat) |
| `API_KEY` | (requis) | Bearer token API |
| `AUDIT_CACHE_REFRESH_INTERVAL` | 60 | Interval de refresh (secondes) |
| `METRICS_PORT` | 9100 | Port d'expose Prometheus |

## Tables touchees

- `watched_users` (lecture)

## Cles Redis

- `audit_cache:watched:{guild_id}` (TTL 5 min)

## Healthcheck

Endpoint `GET /metrics` sur `:9100`.
