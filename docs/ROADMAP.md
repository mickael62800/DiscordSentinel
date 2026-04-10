# ROADMAP — DiscordSentinel

Roadmap unifiée consolidant **tous les chantiers** identifiés dans la documentation `docs/` et les améliorations fonctionnelles déjà discutées.

**Objectif** : ordre d'exécution **optimal** qui maximise le ROI et évite de retravailler du code en le modifiant deux fois.

## 📚 Documents sources consolidés

| Document                                                                          | Sujet                                  |
| --------------------------------------------------------------------------------- | -------------------------------------- |
| [`OPTIMISATIONS_PERFORMANCES.md`](./OPTIMISATIONS_PERFORMANCES.md)                | 12 optimisations perf/scalabilité      |
| [`DB_OPTIMISATIONS.md`](./DB_OPTIMISATIONS.md)                                    | 12 optimisations schéma Postgres (index, partitionnement, JSONB, enums) |
| [`WORKERS_PROPOSES.md`](./WORKERS_PROPOSES.md)                                    | 11 workers (dont `ai-worker` critique) |
| ~~`REFACTOR_GOD_FILES.md`~~                                                       | 9 fichiers god à découper (✅ terminé, doc supprimé) |
| [`MULTI_TENANT_AUTH.md`](./MULTI_TENANT_AUTH.md)                                  | Isolation par guild (OAuth2 + RBAC)    |
| [`ESTIMATION_RAM_PROD.md`](./ESTIMATION_RAM_PROD.md)                              | Tuning build + jemalloc                |
| [`bots/moderation-bot/AMELIORATIONS.md`](../bots/moderation-bot/AMELIORATIONS.md) | 8 ajouts fonctionnels moderation-bot   |

---

## 🎯 Principes directeurs

1. **Mesurer avant d'optimiser** — sans baseline, impossible de valider les gains
2. **Sécurité avant features** — multi-tenant avant de partager l'app
3. **Fondations avant refonte** — DB saine avant cache, cache avant gRPC
4. **Refactor avant ajout** — découper les god files avant d'y ajouter des features
5. **Scaling en dernier** — sharding uniquement quand les limites arrivent
6. **Quick wins en parallèle** — jemalloc, compression, pool HTTP = gratuits, à faire tout de suite

---

## 📅 Vue d'ensemble — 7 phases (exécution full IA)

> **Note** : ces durées supposent une implémentation **full IA** (Claude Code / agent de codage) avec validation humaine à chaque phase. Un humain doit tester, valider les métriques et approuver le merge, mais n'écrit pas le code lui-même.

```
Phase 0 : Observabilité                    (2-3h)    ✅ TERMINÉE
   │
   ├──> Phase 1 : Quick wins zéro-risque   (2-3h)    ✅ TERMINÉE
   │
   ▼
Phase 2 : Fondations DB + Sécurité         (2-3j)    ✅ TERMINÉE (partielle)
   │
   ▼
Phase 3 : Refactor god files               (1j)     ✅ TERMINÉE
   │
   ▼
Phase 4 : ai-worker + workers prioritaires (1-2j)    ✅ TERMINÉE (partielle)
   │
   ▼
Phase 5 : Cache + Streams + Batch writes   (2-3j)
   │
   ▼
Phase 6 : Features moderation + workers 2  (1-2j)
   │
   ▼
Phase 7 : gRPC + scaling horizontal        (variable)
```

**Durée totale** : ~**10-14 jours effectifs** en full IA (vs ~3 mois en dev solo humain).

### Pourquoi si rapide en full IA

- Génération de code boilerplate quasi-instantanée (workers, handlers, DTOs, migrations)
- Refactor mécanique des god files : l'IA lit, découpe, réécrit en minutes
- Pas de fatigue cognitive, pas de context-switching
- Parallélisation de tâches indépendantes dans la même session

### Où le temps reste humain

- **Validation des métriques** après chaque phase (Phase 0 baseline, comparaisons)
- **Tests fonctionnels** en prod avec un vrai serveur Discord
- **Approbations** de déploiement (migrations DB, changements infra)
- **Décisions** architecturales ambiguës
- **Debug** des cas non reproductibles en local

---

## Phase 0 — Observabilité & baseline ✅ **TERMINÉE**

**Durée IA** : 2-3h (instrumentation) + validation humaine baseline
**Bloquant pour** : toutes les phases suivantes

Sans métriques, tu optimises à l'aveugle. **À ne jamais sauter.**

### Tâches

- [x] **[OPT #9]** Instrumenter tokio-metrics sur l'API + 6 workers (helper `init_observability` partagé dans `worker-common`, gauges `tokio_busy_ratio`, `tokio_live_tasks_count`, `tokio_global_queue_depth`, etc. — n'utilise que les champs **stables** de `tokio-metrics`, pas besoin de `tokio_unstable`). Bots Discord non instrumentés (low priority — ils sont idle la plupart du temps).
- [x] **[OPT #9]** Tracing structuré avec correlation IDs (X-Trace-ID) — déjà fait par le `TraceLayer.make_span_with` existant qui extrait l'header `x-request-id` et le propage dans le span `http_request`. Le `SetRequestIdLayer` génère un UUID si absent.
- [x] Activer `pg_stat_statements` sur PostgreSQL
  - `docker-compose.yml` : ajout de `shared_preload_libraries=pg_stat_statements` au commandline du conteneur
  - Migration `099_enable_pg_stat_statements.sql` : `CREATE EXTENSION IF NOT EXISTS pg_stat_statements`
- [x] **Métriques HTTP API** : middleware Axum qui enregistre `http_requests_total{route, method, status}` (counter) et `http_request_duration_seconds{...}` (histogram) — utilise `MatchedPath` pour borner la cardinalité (les routes paramétrées partagent un seul label, ex : `/api/coude/{guild_id}/players/{user_id}`).
- [x] Endpoint `/metrics` Prometheus exposé sur l'API (route publique, pas d'auth — restreindre par firewall en prod)
- [x] Stack Grafana + Prometheus dans `docker-compose.yml` (profil `monitoring`)
  - `infra/prometheus/prometheus.yml` : scrape l'API + les 6 workers (port 9100)
  - `infra/grafana/provisioning/datasources/prometheus.yml` : datasource auto-provisionnée
  - `infra/grafana/provisioning/dashboards/dashboards.yml` : dashboards auto-importés
  - `infra/grafana/dashboards/sentinel-baseline.json` : dashboard avec 5 panels (req/s par route, latence p50/p95/p99, busy_ratio, live_tasks, status codes)
- [x] **Baseline documentée** : `docs/BASELINE_METRICS.md` avec template à remplir + requêtes PromQL/SQL toutes faites + tableau de comparaison phase par phase

### Livrable

`docs/BASELINE_METRICS.md` avec :
- Template prêt-à-remplir : latences API, runtime tokio, top queries Postgres, tailles tables, RAM/CPU par container
- Workflow de validation : reset stats → déploiement phase → comparer ligne par ligne
- Requêtes PromQL et SQL toutes prêtes pour ne pas redécouvrir à chaque phase

### Démarrage de la stack

```bash
# Démarrer Prometheus + Grafana (profil monitoring opt-in)
docker compose --profile monitoring up -d prometheus grafana

# Vérifier que les workers exposent /metrics
curl http://localhost:3000/metrics                # API
docker exec sentinel-moderation-worker curl localhost:9100/metrics  # workers

# Ouvrir les UIs
# - Prometheus : http://localhost:9090
# - Grafana    : http://localhost:3002 (admin/admin)
```

---

## Phase 1 — Quick wins zéro-risque ✅ **TERMINÉE**

**Durée IA** : 2-3h (toutes les optims en une session)
**Dépend de** : Phase 0 (pour mesurer l'impact)
**Parallélisable** : oui, toutes les tâches sont indépendantes

Gains massifs, aucune refonte, aucun risque. **À faire tout de suite après la baseline.**

### Tâches

- [x] **[OPT #8][RAM]** jemalloc en allocateur global sur les **23 binaires** Rust (15 bots + 6 workers + API + gateway). Gated `cfg(not(target_env = "msvc"))` pour ne pas casser les builds Windows MSVC en dev → utilisation de l'allocateur système sur Windows, jemalloc partout ailleurs (Linux Docker prod). Gain attendu : **-15 % RAM résidente**.
- [x] **[RAM]** `[profile.release]` ajouté à chaque `Cargo.toml` binaire avec `lto = "fat"`, `codegen-units = 1`, `strip = true`. **`panic = "unwind"` conservé** (défaut) pour isoler les paniques par tâche tokio — `panic = "abort"` aurait économisé ~5 % de binaire mais ferait crasher tout le process si une seule tâche panique.
- [x] **[OPT #4]** Cache Serenity restrictif par bot via helper `bots/shared/src/cache_settings.rs` — 4 presets : `minimal()`, `small()`, `medium()`, `full()`. Distribution :
  - **minimal** (8 bots) : `blackjack`, `cleanup`, `community`, `coude`, `game`, `welcome` (+2 autres) — pas de cache messages, pas de cache channels/users
  - **small** (5 bots) : `automod`, `image`, `progression`, `roles`, `ticket` — cache channels uniquement
  - **medium** (2 bots) : `audit`, `security` — 100 messages/channel pour reconstituer le contexte des suppressions
  - **full** (2 bots) : `moderation`, `voice` — défaut Serenity, nécessaire pour `voice_states` et permissions modérateur
- [x] **[OPT #11]** Compression HTTP : `CompressionLayer::new().zstd(true).gzip(true)` ajoutée au router Axum (features tower-http : `compression-zstd`, `compression-gzip`). Négocie automatiquement via `Accept-Encoding`. Gain attendu : **-60 % bande passante** sur les payloads JSON.
  - ⚠️ **Compression Redis non faite** (reportée) : nécessiterait de modifier toutes les opérations SET/GET pour compresser/décompresser au niveau applicatif. Beaucoup plus invasif que prévu, ROI faible avant Phase 5 (cache-aside). À reconsidérer en Phase 5.
- [x] **[OPT #1]** Pool HTTP keep-alive tuné dans `bots/shared::BaseApiClient` :
  - `pool_max_idle_per_host = 64` (défaut 32 — absorbe les bursts de commandes Discord)
  - `pool_idle_timeout = 300s` (défaut 90s — évite les re-handshakes TLS toutes les 90s)
  - `tcp_keepalive = 60s` (détecte les connexions zombies via NAT idle timeout)
  - Le `Client` reqwest était déjà un singleton par bot (créé une seule fois et stocké dans la TypeMap Serenity) — c'est juste les paramètres du pool qui n'étaient pas tunés. Gain attendu : **-50 à -80 % de latence** sur les appels API internes.

### Livrable

Ces 5 optimisations déployées en prod. Comparer les métriques avec la baseline — le gain doit être visible.

### Validation post-déploiement

Avant de passer en Phase 2, capturer dans `docs/BASELINE_METRICS.md` :
- Le **delta de RAM** de chaque service (`docker stats`) — attendu : -30 à -50 % global
- La **latence p95** des endpoints internes côté bots (avec keep-alive vs sans)
- La **taille moyenne des réponses HTTP** côté bot (avec compression activée)

---

## Phase 2 — Fondations DB + Sécurité multi-tenant ✅ **TERMINÉE (partielle)**

**Durée IA** : 2-3 jours (audit DB complet + 12 optims schéma + middleware auth)
**Dépend de** : Phase 0 (pg_stat_statements)
**Critique avant** : toute future feature partagée à des admins externes

> ✅ **Livré** : migrations 100-104 + middleware multi-tenant + tuning RAM + PgBouncer.
> Le reste (JSONB config_value, NOT NULL constraints, cache moka, partitionnement
> de moderation_actions/security_events/coude_casino_log) est différé — voir
> les sous-sections concernées.

### Récapitulatif livré

- **A.1** (migration `100_phase2_quick_wins.sql` + `101_phase2_discord_ids_varchar20.sql`)
  - DROP de 2 index simples redondants couverts par composites (`idx_audit_logs_guild`, `idx_infractions_guild`)
  - Index partiels soft-delete : `idx_voice_channels_active WHERE channel_status='open'` + `idx_tickets_open WHERE status IN ('open','assigned')`
  - 3 index GIN sur JSONB : `infractions.flags`, `security_events.user_ids`, `bot_definitions.config_schema`
  - DROP `coude_combats.channel_id_temp` (colonne morte)
  - Migration PL/pgSQL idempotente convertit ~30 colonnes Discord ID `TEXT → VARCHAR(20)` via introspection `information_schema`
- **A.2** (migration `102_phase2_materialized_views.sql` + jobs `cache-worker`)
  - 3 vues matérialisées : `mv_coude_leaderboard`, `mv_wallet_leaderboard`, `mv_level_leaderboard` avec rang précalculé + index UNIQUE pour `REFRESH CONCURRENTLY`
  - Table `user_cache(guild_id, user_id, username)` PK composé, source de vérité usernames
  - Job `refresh_leaderboards` (5 min) + job `sync_user_cache` (15 min) ajoutés au cache-worker
  - Repositories `wallet`, `level`, `coude_player` switchés sur les MV (gain 100-1000× sur listing leaderboards)
- **A.3** — partiel (migration `103_phase2_enums_and_permissions_bigint.sql`)
  - 3 enums Postgres créés : `coude_class`, `moderation_gravity`, `voice_channel_kind`
  - 3 nouveaux value_objects Rust : `CoudeClass`, `ModerationGravity`, `VoiceChannelKind` avec `#[derive(sqlx::Type)]`
  - Entities `CoudePlayer`, `ModerationAction`, `VoiceChannel` migrées + repositories adaptés + DTOs convertis aux frontières
  - `discord_roles.permissions` `TEXT → BIGINT` (DTO HTTP reste String pour la safety JS)
  - **⏸️ Différé** : `bot_guild_config.config_value TEXT → JSONB` (23 callsites bots, gain marginal court terme), contraintes `NOT NULL/CHECK` (risque sur données existantes)
- **A.4** (migration `104_phase2_partitioning.sql` + job `manage_partitions`)
  - Partitionnement RANGE mensuel de 4 tables hot : `infractions`, `audit_logs`, `user_activity_log`, `logs`
  - 12 partitions mensuelles 2026-04 → 2027-03 + partition `DEFAULT` pour chacune
  - PK passée à `(id, partition_key)` (transparent pour le code Rust)
  - Job `manage_partitions` (24h) crée automatiquement les partitions M+1 et M+2 (idempotent)
  - **⏸️ Skip** : `moderation_actions`, `security_events` (gain marginal), `daily_activity`/`hourly_activity` (pas de `created_at`), `coude_casino_log` (BIGSERIAL incompatible)
- **A.5** (`docker-compose.yml`)
  - Postgres : `shared_buffers=4GB`, `effective_cache_size=10GB`, `work_mem=64MB`, `maintenance_work_mem=1GB`, `wal_buffers=16MB`, `max_wal_size=4GB`, `checkpoint_completion_target=0.9`
  - Redis : `maxmemory=2gb`, `maxmemory-policy=allkeys-lru`
  - **⏸️ Différé** : étape 9 cache moka in-memory dans l'API (Rust applicatif non-trivial)
- **A.6** (`docker-compose.yml`)
  - PgBouncer (image `edoburu/pgbouncer`) en mode `transaction` : `pool_size=25`, `max_client_conn=1000`, `max_prepared_statements=100` (compat sqlx)
  - Tous les services API + workers repointés sur `pgbouncer:5432`
  - Pools sqlx déjà conformes (API 20, workers 5)
- **B** (middleware + DiscordApiService + desktop adapter)
  - Middleware Axum `guild_auth_middleware` (`services/api/src/adapters/inbound/http/middleware/guild_auth.rs`) : pass-through si `X-Discord-Token` absent (bots/internal), sinon extrait `guild_id` de l'URI, fetch+cache Redis `user_guilds:<hash>` (TTL 5 min), refuse 403 si non autorisé
  - `DiscordApiService::get_user_guilds(access_token)` ajoute l'appel `GET /users/@me/guilds`
  - Wired dans `router.rs` après `auth_middleware`
  - Desktop : `ApiAdapter::set_discord_token`/`clear_discord_token` + `AuthService` propage le token après OAuth2 réussi (lib.rs câble la dépendance)
  - 5 nouveaux tests `guild_auth` + 232 tests existants → **237 tests passent**

---

> 💡 **C'est la phase avec le plus gros ratio gain/effort de tout le projet.** Le partitionnement et les vues matérialisées apportent à eux seuls plus de gains que toutes les optimisations applicatives réunies.

### Partie A — Schéma Postgres sain (voir [`DB_OPTIMISATIONS.md`](./DB_OPTIMISATIONS.md))

> ⚠️ **Ordre critique** : optims non-breaking d'abord, breaking changes ensuite, partitionnement en dernier (le plus complexe). PgBouncer tout à la fin, **après** que les queries soient saines — sinon il amplifie la contention.

#### A.1 — Quick wins zéro-breaking (1-2h IA)

- [ ] **[DB #1]** Audit `pg_stat_statements` → top 20 queries lentes (documenter en baseline)
- [ ] **[DB #1]** Supprimer les index dupliqués (`idx_infractions_guild_created` en 058 et 072, idem audit_logs)
- [ ] **[DB #2]** Index partiels pour soft-delete (`voice_channels` WHERE status='open', tickets WHERE status IN ('open','assigned'))
- [ ] **[DB #3]** Index GIN sur JSONB fréquemment requêté (`infractions.flags`, `security_events.user_ids`, `bot_definitions.config_schema`)
- [ ] **[DB #4]** Migration TEXT → VARCHAR(20) pour tous les Discord IDs (~30 tables) — **zéro impact code**, SQLx mappe pareil
- [ ] **[DB #12]** Audit et suppression des colonnes mortes (`voice_channels.channel_id_temp`, `tickets.channels`)
- [ ] Audit des FK sans index + ajout des index composites manquants (`guild_id + user_id`, `created_at` en range)

**Gain** : -25 à -35 % taille index, +10-50× sur queries JSONB, pas de breaking change.

#### A.2 — Optimisations non-breaking additives (3-4h IA)

- [ ] **[DB #7]** Créer les vues matérialisées leaderboards (`coude_leaderboard`, `user_wallets_leaderboard`, `user_levels_leaderboard`)
- [ ] Ajouter les méthodes `get_leaderboard_from_view()` dans les repositories concernés :
  - `services/api/src/adapters/outbound/postgres/coude_player_repository.rs`
  - `services/api/src/adapters/outbound/postgres/wallet_repository.rs`
  - `services/api/src/adapters/outbound/postgres/level_repository.rs`
  - `services/api/src/adapters/outbound/postgres/stats_repository.rs`
- [ ] Ajouter le job de refresh concurrent (`REFRESH MATERIALIZED VIEW CONCURRENTLY`) au `cache-worker` existant
- [ ] **[DB #10]** Créer la table `user_cache` + nouveau `user-cache-worker` pour sync des usernames Discord

**Gain** : **100-1000×** sur les leaderboards, fin de la dénormalisation stale des usernames.

#### A.3 — Breaking changes contrôlés (5-7h IA)

> ⚠️ Ces changements touchent les types Rust → coordonner déploiement API + bots/workers.

- [ ] **[DB #8]** Enums Postgres (`coude_class`, `moderation_gravity`, `voice_channel_kind`, `infraction_action`)
  - Modifier `services/api/src/domain/entities/coude_player.rs` : `class: Option<String>` → `Option<CoudeClass>`
  - Modifier `services/api/src/domain/entities/moderation_action.rs` : `gravity: Option<String>` → `Option<ModerationGravity>`
  - Modifier `services/api/src/domain/entities/voice_channel.rs` : `kind: String` → `VoiceChannelKind`
  - Adapter les repositories `FromRow` et les DTOs HTTP
- [ ] **[DB #9]** `discord_roles.permissions` TEXT → BIGINT
  - Modifier `services/api/src/domain/entities/discord_role.rs` : `permissions: String` → `permissions: i64`
- [ ] **[DB #5]** `bot_guild_config.config_value` TEXT → JSONB (**le plus breaking**)
  - Modifier `services/api/src/domain/entities/bot_config.rs` : `config_value: String` → `serde_json::Value`
  - Modifier `services/api/src/adapters/outbound/postgres/bot_config_repository.rs` : `FromRow` + `set_config()`
  - Modifier `services/api/src/adapters/inbound/http/dto/bot_config.rs` : DTOs HTTP
  - Vérifier les clients desktop/bots qui lisent `config_value` (sérialisation JSON)
  - Ajouter `CREATE INDEX ... USING GIN (config_value)`
- [ ] **[DB #11]** Ajouter les contraintes `NOT NULL` et `CHECK` manquantes (après audit des données existantes)

**Gain** : validation au niveau DB, types Rust plus sûrs, opérations bitwise SQL pour permissions.

#### A.4 — Partitionnement des tables event-heavy (2-3h IA)

> 🔴 **Chantier majeur** mais transparent pour l'applicatif (aucune requête n'utilise `ONLY`, tous les INSERTs bindent `created_at`).

- [ ] **[DB #6]** Partitionnement par `RANGE(created_at)` des tables :
  - `infractions`, `audit_logs`, `user_activity_log`, `moderation_actions`
  - `security_events`, `logs`, `daily_activity`, `hourly_activity`, `coude_casino_log`
- [ ] Pour chaque table : RENAME + CREATE partitionné + INSERT SELECT + DROP old
- [ ] Créer les partitions mensuelles pour les 12 prochains mois + partition `DEFAULT` pour l'historique
- [ ] Vérifier que toutes les contraintes `UNIQUE` incluent la clé de partition (à modifier si nécessaire)
- [ ] **[NOUVEAU WORKER]** Créer `partition-manager-worker` qui :
  - Le 25 de chaque mois : crée la partition du mois M+2
  - Archive/drop les partitions > rétention configurable (12 mois par défaut)

**Gain** :
- Queries temporelles **10-100× plus rapides**
- VACUUM **-80 à -95 %** de durée
- Purges en **O(1)** via DROP PARTITION
- Possibilité d'archivage transparent

#### A.5 — Tuning RAM Postgres/Redis (30min IA, zéro code)

> 💡 **Convertir la RAM inutilisée en performance pure.** Avec 16 GB serveur et ~2-3 GB utilisés par l'applicatif, il reste 12 GB qui dorment. Voici comment les transformer en gain concret. **Aucun impact code, 100 % tuning de config, rollback trivial.**

**Ordre précis des tâches** — à faire dans cet ordre, valider chaque étape avant la suivante :

##### Étape 1 — Postgres `shared_buffers` (LE plus gros gain, 5 min)

- [ ] Éditer `postgresql.conf` (ou surcharger via docker-compose `command:`)
- [ ] Définir `shared_buffers = 4GB` (25 % de 16 GB)
- [ ] Redémarrer Postgres
- [ ] Vérifier avec `SHOW shared_buffers;`
- [ ] **Valider** : lancer quelques queries hot et comparer à la baseline (Phase 0)

**Gain attendu** : 5-50× sur les queries read-heavy. C'est LA config la plus impactante de tout le projet.

##### Étape 2 — Postgres `effective_cache_size` (2 min)

- [ ] Définir `effective_cache_size = 10GB` dans `postgresql.conf`
- [ ] Reload config : `SELECT pg_reload_conf();` (pas besoin de restart)
- [ ] Vérifier : `SHOW effective_cache_size;`

**Gain** : le query planner préfère les index scans aux seq scans. Pas de consommation RAM directe, juste une indication.

##### Étape 3 — Postgres `work_mem` (5 min)

- [ ] Définir `work_mem = 64MB` dans `postgresql.conf`
- [ ] Reload config
- [ ] **Vérifier le budget** : `max_connections × work_mem × 2 (opérations moyennes)` doit rester < 30 % RAM totale
  - Avec 30 conn × 64 MB × 2 = 3.8 GB au pic → OK sur 16 GB
- [ ] **Valider** : monitorer les `temp files` avec `pg_stat_database.temp_bytes` — doit diminuer drastiquement

**Gain** : les `ORDER BY`, `GROUP BY`, `HASH JOIN` 10-100× plus rapides (plus de spill sur disque).

##### Étape 4 — Postgres `maintenance_work_mem` (2 min)

- [ ] Définir `maintenance_work_mem = 1GB`
- [ ] Reload config
- [ ] **Valider** : lancer un `VACUUM` ou `REINDEX` sur une grosse table, chronométrer

**Gain** : VACUUM 5-10× plus rapide, CREATE INDEX quasi-instantané. Critique pour les phases de maintenance.

##### Étape 5 — Postgres WAL tuning (5 min)

- [ ] Définir dans `postgresql.conf` :
  ```conf
  wal_buffers = 16MB
  checkpoint_completion_target = 0.9
  max_wal_size = 4GB
  min_wal_size = 1GB
  ```
- [ ] Redémarrer Postgres (certains paramètres WAL nécessitent restart)
- [ ] **Valider** : monitorer `pg_stat_bgwriter` pour voir les checkpoints se lisser

**Gain** : I/O d'écriture lissés, moins de pics, meilleur throughput en write.

##### Étape 6 — Huge Pages Linux pour Postgres (10 min, optionnel mais recommandé)

- [ ] Calculer les huge pages nécessaires : `shared_buffers / 2MB = 2048`
- [ ] Ajouter au kernel : `sysctl -w vm.nr_hugepages=2200` (marge 10 %)
- [ ] Persister dans `/etc/sysctl.conf`
- [ ] Ajouter `huge_pages = on` dans `postgresql.conf`
- [ ] Redémarrer Postgres
- [ ] Vérifier : `SHOW huge_pages;` → doit retourner `on`

**Gain** : réduit la pression TLB, accélère les accès à `shared_buffers` de 5-10 % supplémentaires.

##### Étape 7 — Désactiver le swap (ou réduire swappiness) (2 min)

- [ ] `sysctl -w vm.swappiness=1` (Postgres ne doit **jamais** swap)
- [ ] Persister dans `/etc/sysctl.conf`
- [ ] Optionnel : désactiver complètement avec `swapoff -a` si assez de RAM

**Gain** : évite les crashs de perf catastrophiques quand Postgres touche au swap.

##### Étape 8 — Redis `maxmemory` (3 min)

- [ ] Éditer `redis.conf` (ou command docker-compose)
- [ ] Définir :
  ```conf
  maxmemory 2gb
  maxmemory-policy allkeys-lru
  ```
- [ ] Redémarrer Redis
- [ ] Vérifier : `redis-cli CONFIG GET maxmemory`
- [ ] **Valider** : monitorer `INFO memory` et `INFO stats` (keyspace_hits / keyspace_misses) — le hit rate doit augmenter

**Gain** : hit rate cache passe typiquement de 60-70 % à 85-95 %. Moins d'allers-retours vers Postgres.

##### Étape 9 — Cache Rust in-memory (moka) sur l'API (20 min IA)

- [ ] Ajouter `moka = { version = "0.12", features = ["future"] }` au `Cargo.toml` de l'API
- [ ] Créer `services/api/src/adapters/outbound/cache/moka_cache.rs`
- [ ] Wrapper sur les repositories hot :
  - `GuildConfigRepository` — TTL 5 min, capacity 10k
  - `PermissionsRepository` — TTL 2 min, capacity 50k
  - `UserProfileRepository` — TTL 10 min, capacity 100k
- [ ] Invalidation sur writes via pub/sub Redis
- [ ] **Valider** : mesurer la latence des endpoints concernés avant/après

**Gain** : lookup en **~100 ns** (RAM process) vs ~500 µs (Redis) vs ~5 ms (Postgres). Facteur **1000-50000** sur les données cachables.

##### Étape 10 — Cache Serenity agressif (bots concernés uniquement)

- [ ] Pour `moderation-bot` et `automod-bot` : augmenter `CacheSettings.max_messages` à 1000-2000
- [ ] Pour `audit-bot` : cache members complet des guilds surveillées
- [ ] **Valider** : monitorer les appels REST Discord, qui doivent chuter de 50-90 %

**Gain** : moins de rate-limits Discord, latence bot plus faible, UX améliorée.

---

#### A.6 — PgBouncer + pools SQLx (30min-1h IA)

> ⚠️ **À faire après le tuning RAM** : PgBouncer augmente le nombre de clients simultanés, donc il faut d'abord que chaque connexion Postgres soit efficace (work_mem, shared_buffers) avant de les multiplier.

- [ ] **[OPT #6]** Déployer **PgBouncer** en transaction pooling devant Postgres (ajout docker-compose)
- [ ] Configuration PgBouncer :
  ```ini
  pool_mode = transaction
  max_client_conn = 1000
  default_pool_size = 25
  reserve_pool_size = 5
  ```
- [ ] Ajuster les pools SQLx :
  - API : `max_connections = 20-30`
  - Workers : `max_connections = 5-10` chacun
- [ ] Vérifier la compatibilité transaction pooling (pas de `SET`, `LISTEN/NOTIFY`, advisory locks session-scoped)
- [ ] **Valider** : `pg_stat_activity` doit montrer un nombre stable de connexions backend (quelques dizaines) avec des centaines de clients PgBouncer

**Gain** : 5-10× plus de clients simultanés sans fatiguer Postgres.

---

### 📊 Budget RAM final recommandé (serveur 16 GB)

| Composant | RAM allouée | Impact |
|---|---|---|
| Postgres `shared_buffers` | **4 GB** | 🔴🔴🔴 Queries read 5-50× |
| Postgres `work_mem` × 30 conn au pic | ~2 GB | 🔴🔴 Sorts/joins 10-100× |
| Postgres `maintenance_work_mem` | 1 GB | 🔴 VACUUM 5-10× |
| Redis `maxmemory` | 2 GB | 🔴🔴 Hit rate +30 % |
| Cache Rust moka (API) | ~500 MB | 🔴🔴 Lookup 1000× |
| Bots + API + workers (actuel) | ~2.5 GB | — |
| Page cache Linux (automatique) | ~3 GB | 🔴🔴 Cache disque gratuit |
| Marge OS + pics | ~1 GB | sécurité |
| **Total** | **~16 GB** | Exploitation optimale |

### ⚠️ Checklist de validation finale

Après avoir fait les 10 étapes, vérifier :

- [ ] `free -h` montre ~12 GB utilisés, ~4 GB libres (destinés au page cache)
- [ ] `pg_stat_database.temp_bytes` a chuté (pas de spill disk)
- [ ] `pg_stat_bgwriter.checkpoints_timed` >> `checkpoints_req` (checkpoints planifiés)
- [ ] Redis `INFO stats` : `keyspace_hits / (hits + misses)` > 85 %
- [ ] Latence p95 des endpoints hot divisée par 3-10× vs baseline Phase 0
- [ ] Aucun OOM dans `dmesg`

### 🎯 Si budget temps très limité sur cette phase

Fais **uniquement les étapes 1 + 3 + 8** (shared_buffers, work_mem, Redis maxmemory). **15 minutes**, **80 % du gain total**. Le reste peut attendre.

**Gain** : passage de ~2 GB utilisés à ~12 GB utilisés activement, pour une **amélioration perf globale de 3-10×**.

### Partie B — Multi-tenant (4-6h IA)

- [ ] **[AUTH]** Implémenter Solution 1 : filtrage par guild via Discord OAuth2
- [ ] **[AUTH]** Nouveau middleware API `guild_auth_middleware`
- [ ] **[AUTH]** Cache Redis des guilds autorisées (TTL 5 min)
- [ ] **[AUTH]** Modifier `apps/desktop/src-tauri/src/infrastructure/api_adapter.rs` pour envoyer le token Discord
- [ ] **[AUTH]** Refuser l'accès si `guild_id` demandé n'est pas dans la liste des guilds autorisées

### Livrable

- Baseline DB améliorée de 5-10× sur les endpoints hot
- Isolation multi-tenant opérationnelle : partager l'app à un admin externe est safe

---

## Phase 3 — Refactor god files ✅ **TERMINÉE**

**Durée IA** : 1 jour (refactor quasi-mécanique, très adapté au full IA)
**Dépend de** : rien (peut démarrer en parallèle de Phase 2B)
**Doit être fait avant** : ajout de nouvelles features dans les fichiers concernés

On découpe **avant** d'ajouter les nouveaux workers et features, sinon on ajoute des lignes à des fichiers déjà monstrueux. Le refactor mécanique (split, imports, re-exports) est une tâche idéale pour l'IA.

### Tâches (ordre recommandé dans l'ancien `REFACTOR_GOD_FILES.md`, désormais supprimé)

- [x] **[REFAC P1]** `handlers/coude.rs` (2370 lignes) → split en 8 fichiers **+ hexagonal complet** (6 slices verticales)
  - 8 handlers : `coude/{mod, dto, players, combats, bets, economy, inventory, social}.rs`
  - 6 entités domaine (`coude_player`, `coude_combat`, `coude_bet`, `coude_inventory`, `coude_social`) + fonction pure `calculate_bet_resolution` avec 3 tests unitaires
  - 6 ports outbound (`Coude{Player,Combat,Bet,Economy,Inventory,Social}Repository`)
  - 6 ports inbound (`ManageCoude{Players,Combats,Bets,Economy,Inventory,Social}UseCase`)
  - 6 services + 6 adapters PG
  - Wiring AppState complet + stubs tests
  - **3 bugs latents corrigés au passage** : `FighterBetBonus.total_pot` manquant dans la réponse, race condition double-resolve combats, endpoints manquants pour `/reset-stats` et `/season/current`
- [x] **[REFAC P2]** `blackjack-bot/handler.rs` → 4 fichiers (`handler/{mod, table, game, afk_cleanup}.rs`)
- [x] **[REFAC P2]** `blackjack-bot/commands/blackjack.rs` → 4 fichiers (`blackjack/{mod, embeds, buttons, messages}.rs`)
- [x] **[REFAC P2]** `voice-bot/handlers/voice.rs` (847 lignes) → 4 fichiers (`voice/{mod, member_events, channel_lifecycle, channel_permissions}.rs`)
- [x] **[REFAC P3]** `api/handlers/blackjack.rs` → 4 fichiers (`blackjack/{mod, dto, game, tables}.rs`)
- [x] **[REFAC P3]** `audit-bot/handler.rs` → 3 fichiers (`handler/{mod, type_keys, watched_users}.rs`)
- [x] **[REFAC P3]** `coude-bot/commands/coude.rs` → 2 fichiers (`coude/{mod, challenge_ui}.rs`) — extraction des embeds/boutons uniquement ; `validation.rs` non créé car la logique reste trop couplée au flow du handler pour justifier une extraction mécanique
- [x] **[REFAC P3]** `ComponentConfigPage.vue` (1185 → 1112 lignes) + extraction `molecules/BotTokenManager.vue` — seul composant réellement self-contained ; `ConfigForm.vue` et `ConfigToggles.vue` non extraits car le v-model bidirectionnel sur les champs dynamiques nécessiterait une refonte plus risquée
- [x] **[REFAC P3]** `AuditPage.vue` (410 → 197 lignes) + extraction `molecules/AuditEventDetail.vue` — l'essentiel de la complexité (10+ templates conditionnels) déplacée dans un composant dédié ; `AuditFilters.vue` non extrait car déjà minimaliste (2 inputs)

### Livrable

- ✅ Plus aucun god file (le plus gros restant : `handlers/coude/dto.rs` à 597 lignes, mais 100% déclaratif — DTOs + From impls)
- ✅ Architecture hexagonale complète pour le domaine Coup de Coude
- ✅ 229 tests lib sentinel-api verts, 0 warning nouveau
- ✅ `cargo check` et `vue-tsc` clean sur tous les crates/apps touchés
- ✅ API publique préservée à 100% (router.rs et consommateurs inchangés)

---

## Phase 4 — ai-worker + workers prioritaires ✅ **TERMINÉE (partielle)**

**Durée IA** : 1-2 jours
**Dépend de** : Phase 2A (DB saine), Phase 3 (fichiers refactorés)

> ✅ **Livré** : ai-worker complet (queue async + worker), temp-roles-worker
> (extraction propre), sanction-expiry-worker (enrichi sur job existant).
> **⏸️ Différé** : voice-afk-worker (sweep 100 % in-memory dans le bot,
> extraction nécessite refonte du tracker partagé — gain marginal).

### Récapitulatif livré

#### A — ai-worker

- **Migration `105_create_ai_jobs.sql`** : table `ai_jobs(id, guild_id, job_type, status, input_payload, result_payload, error_message, retries, max_retries, cost_tokens, created_at, started_at, completed_at)` avec contraintes CHECK sur `status` et `job_type`. Index hot path : `idx_ai_jobs_pending WHERE status='pending'`, `idx_ai_jobs_processing WHERE status='processing'` (timeout detector), `idx_ai_jobs_guild_created` (futur quota/billing).
- **Endpoints API** (`services/api/src/adapters/inbound/http/handlers/ai_jobs.rs`)
  - `POST /api/ai/jobs` → 202 Accepted avec `{job_id, status: "pending"}` immédiat
  - `GET /api/ai/jobs/:id` → statut courant + résultat si terminé
  - Pattern pragmatique direct sqlx (comme `bot_persistence.rs`), validations simples
- **Crate `services/workers/ai-worker/`** (full crate avec `main.rs`, `config.rs`, `scheduler.rs`, `jobs/drain_ai_jobs.rs`, `Cargo.toml`, `Dockerfile`)
  - Job `drain_ai_jobs` : poll toutes les 2s
  - Reset automatique des jobs `processing` zombies (timeout configurable, défaut 120s)
  - Claim atomique via `UPDATE ... SELECT ... FOR UPDATE SKIP LOCKED RETURNING` (concurrence-safe pour scaler horizontalement)
  - Dispatch HTTP vers `/analyze` ou `/analyze/image` de l'API (le worker n'embarque PAS ONNX, simplification déploiement)
  - Retry exponentiel via `retries++` jusqu'à `max_retries` (3 par défaut), au-delà → status `'dead'` (DLQ logique)
  - Publication du résultat sur Redis pub/sub `ai_result:{job_id}` + `SET` avec TTL 600s (bots qui se réveillent en retard)
  - Wired dans `docker-compose.yml` avec deps postgres + redis
- **⏸️ Non livré** : refactor des bots `automod-bot`/`image-bot` pour passer en async opt-in. Les bots continuent à appeler `/analyze` synchrone (5s timeout). La queue est en place, à eux d'opter pour le pattern async sur une itération suivante.

#### B.1 — temp-roles-worker

- **Crate `services/workers/temp-roles-worker/`** (full crate, Dockerfile inclus)
- Job `expire_temp_roles` : scan `temp_roles WHERE expires_at <= NOW()` toutes les 60s
- Publie un event `temp_role_expire` sur `sentinel:events` (Redis pub/sub) que `community-bot` écoute déjà via `sentinel_shared::redis_listener`
- Le worker n'appelle PAS Discord directement (pas de gateway connection) — le `community-bot` fait le `member.remove_role()` localement et `DELETE` la ligne via l'API existante
- Wired dans `docker-compose.yml`
- **Note** : le code de cleanup actuel dans `bots/community-bot/src/main.rs:66-107` peut être supprimé une fois le worker validé en production. Pour cette itération, on coexiste (pas de breaking change immédiat).

#### B.2 — sanction-expiry-worker (enrichissement, pas un nouveau crate)

- **Découverte de l'audit** : la table `sanction_reminders` (migration 042) ET un job `send_reminders` existaient déjà dans `moderation-worker`. L'ancien job se contentait de marquer `status='sent'` et logger — aucune notification effective au modérateur.
- **Enrichissement du job existant** (`services/workers/moderation-worker/src/jobs/send_reminders.rs`) :
  - Ajoute un paramètre `&redis::Client`
  - Pour chaque rappel : publie un event `sanction_expiry_reminder` sur `sentinel:events` avec tous les champs nécessaires (`reminder_id`, `guild_id`, `moderator_id/name`, `target_id/name`, `action_type`, `reason`, `expires_at`, `minutes_left`)
  - Le `moderation-bot` peut maintenant écouter cet event et envoyer un DM au modérateur (gateway side)
  - Marquage `status='sent'` AVANT publication pour éviter les doublons (idempotence)
- **Câblage** : `moderation-worker/src/main.rs` crée le `redis::Client`, `scheduler.rs` le passe au job, `config.rs` ajoute `redis_url`, `docker-compose.yml` injecte `REDIS_URL` dans le container.

#### B.3 — voice-afk-worker (différé)

- **Pourquoi** : le sweep AFK actuel est 100 % en mémoire (`DashMap` dans `voice-bot`), pas de DB de tracking. L'extraction propre nécessiterait soit de persister l'état (haut volume d'écritures), soit de partager un état via Redis (complexification importante), soit de garder la timer dans le bot (gain architectural nul).
- **Status** : différé en attendant un cas d'usage concret qui justifie le coût (par exemple, scaler horizontalement le voice-bot avec plusieurs instances).

### Validation

- **API** : 237/237 tests `cargo test --lib` passent (aucune régression)
- **ai-worker** : `cargo check` clean
- **temp-roles-worker** : `cargo check` clean
- **moderation-worker** (enrichi) : `cargo check --tests` clean

### Fichiers principaux

- Migration : `services/api/migrations/105_create_ai_jobs.sql`
- API handler : `services/api/src/adapters/inbound/http/handlers/ai_jobs.rs`
- Routes API : `services/api/src/adapters/inbound/http/router.rs:490-491`
- Nouveau crate : `services/workers/ai-worker/{Cargo.toml,Dockerfile,src/{main,config,scheduler}.rs,src/jobs/{mod,drain_ai_jobs}.rs}`
- Nouveau crate : `services/workers/temp-roles-worker/{Cargo.toml,Dockerfile,src/{main,config,scheduler}.rs,src/jobs/{mod,expire_temp_roles}.rs}`
- Enrichissement : `services/workers/moderation-worker/src/{config,main,scheduler}.rs` + `src/jobs/send_reminders.rs`
- docker-compose : 2 nouveaux services + REDIS_URL ajouté à moderation-worker

### Partie A — ai-worker (priorité MAX, 6-8h IA)

Extraction des appels IA texte/image hors de l'API. **Le plus gros gain architectural du projet.**

- [ ] **[WORKER #11]** Créer `services/workers/ai-worker/`
- [ ] Table `ai_jobs` (id, type, prompt, status, result, created_at, completed_at, cost_tokens)
- [ ] Endpoint API `POST /ai/jobs` → retourne `202 Accepted` avec `job_id` immédiatement
- [ ] Endpoint API `GET /ai/jobs/:id` → statut du job
- [ ] Worker consomme la queue, appelle le provider (OpenAI/Anthropic/Replicate)
- [ ] Rate-limiter global côté worker
- [ ] Retry exponentiel avec dead-letter queue
- [ ] Publication du résultat sur Redis pub/sub `ai_result:{job_id}`
- [ ] Refactor des bots consommateurs IA pour écouter le pub/sub
- [ ] Tracking coût par guild (pour future facturation/quota)

### Partie B — Workers d'extraction haute priorité (4-6h IA)

Tâches qui bloquent le gateway Discord des bots → à extraire vite.

- [ ] **[WORKER #1]** `voice-afk-worker` — extrait le sweep AFK du voice-bot
- [ ] **[WORKER #2]** `temp-roles-worker` — extrait le cleanup des rôles temporaires de community-bot
- [ ] **[WORKER #5]** `sanction-expiry-worker` — notifie 24h avant expiration des mutes/bans temporaires

### Livrable

- IA complètement asynchrone, API débloquée des appels 5-60s
- 3 nouveaux workers qui allègent les gateways Discord

---

## Phase 5 — Cache + Streams + Batch writes

**Durée IA** : 2-3 jours
**Dépend de** : Phase 2A (index DB), Phase 4 (workers de base en place)

### Partie A — Cache-aside systématique (4-6h IA)

- [ ] **[OPT #7]** Helper `cached_read<T>` dans la couche adapters de l'API
- [ ] Appliquer sur les endpoints hot identifiés via `pg_stat_statements` :
  - Guild configs
  - User profiles
  - Permissions
  - Moderation history récente
- [ ] Invalidation via Redis pub/sub lors des writes
- [ ] TTL court (30-60s) pour les données qui changent peu

### Partie B — Migration Redis Streams (4-6h IA)

- [ ] **[OPT #3]** Refactor `sentinel-shared::RedisListener` vers Redis Streams + consumer groups
- [ ] Migration des 3 bots qui utilisent déjà pub/sub (moderation, ticket, coude)
- [ ] Ajout de la persistance et ACK explicite
- [ ] Documentation pattern pour les futurs workers

### Partie C — Batch writes DB (4-6h IA)

- [ ] **[OPT #10]** Buffer en mémoire + flush toutes les 500ms OU 100 events côté API
- [ ] Appliquer aux tables event-heavy : `audit_log`, `moderation_actions`, `analytics_events`
- [ ] Utiliser `INSERT ... VALUES (...), (...), (...)` ou `COPY FROM STDIN`

### Livrable

- -70 à -90 % de charge DB sur les endpoints hot
- Workers scalables horizontalement (plusieurs instances possibles)
- 10-50× de throughput sur les tables event-heavy

---

## Phase 6 — Features moderation + workers secondaires

**Durée IA** : 1-2 jours
**Dépend de** : Phase 4 (workers de base), Phase 5 (Redis Streams pour le reminder-worker)

### Partie A — Workers secondaires (4-6h IA)

- [ ] **[WORKER #6]** `appeal-sla-worker` — SLA sur tickets d'appel, escalade +48h
- [ ] **[WORKER #7]** `discord-audit-sync-worker` — réconciliation actions hors-bot
- [ ] **[WORKER #9]** `reminder-worker` générique — infra de rappels réutilisable
- [ ] **[WORKER #10]** `export-worker` — exports JSON/CSV asynchrones
- [ ] **[WORKER #3]** `audit-cache-worker` — extrait refresh watched-users
- [ ] **[WORKER #4]** `blackjack-cleanup-worker` — extrait tables AFK

### Partie B — Améliorations fonctionnelles moderation-bot (4-8h IA)

Alignées avec `bots/moderation-bot/AMELIORATIONS.md`. Beaucoup s'appuient maintenant sur les workers de la Phase 6A.

- [ ] **[MOD #1]** Rappels & expirations actives (utilise `sanction-expiry-worker` de Phase 4)
- [ ] **[MOD #6]** Templates de sanction composables (raison + durée + gravité + DM)
- [ ] **[MOD #2]** `/evidence` — attacher preuves à une action existante
- [ ] **[MOD #4]** Confirmation interactive sur cibles à risque
- [ ] **[MOD #8]** Transcript automatique des call rooms (via `export-worker`)
- [ ] **[MOD #7]** `/modstats` — métriques par modérateur (utilise `stats-digest-worker`)
- [ ] **[MOD #3]** `/review` — file de relecture
- [ ] **[MOD #5]** `/compare` — historique croisé

### Livrable

- Couverture modération complète (manuelle + automatisée)
- Workers secondaires qui complètent l'écosystème

---

## Phase 7 — gRPC interne + scaling horizontal

**Durée IA** : 2-4 jours, à déclencher quand les limites arrivent
**Dépend de** : Phase 5 (cache + streams en place)

### Partie A — gRPC interne (1-2 jours IA)

- [ ] **[OPT #2]** Définir les contrats `.proto` pour les endpoints internes hot
- [ ] Générer les clients/serveurs via `tonic-build`
- [ ] Migrer progressivement endpoint par endpoint (commencer par `/logs`, `/heartbeat`)
- [ ] Garder REST pour le desktop et les clients externes
- [ ] **Ne migrer que les endpoints dans le top latence** — les endpoints froids restent en REST

### Partie B — RBAC fin (1 jour IA)

- [ ] **[AUTH]** Solution 3 : tables `api_users`, `api_user_guilds`, `roles`
- [ ] Rôles : `owner`, `admin`, `moderator`, `viewer`
- [ ] Middleware de vérification du rôle requis par endpoint
- [ ] Interface de gestion des permissions dans le desktop

### Partie C — Sharding Discord (uniquement si besoin)

À déclencher **seulement** quand :

- Le nombre de guilds dépasse 2500
- Les gateways Discord commencent à ramer
- On vise un déploiement multi-serveurs

- [ ] **[OPT #5]** `ShardManager` Serenity configuré dynamiquement
- [ ] Chaque shard dans son propre process
- [ ] État distribué via Redis (cache-worker)

### Stats-digest-worker (si pas déjà fait)

- [ ] **[WORKER #8]** `stats-digest-worker` — digests hebdomadaires modérateurs

---

## 📊 Tableau récapitulatif — Priorisation par effort/impact

| Phase | Chantier                                                  | Effort IA | Impact                | Type      |
| ----- | --------------------------------------------------------- | --------- | --------------------- | --------- |
| 0     | Observabilité                                             | 2-3h      | 🔴🔴🔴 (bloquant)     | Infra     |
| 1     | jemalloc + LTO + Cache Serenity + Compression + HTTP pool | 2-3h      | 🔴🔴🔴                | Quick win |
| 2A.1  | Quick wins DB (index dup, partiels, GIN, VARCHAR, dead cols) | 1-2h   | 🔴🔴                  | DB        |
| 2A.2  | Vues matérialisées leaderboards + table `user_cache`     | 3-4h      | 🔴🔴🔴                | DB        |
| 2A.3  | Breaking (enums, BIGINT perms, config_value JSONB)        | 5-7h      | 🔴🔴                  | DB        |
| 2A.4  | Partitionnement tables event-heavy + `partition-manager-worker` | 2-3h | 🔴🔴🔴              | DB        |
| 2A.5  | Tuning RAM Postgres/Redis + cache moka in-memory          | 30min-1h  | 🔴🔴🔴                | Tuning    |
| 2A.6  | PgBouncer + pools SQLx ajustés                            | 30min-1h  | 🔴🔴                  | Infra     |
| 2B    | Multi-tenant OAuth2                                       | 4-6h      | 🔴🔴 (sécurité)       | Feature   |
| 3     | Refactor god files (9 fichiers) ✅                        | 1j        | 🔴🔴 (maintenabilité) | Dette     |
| 4A    | `ai-worker`                                               | 6-8h      | 🔴🔴🔴                | Worker    |
| 4B    | 3 workers prioritaires                                    | 4-6h      | 🔴🔴                  | Worker    |
| 5A    | Cache-aside Redis                                         | 4-6h      | 🔴🔴🔴                | Perf      |
| 5B    | Redis Streams                                             | 4-6h      | 🔴🔴 (scaling)        | Infra     |
| 5C    | Batch writes DB                                           | 4-6h      | 🔴🔴                  | Perf      |
| 6A    | 6 workers secondaires                                     | 4-6h      | 🔴                    | Worker    |
| 6B    | 8 features moderation                                     | 4-8h      | 🔴🔴                  | Feature   |
| 7A    | gRPC interne                                              | 1-2j      | 🔴🔴                  | Perf      |
| 7B    | RBAC fin                                                  | 1j        | 🔴                    | Feature   |
| 7C    | Sharding Discord                                          | variable  | 🔴🔴🔴 (si besoin)    | Scaling   |

**Total effort IA** : ~10-14 jours effectifs (hors validation humaine entre phases).

> 📌 La Phase 2A a grossi car l'audit DB complet a révélé **12 optimisations schéma** (cf. `DB_OPTIMISATIONS.md`), dont le partitionnement et les vues matérialisées qui apportent à eux seuls des gains de **10-1000×** sur certaines queries.

---

## ⚠️ Anti-patterns à éviter

| À NE PAS faire                         | Pourquoi                                                                   |
| -------------------------------------- | -------------------------------------------------------------------------- |
| Sauter la Phase 0                      | Tu optimises à l'aveugle, tu peux même dégrader sans le voir               |
| Sharding avant PgBouncer               | Tu multiplies les connexions Postgres sans pooler → crash                  |
| gRPC avant Phase 1                     | Tu rewrite deux fois le `BaseApiClient`                                    |
| Cache Redis avant index Postgres       | Tu caches des queries mal écrites, tu masques les vrais problèmes          |
| Ajouter features dans les god files    | Tu alourdis ce qui doit être refactoré                                     |
| Workers avant DB saine                 | Les workers ajoutent du parallélisme → amplifient les goulots DB existants |
| Multi-tenant après avoir partagé l'app | Fuite de données garantie                                                  |
| Migration Redis Streams avant baseline | Tu ne verras pas l'impact réel                                             |

---

## 🔗 Dépendances inter-phases

```
Phase 0 ────────┐
                │
                ▼
Phase 1 (parallèle) ──┐
                      │
                      ▼
          Phase 2A (DB) ──────┬──> Phase 2B (auth)
                              │
                              ▼
          Phase 3 (refactor)
                              │
                              ▼
          Phase 4A (ai-worker) ──> Phase 4B (workers prio)
                              │
                              ▼
          Phase 5A (cache) ──> Phase 5B (streams) ──> Phase 5C (batch)
                              │
                              ▼
          Phase 6A (workers 2) ──> Phase 6B (features mod)
                              │
                              ▼
          Phase 7 (gRPC / RBAC / sharding)
```

---

## 📝 Suivi

Créer un board (GitHub Projects / Linear / autre) avec une colonne par phase et cocher les tâches au fur et à mesure. Après chaque phase, **comparer les métriques à la baseline** (Phase 0) pour valider le gain.

Ne pas hésiter à **répéter la Phase 0** après chaque grosse phase pour mettre à jour la baseline et identifier les nouveaux goulots.

---

## ⏱️ Note sur les durées "full IA"

Toutes les durées sont exprimées en **temps IA effectif** (session de génération de code). Elles ne comptent **pas** :

- Les allers-retours de validation humaine (review de diff, approbation merge)
- Les tests fonctionnels en prod avec vrai serveur Discord
- Les temps de compilation Rust (peuvent ajouter 10-30 min par session selon le workspace)
- Les déploiements et migrations DB en prod
- Le debug de bugs non reproductibles en local

**En pratique** : pour un chantier annoncé à "4-6h IA", compter **1 journée calendaire** humaine (IA + review + test + deploy). La roadmap complète s'étale donc sur **~3-4 semaines calendaires** à rythme soutenu, au lieu des 3 mois initialement estimés pour un dev solo humain.
