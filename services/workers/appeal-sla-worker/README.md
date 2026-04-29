# appeal-sla-worker

Escalade SLA des tickets d'appel de sanction. Si un appel reste sans
reponse au-dela d'un seuil, alerte la moderation via WS broadcast.

## Role

1. Scan `appeal_tickets` (status='open' + age > SLA threshold)
2. Pour chaque ticket "stale" :
   - Met a jour le `severity_level` (1 -> 2 -> 3)
   - Publie un event WebSocket `appeal_sla_breach` via Redis pub/sub
   - Logge dans `audit_logs`

Permet aux moderateurs/admins d'etre notifies des appels oublies.

## Variables d'environnement

| Var | Defaut | Role |
|---|---|---|
| `DATABASE_URL` | (requis) | Connexion Postgres |
| `REDIS_URL` | (requis) | Pub/sub WS events |
| `API_URL` | (requis) | URL de l'API |
| `API_KEY` | (requis) | Bearer token API |
| `APPEAL_SLA_SCAN_INTERVAL` | 120 | Interval de scan (secondes) |
| `METRICS_PORT` | 9100 | Port d'expose Prometheus |

## Tables touchees

- `appeal_tickets` (lecture/ecriture)
- `audit_logs` (ecriture)

## Healthcheck

Endpoint `GET /metrics` sur `:9100`.
