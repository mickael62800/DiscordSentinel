# analytics-worker

**Rôle** : Enregistre des snapshots d'activité quotidienne et horaire (messages, voix, membres actifs, infractions) pour chaque guild, alimentant les dashboards.

## Jobs périodiques

| Job | Intervalle défaut | Fichier |
|---|---|---|
| `daily_snapshot` | 1h (`DAILY_SNAPSHOT_INTERVAL`) | `src/jobs/daily_snapshot.rs` |
| `hourly_snapshot` | 1h (`HOURLY_SNAPSHOT_INTERVAL`) | `src/jobs/hourly_snapshot.rs` |

## Dépendances externes

- PostgreSQL
- API interne (heartbeat + lifecycle logs)

## Modules clés

- `src/main.rs` — startup
- `src/config.rs` — intervalles des snapshots
- `src/scheduler.rs` — enregistre les 2 jobs
- `src/jobs/daily_snapshot.rs` — UPSERT dans `daily_activity`
- `src/jobs/hourly_snapshot.rs` — UPSERT dans `hourly_activity`

## Variables d'env

- `DATABASE_URL` / `API_URL`
- `DAILY_SNAPSHOT_INTERVAL`
- `HOURLY_SNAPSHOT_INTERVAL`

## Tables DB

- `daily_activity` (UPSERT) — clé `(guild_id, day)`
- `hourly_activity` (UPSERT) — clé `(guild_id, day, hour)`
- Lecture : `user_stats`, `audit_logs`, `infractions` pour calculer les agrégats
