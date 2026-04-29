# monitoring-worker

Detection offline/online des bots et workers via Redis heartbeats. Publie des
events WS quand un service tombe ou revient.

## Role

1. Toutes les `MONITOR_CHECK_INTERVAL` secondes (defaut 30s)
2. Lit les cles Redis `heartbeat:{worker_name}` (TTL 90s)
3. Pour chaque service connu :
   - Si heartbeat absent / expire et derniere fois online -> publie
     `service_offline` event WS
   - Si heartbeat present et derniere fois offline -> publie `service_online`
4. Met a jour le cache d'etat `monitor_state:{service}` Redis

Cible Prometheus : ce worker n'a pas de DB, juste Redis. Les bots et workers
ecrivent leur heartbeat via `worker-common::start_heartbeat`.

## Variables d'environnement

| Var | Defaut | Role |
|---|---|---|
| `REDIS_URL` | (requis) | Heartbeats + pub/sub |
| `API_URL` | (requis) | URL de l'API (lifecycle log) |
| `API_KEY` | (requis) | Bearer token API |
| `MONITOR_CHECK_INTERVAL` | 30 | Interval check (secondes) |
| `METRICS_PORT` | 9100 | Port d'expose Prometheus |

## Cles Redis

- `heartbeat:{service_name}` (lecture, TTL 90s)
- `monitor_state:{service_name}` (lecture/ecriture, etat current)
- Pub/sub `sentinel:events` (publication)

## Healthcheck

Endpoint `GET /metrics` sur `:9100`.
