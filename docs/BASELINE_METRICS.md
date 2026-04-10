# Baseline metrics — Phase 0

> Document de référence à **remplir une fois** au démarrage de l'observabilité,
> puis à **comparer** après chaque phase de la roadmap pour valider les gains.

## Comment générer cette baseline

```bash
# 1. Démarrer la stack monitoring (Prometheus + Grafana)
docker compose --profile monitoring up -d prometheus grafana

# 2. Démarrer l'API + workers
docker compose up -d

# 3. Laisser tourner ~30 min sous charge réelle (ou simuler avec un script de stress)

# 4. Ouvrir Grafana
#    URL    : http://localhost:3002
#    Login  : admin / admin (ou GRAFANA_USER / GRAFANA_PASSWORD)
#    Dashboard : "DiscordSentinel — Baseline (Phase 0)"

# 5. Ouvrir Prometheus pour les requêtes ad-hoc
#    URL : http://localhost:9090

# 6. Reset des stats Postgres (avant la phase à mesurer)
psql -h localhost -U sentinel -d discord_sentinel -c "SELECT pg_stat_statements_reset();"
```

---

## 📊 Métriques applicatives (Prometheus)

### Latences API (p50 / p95 / p99)

| Endpoint | p50 (ms) | p95 (ms) | p99 (ms) | Volume req/s | Notes |
|---|---|---|---|---|---|
| `POST /analyze` (texte) | _à remplir_ | _à remplir_ | _à remplir_ | _à remplir_ | Endpoint le plus lourd (inférence ONNX) |
| `POST /analyze/image` | _à remplir_ | _à remplir_ | _à remplir_ | _à remplir_ | Encore plus lourd, base64 + vision |
| `GET /api/coude/{guild_id}/players/{user_id}` | _à remplir_ | _à remplir_ | _à remplir_ | _à remplir_ | Lecture player, hot path |
| `POST /api/coude/{guild_id}/combats/create` | _à remplir_ | _à remplir_ | _à remplir_ | _à remplir_ | Mutation combat |
| `POST /api/coude/combats/{combat_id}/resolve` | _à remplir_ | _à remplir_ | _à remplir_ | _à remplir_ | Avec garde anti-race |
| `GET /api/coude/{guild_id}/leaderboard/{cat}` | _à remplir_ | _à remplir_ | _à remplir_ | _à remplir_ | Cible Phase 2A.2 (vue matérialisée) |
| `GET /api/wallet/{guild_id}/leaderboard` | _à remplir_ | _à remplir_ | _à remplir_ | _à remplir_ | Idem |
| `GET /api/dashboard/stats` | _à remplir_ | _à remplir_ | _à remplir_ | _à remplir_ | Stats agrégées |
| `POST /api/coude/{guild_id}/bets` | _à remplir_ | _à remplir_ | _à remplir_ | _à remplir_ | Transaction multi-table |
| `GET /api/audit-logs` | _à remplir_ | _à remplir_ | _à remplir_ | _à remplir_ | Cible Phase 2A.4 (partitionnement) |

**Requête PromQL pour récupérer p95 par route :**
```promql
histogram_quantile(0.95, sum by (route, le) (rate(http_request_duration_seconds_bucket[5m])))
```

### Taux d'erreurs HTTP

| Status | req/s actuel | Notes |
|---|---|---|
| 2xx | _à remplir_ | Doit être ~100% |
| 4xx | _à remplir_ | Erreurs client (validation, auth) |
| 5xx | _à remplir_ | **Doit être ~0** — chaque 5xx = bug à investiguer |

---

## ⚙️ Métriques runtime tokio

| Service | `tokio_busy_ratio` | `tokio_live_tasks_count` | `tokio_workers_count` | Alerte si |
|---|---|---|---|---|
| `sentinel-api` | _à remplir_ | _à remplir_ | _à remplir_ | busy_ratio > 0.7 sur 5 min |
| `moderation-worker` | _à remplir_ | _à remplir_ | _à remplir_ | idem |
| `analytics-worker` | _à remplir_ | _à remplir_ | _à remplir_ | idem |
| `cache-worker` | _à remplir_ | _à remplir_ | _à remplir_ | idem |
| `cleanup-worker` | _à remplir_ | _à remplir_ | _à remplir_ | idem |
| `coude-worker` | _à remplir_ | _à remplir_ | _à remplir_ | idem |
| `monitoring-worker` | _à remplir_ | _à remplir_ | _à remplir_ | idem |

---

## 🐘 Métriques PostgreSQL (`pg_stat_statements`)

### Top 10 des queries les plus coûteuses (par `total_exec_time`)

```sql
SELECT
    LEFT(query, 80) AS query_excerpt,
    calls,
    ROUND(total_exec_time::numeric / 1000, 2) AS total_secs,
    ROUND(mean_exec_time::numeric, 2) AS mean_ms,
    ROUND(max_exec_time::numeric, 2) AS max_ms,
    rows
FROM pg_stat_statements
WHERE query NOT LIKE '%pg_stat_statements%'
  AND query NOT LIKE 'COMMIT'
  AND query NOT LIKE 'BEGIN%'
ORDER BY total_exec_time DESC
LIMIT 10;
```

| # | Query (extrait) | calls | total (s) | mean (ms) | max (ms) | rows |
|---|---|---|---|---|---|---|
| 1 | _à remplir_ | _à remplir_ | _à remplir_ | _à remplir_ | _à remplir_ | _à remplir_ |
| 2 | _à remplir_ | _à remplir_ | _à remplir_ | _à remplir_ | _à remplir_ | _à remplir_ |
| 3 | _à remplir_ | _à remplir_ | _à remplir_ | _à remplir_ | _à remplir_ | _à remplir_ |
| 4 | _à remplir_ | _à remplir_ | _à remplir_ | _à remplir_ | _à remplir_ | _à remplir_ |
| 5 | _à remplir_ | _à remplir_ | _à remplir_ | _à remplir_ | _à remplir_ | _à remplir_ |
| 6 | _à remplir_ | _à remplir_ | _à remplir_ | _à remplir_ | _à remplir_ | _à remplir_ |
| 7 | _à remplir_ | _à remplir_ | _à remplir_ | _à remplir_ | _à remplir_ | _à remplir_ |
| 8 | _à remplir_ | _à remplir_ | _à remplir_ | _à remplir_ | _à remplir_ | _à remplir_ |
| 9 | _à remplir_ | _à remplir_ | _à remplir_ | _à remplir_ | _à remplir_ | _à remplir_ |
| 10 | _à remplir_ | _à remplir_ | _à remplir_ | _à remplir_ | _à remplir_ | _à remplir_ |

### Top 10 des index inutilisés

```sql
SELECT
    schemaname || '.' || relname AS table,
    indexrelname AS index,
    pg_size_pretty(pg_relation_size(indexrelid)) AS size,
    idx_scan AS scans
FROM pg_stat_user_indexes
WHERE idx_scan < 10
ORDER BY pg_relation_size(indexrelid) DESC
LIMIT 10;
```

| Table | Index | Taille | Scans | Action |
|---|---|---|---|---|
| _à remplir_ | _à remplir_ | _à remplir_ | _à remplir_ | DROP (Phase 2A.1) |

### Tailles des plus grosses tables

```sql
SELECT
    schemaname || '.' || tablename AS table,
    pg_size_pretty(pg_total_relation_size(schemaname||'.'||tablename)) AS total_size,
    pg_size_pretty(pg_relation_size(schemaname||'.'||tablename)) AS data_size,
    pg_size_pretty(pg_total_relation_size(schemaname||'.'||tablename) - pg_relation_size(schemaname||'.'||tablename)) AS index_size
FROM pg_tables
WHERE schemaname = 'public'
ORDER BY pg_total_relation_size(schemaname||'.'||tablename) DESC
LIMIT 15;
```

| Table | Total | Data | Index |
|---|---|---|---|
| _à remplir_ | _à remplir_ | _à remplir_ | _à remplir_ |

### Connexions actives

```sql
SELECT state, COUNT(*) FROM pg_stat_activity GROUP BY state;
```

| État | Nombre | Notes |
|---|---|---|
| `active` | _à remplir_ | Doit rester < `max_connections * 0.7` |
| `idle` | _à remplir_ | Indicateur de pool oversized si très élevé |
| `idle in transaction` | _à remplir_ | **Doit être 0** — sinon = lock leak |

---

## 💾 Ressources système (par container Docker)

| Service | RAM (MB) | CPU (% moyen) | CPU (% pic) | Notes |
|---|---|---|---|---|
| `sentinel-api` | _à remplir_ | _à remplir_ | _à remplir_ | Cible Phase 1 : -30% RAM (jemalloc + LTO + cache Serenity) |
| `sentinel-postgres` | _à remplir_ | _à remplir_ | _à remplir_ | |
| `sentinel-redis` | _à remplir_ | _à remplir_ | _à remplir_ | |
| `sentinel-gateway` | _à remplir_ | _à remplir_ | _à remplir_ | |
| `sentinel-moderation-worker` | _à remplir_ | _à remplir_ | _à remplir_ | |
| `sentinel-analytics-worker` | _à remplir_ | _à remplir_ | _à remplir_ | |
| `sentinel-cache-worker` | _à remplir_ | _à remplir_ | _à remplir_ | |
| `sentinel-cleanup-worker` | _à remplir_ | _à remplir_ | _à remplir_ | |
| `sentinel-coude-worker` | _à remplir_ | _à remplir_ | _à remplir_ | |
| `sentinel-monitoring-worker` | _à remplir_ | _à remplir_ | _à remplir_ | |
| `sentinel-automod-bot` | _à remplir_ | _à remplir_ | _à remplir_ | |
| `sentinel-moderation-bot` | _à remplir_ | _à remplir_ | _à remplir_ | |
| `sentinel-coude-bot` | _à remplir_ | _à remplir_ | _à remplir_ | |
| `sentinel-voice-bot` | _à remplir_ | _à remplir_ | _à remplir_ | |
| _autres bots…_ | _à remplir_ | _à remplir_ | _à remplir_ | |

**RAM totale (somme)** : _à remplir_ MB
**CPU total moyen** : _à remplir_ %

Commande utile :
```bash
docker stats --no-stream --format "table {{.Name}}\t{{.MemUsage}}\t{{.CPUPerc}}"
```

---

## 📈 Comparaison après chaque phase

| Phase | Date | Latence p95 / endpoint hot | RAM totale | Top query (mean ms) | Notes |
|---|---|---|---|---|---|
| **Phase 0 (baseline)** | _à remplir_ | _à remplir_ | _à remplir_ | _à remplir_ | Baseline initiale |
| Phase 1 (quick wins) | _à remplir_ | _à remplir_ | _à remplir_ | _à remplir_ | Attendu : -30% RAM, -50% latence calls API internes |
| Phase 2A.1 (DB quick wins) | _à remplir_ | _à remplir_ | _à remplir_ | _à remplir_ | Attendu : -25% taille index, +10x JSONB |
| Phase 2A.2 (vues matérialisées) | _à remplir_ | _à remplir_ | _à remplir_ | _à remplir_ | Attendu : 100-1000x sur leaderboards |
| Phase 2A.4 (partitionnement) | _à remplir_ | _à remplir_ | _à remplir_ | _à remplir_ | Attendu : 10-100x sur queries temporelles |
| Phase 4 (ai-worker) | _à remplir_ | _à remplir_ | _à remplir_ | _à remplir_ | Attendu : `/analyze` p95 < 100ms (était 5-60s) |
| Phase 5 (cache + streams) | _à remplir_ | _à remplir_ | _à remplir_ | _à remplir_ | Attendu : -70% charge DB sur endpoints hot |

---

## 🔧 Workflow de validation après chaque phase

1. **Reset stats** : `SELECT pg_stat_statements_reset();`
2. **Déployer** la phase
3. **Laisser tourner** sous charge identique au baseline (~30 min minimum)
4. **Recopier** les chiffres dans la ligne correspondante du tableau ci-dessus
5. **Comparer** au baseline initial — si pas d'amélioration mesurable, **investiguer avant de passer à la phase suivante**
6. **Commit** le `BASELINE_METRICS.md` mis à jour pour traçabilité historique
