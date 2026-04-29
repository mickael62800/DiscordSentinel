# cache-worker

Worker multi-jobs qui maintient des caches pour acceler les endpoints API et
gere les partitions/MV de la DB.

## Role (5 jobs)

1. **analytics_refresh** : refresh des caches Redis pour `/api/analytics/*`
2. **dashboard_refresh** : refresh des KPIs du dashboard
3. **voice_stats_refresh** : agreges voice (top channels, sessions/jour)
4. **leaderboards_refresh** : `REFRESH MATERIALIZED VIEW CONCURRENTLY` sur
   `mv_coude_leaderboard`, `mv_wallet_leaderboard`, `mv_level_leaderboard`
5. **user_cache_sync** : alimente la table `user_cache` (source de verite des
   usernames Discord) en aggregeant les 4 tables hot
6. **manage_partitions** : cree les partitions M+1 et M+2 sur les tables
   partitionnees (infractions, audit_logs, user_activity_log, logs)

## Variables d'environnement

| Var | Defaut | Role |
|---|---|---|
| `DATABASE_URL` | (requis) | Connexion Postgres |
| `REDIS_URL` | (requis) | Cache de destination |
| `API_URL` | (requis) | URL de l'API |
| `API_KEY` | (requis) | Bearer token API |
| `ANALYTICS_CACHE_REFRESH` | 300 | Interval analytics (secondes) |
| `DASHBOARD_CACHE_REFRESH` | 600 | Interval dashboard (secondes) |
| `VOICE_STATS_CACHE_REFRESH` | 3600 | Interval voice stats (secondes) |
| `LEADERBOARDS_REFRESH` | 300 | Interval leaderboards (secondes) |
| `USER_CACHE_SYNC` | 600 | Interval user_cache (secondes) |
| `METRICS_PORT` | 9100 | Port d'expose Prometheus |

## Tables touchees

- Lecture : toutes les tables hot (infractions, audit_logs, user_activity_log,
  logs, voice_sessions, etc.)
- Ecriture : `user_cache`, `mv_*` (refresh)

## Healthcheck

Endpoint `GET /metrics` sur `:9100`.
