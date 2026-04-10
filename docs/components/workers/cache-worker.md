# cache-worker

**Rôle** : Worker multi-usage qui rafraîchit les caches Redis (analytics, dashboard, voice stats), refresh les vues matérialisées, synchronise `user_cache` et crée les partitions mensuelles futures.

## Jobs périodiques (6 jobs)

| Job | Intervalle défaut | Fichier | Phase |
|---|---|---|---|
| `warm_analytics` | 5 min (`ANALYTICS_CACHE_REFRESH`) | `src/jobs/warm_analytics.rs` | baseline |
| `warm_dashboard` | 10 min (`DASHBOARD_CACHE_REFRESH`) | `src/jobs/warm_dashboard.rs` | baseline |
| `warm_voice_stats` | 1h (`VOICE_STATS_CACHE_REFRESH`) | `src/jobs/warm_voice_stats.rs` | baseline |
| `refresh_leaderboards` | 5 min (`LEADERBOARDS_REFRESH`) | `src/jobs/refresh_leaderboards.rs` | 2 A.2 |
| `sync_user_cache` | 15 min (`USER_CACHE_SYNC`) | `src/jobs/sync_user_cache.rs` | 2 A.2 |
| `manage_partitions` | 24h (`PARTITION_MANAGER`) | `src/jobs/manage_partitions.rs` | 2 A.4 |

### Détails jobs Phase 2

- **refresh_leaderboards** — lance `REFRESH MATERIALIZED VIEW CONCURRENTLY` sur `mv_coude_leaderboard`, `mv_wallet_leaderboard`, `mv_level_leaderboard`. Concurrent = les lectures continuent pendant le refresh.
- **sync_user_cache** — `INSERT INTO user_cache ... SELECT DISTINCT ON (guild_id, user_id) ... FROM (UNION ALL 4 tables hot) ... ON CONFLICT DO UPDATE ... WHERE updated_at < EXCLUDED.updated_at`. Maintient la source de vérité des usernames Discord.
- **manage_partitions** — pour chaque table partitionnée (`infractions`, `audit_logs`, `user_activity_log`, `logs`), vérifie que les partitions M+1 et M+2 existent et les crée sinon. Idempotent.

## Dépendances externes

- PostgreSQL
- Redis (warm_* jobs écrivent des clés avec TTL)

## Modules clés

- `src/main.rs` — startup avec Redis
- `src/config.rs` — 6 intervalles configurables
- `src/scheduler.rs` — enregistrement des 6 jobs
- `src/jobs/*.rs` — 6 fichiers, un par job

## Variables d'env

- `DATABASE_URL` / `REDIS_URL` / `API_URL`
- `ANALYTICS_CACHE_REFRESH` / `DASHBOARD_CACHE_REFRESH` / `VOICE_STATS_CACHE_REFRESH`
- `LEADERBOARDS_REFRESH` / `USER_CACHE_SYNC` / `PARTITION_MANAGER`

## Tables DB

- Lecture : `infractions`, `coude_players`, `user_wallets`, `user_levels`, `user_stats`
- Écriture : `user_cache` (UPSERT), vues matérialisées (REFRESH), DDL (CREATE TABLE partition)
- Redis : clés `analytics:*`, `dashboard:*`, `voice_stats:*` avec TTL
