# cleanup-worker

**Rôle** : Supprime les anciennes données selon les périodes de rétention configurées (voice_sessions, logs, closed tickets, user_activity_log) et exécute `VACUUM ANALYZE` optionnel pour récupérer l'espace disque.

## Jobs périodiques

| Job | Intervalle défaut | Fichier |
|---|---|---|
| `cleanup_old_data` | 1h (`CLEANUP_INTERVAL_HOURS`) | `src/jobs/cleanup_old_data.rs` |
| `vacuum_tables` | 24h (`VACUUM_INTERVAL_HOURS`, si `VACUUM_ENABLED=true`) | `src/jobs/vacuum_tables.rs` |

## Rétentions (variables d'env)

| Table | Défaut | Env var |
|---|---|---|
| `voice_sessions` | 90 jours | `VOICE_SESSIONS_RETENTION_DAYS` |
| `logs` | 30 jours | `LOGS_RETENTION_DAYS` |
| Tickets fermés | 180 jours | `CLOSED_TICKETS_RETENTION_DAYS` |
| `user_activity_log` | (via défaut cleanup) | — |
| `audit_logs` | (via défaut cleanup) | — |

## Dépendances externes

- PostgreSQL uniquement

## Modules clés

- `src/main.rs` — startup
- `src/config.rs` — intervalles et rétentions
- `src/scheduler.rs` — enregistre les 2 jobs
- `src/jobs/cleanup_old_data.rs` — `DELETE FROM ... WHERE created_at < NOW() - INTERVAL ...`
- `src/jobs/vacuum_tables.rs` — `VACUUM (ANALYZE) <table>`

## Variables d'env

- `DATABASE_URL` / `API_URL`
- `CLEANUP_INTERVAL_HOURS`
- `VACUUM_ENABLED` / `VACUUM_INTERVAL_HOURS`
- `VOICE_SESSIONS_RETENTION_DAYS` / `LOGS_RETENTION_DAYS` / `CLOSED_TICKETS_RETENTION_DAYS`

## Note Phase 2 A.4

Les tables partitionnées (`infractions`, `audit_logs`, `user_activity_log`, `logs`) pourront **à terme** utiliser `DROP PARTITION` au lieu de `DELETE`, ce qui est O(1) vs O(N). Non implémenté à date — le cleanup utilise encore `DELETE`. À envisager en Phase 5 ou 6.
