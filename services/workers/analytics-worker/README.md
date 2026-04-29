# analytics-worker

Snapshots periodiques des stats serveurs pour les graphes du dashboard.

## Role

- **Snapshot quotidien** : aggrege les stats du jour (messages, infractions,
  activites par heure) dans `analytics_daily_snapshots`. Une seule ligne par
  guild par jour.
- **Snapshot horaire** : capture instantannee toutes les heures pour les
  graphes "live". Garde 7 jours (rotation par cleanup-worker).

## Variables d'environnement

| Var | Defaut | Role |
|---|---|---|
| `DATABASE_URL` | (requis) | Connexion Postgres |
| `API_URL` | (requis) | URL de l'API |
| `API_KEY` | (requis) | Bearer token API |
| `DAILY_SNAPSHOT_INTERVAL` | 24 | Interval daily (heures) |
| `HOURLY_SNAPSHOT_INTERVAL` | 60 | Interval hourly (minutes) |
| `METRICS_PORT` | 9100 | Port d'expose Prometheus |

## Tables touchees

- `analytics_daily_snapshots` (ecriture)
- `analytics_hourly_snapshots` (ecriture)
- `infractions`, `audit_logs`, `user_activity_log` (lecture)

## Healthcheck

Endpoint `GET /metrics` sur `:9100`.
