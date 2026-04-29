# cleanup-worker

Retention DB + VACUUM. Maintient les tables hot a une taille controlee en
supprimant les vieilles lignes selon des SLA configurables.

## Role

### Cleanup (chaque CLEANUP_INTERVAL_HOURS, defaut 1h)

- `voice_sessions` plus vieux que `VOICE_SESSIONS_RETENTION_DAYS` (defaut 90)
- `logs` plus vieux que `LOGS_RETENTION_DAYS` (defaut 30)
- `tickets` fermes plus vieux que `CLOSED_TICKETS_RETENTION_DAYS` (defaut 180)
- `analytics_hourly_snapshots` plus vieux que 7 jours

### VACUUM (chaque VACUUM_INTERVAL_HOURS, defaut 24h, optionnel)

- `VACUUM ANALYZE` sur les tables partitionnees pour reclamer l'espace
  apres les DELETE en masse.

## Variables d'environnement

| Var | Defaut | Role |
|---|---|---|
| `DATABASE_URL` | (requis) | Connexion Postgres |
| `API_URL` | (requis) | URL de l'API |
| `API_KEY` | (requis) | Bearer token API |
| `CLEANUP_INTERVAL_HOURS` | 1 | Interval cleanup (heures) |
| `VACUUM_INTERVAL_HOURS` | 24 | Interval VACUUM (heures) |
| `VACUUM_ENABLED` | true | Active le VACUUM |
| `VOICE_SESSIONS_RETENTION_DAYS` | 90 | Retention voice_sessions |
| `LOGS_RETENTION_DAYS` | 30 | Retention logs |
| `CLOSED_TICKETS_RETENTION_DAYS` | 180 | Retention tickets fermes |
| `METRICS_PORT` | 9100 | Port d'expose Prometheus |

## Tables touchees

- DELETE : `voice_sessions`, `logs`, `tickets`, `analytics_hourly_snapshots`
- VACUUM : tables partitionnees (`infractions`, `audit_logs`, `user_activity_log`, `logs`)

## Healthcheck

Endpoint `GET /metrics` sur `:9100`.
