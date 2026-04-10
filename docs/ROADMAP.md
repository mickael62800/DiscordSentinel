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
| [`PHASES_0_2_DIFFERES.md`](./PHASES_0_2_DIFFERES.md)                              | **Ce qui a été reporté** pendant les Phases 0-2 (avec justification) |
| [`BASELINE_METRICS.md`](./BASELINE_METRICS.md)                                    | Template baseline + requêtes PromQL/SQL (Phase 0) |

---

## 🎯 Principes directeurs

1. **Mesurer avant d'optimiser** — sans baseline, impossible de valider les gains
2. **Sécurité avant features** — multi-tenant avant de partager l'app
3. **Fondations avant refonte** — DB saine avant cache, cache avant gRPC
4. **Refactor avant ajout** — découper les god files avant d'y ajouter des features
5. **Scaling en dernier** — sharding uniquement quand les limites arrivent
6. **Quick wins en parallèle** — jemalloc, compression, pool HTTP = gratuits, à faire tout de suite

---

## 📊 État d'avancement (au 2026-04-10)

| Phase | Status | Scope livré | Différés |
|---|---|---|---|
| **0** Observabilité | ✅ TERMINÉE | Intégralité du scope | — |
| **1** Quick wins | ✅ TERMINÉE | 5/5 optims | Compression Redis (reportée à Phase 5) |
| **2** Fondations DB + multi-tenant | ✅ **partielle** | A.1, A.2, A.3(2/4), A.4(4/9), A.5(8/10), A.6, B | JSONB config, NOT NULL, cache moka, 5 tables non partitionnées |
| **3** Refactor god files | ✅ TERMINÉE | Intégralité du scope | — |
| **4** ai-worker + workers prio | ✅ **partielle** | A ai-worker complet, B.1 temp-roles, B.2 sanction-expiry | voice-afk-worker (sweep in-memory) |
| **5** Cache + Streams + Batch | 🟡 **2/3** | 5B Streams ✅, 5C Batch writes ✅ | 5A Cache-aside (bloqué baseline) |
| **6** Features moderation + workers 2 | 🟡 **8/8 features, 5/6 workers** | 6A appeal-sla + export + audit-cache + blackjack-cleanup ✅, 6B 8/8 features ✅ | voice-afk (non extractible archi), discord-audit-sync (scope) |
| **7** gRPC + scaling | 🟡 **partielle** | 7B RBAC ✅ **clôturé à 100%** (23 handlers + superadmin /purge/logs) | 7A gRPC (bloqué baseline), 7C sharding (pas requis) |

> 👉 Pour le détail exhaustif de **ce qui n'a pas été fait dans les phases 0-2** (et pourquoi), voir [`PHASES_0_2_DIFFERES.md`](./PHASES_0_2_DIFFERES.md).

**Validation humaine en attente** : capturer la baseline en prod via `docs/BASELINE_METRICS.md` **avant** d'attaquer la Phase 5. Plusieurs items différés (notamment le cache moka in-memory) deviennent pertinents ou inutiles selon les chiffres observés.

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

### Partie B — Migration Redis Streams ✅ **TERMINÉE**

> ✅ **Livré** : une seule stream `sentinel:events` partagée, 2 modes de lecture
> (XREADGROUP durable + XREAD `$` live tail). L'audit a révélé que seuls 2 bots
> (moderation, ticket) consommaient réellement du pub/sub (coude-bot ne l'utilisait
> pas, contrairement à ce qu'indiquait la roadmap initiale).

#### Architecture livrée

- **Stream unique** : `sentinel:events` avec `MAXLEN ~ 10000` (borne mémoire, O(1) amorti).
- **Format entry** : un seul champ `payload` contenant le JSON `{"event", "data"}` — identique à l'ancien format pub/sub pour zéro changement côté handlers.
- **Abstraction consumer** : `sentinel_shared::event_bus::listen_stream_group(group, consumer, handler)` (bots/shared/src/event_bus.rs) — XREADGROUP + XACK, reconnexion exponentielle, auto-claim des pending > 60s via XAUTOCLAIM, création idempotente du consumer group (MKSTREAM + BUSYGROUP).
- **Helper publish** : `sentinel_shared::event_bus::publish(conn, event, data)` — XADD avec MAXLEN ~.
- **Consumer name** : `default_consumer_name()` → `{HOSTNAME}-{pid}`, stable à travers les redémarrages sur docker-compose/k8s StatefulSet.

#### Producers migrés (5)

- `bots/shared/src/api_client.rs::EventPublisher` → délègue à `event_bus::publish` (lazy-connect préservé, utilisé par community/moderation/ticket/progression bots pour `publish_event`)
- `services/api/src/adapters/inbound/ws/broadcaster.rs::EventBroadcaster` → XADD inline
- `services/workers/monitoring-worker/src/monitor.rs` → helper `xadd_event` local pour bot_status online/offline
- `services/workers/temp-roles-worker/src/jobs/expire_temp_roles.rs` → XADD inline (temp_role_expire)
- `services/workers/moderation-worker/src/jobs/send_reminders.rs` → XADD inline (sanction_expiry_reminder), marquage `status='sent'` AVANT XADD pour idempotence

#### Consumers migrés

- **moderation-bot** (`bots/moderation-bot/src/handler.rs:64`) : consumer group `moderation-bot`, XREADGROUP durable → les events de sanction-expiry/pending-action sont rejoués si le bot était down au moment de l'émission
- **ticket-bot** (`bots/ticket-bot/src/handler.rs:63`) : consumer group `ticket-bot`, XREADGROUP durable → idem pour ticket_sla_updated et events desktop
- **gateway** (`services/gateway/src/redis_subscriber.rs`) : XREAD `$` live tail (pas de group, pas de rattrapage) → sémantique fire-and-forget préservée pour le relay WebSocket desktop (on ne veut pas rejouer 1000 events périmés au reconnect client)

#### Pattern pour les futurs workers

**Émettre un event depuis un worker** (pas besoin de l'abstraction, 10 lignes) :

```rust
let _: String = redis::cmd("XADD")
    .arg("sentinel:events")
    .arg("MAXLEN").arg("~").arg(10_000)
    .arg("*")
    .arg("payload").arg(&serde_json::json!({"event": "my_event", "data": {...}}).to_string())
    .query_async(&mut conn).await?;
```

**Consommer des events dans un bot** (utiliser l'abstraction) :

```rust
tokio::spawn(async move {
    let consumer = sentinel_shared::event_bus::default_consumer_name();
    sentinel_shared::event_bus::listen_stream_group(
        "my-bot".to_string(),  // consumer group = nom du bot
        consumer,               // consumer name = hostname-pid
        move |payload_json| async move {
            // payload_json est la String `{"event", "data"}` (même format qu'avant)
            handle_event(&payload_json).await;
        },
    ).await;
});
```

**Gains obtenus** :

- Durabilité : les events survivent au redémarrage d'un consumer (vs pub/sub qui perdait tout)
- ACK explicite : XACK après traitement réussi, auto-claim des pending abandonnés
- Scaling horizontal : plusieurs replicas d'un même bot partagent le consumer group, Redis distribue les events entre eux
- Observabilité : `XINFO STREAM sentinel:events` + `XPENDING sentinel:events <group>` donnent une vue précise du lag et des entries bloquées
- Format payload identique → zéro changement côté handlers, migration transparente

#### Suppression de l'ancien code

- `bots/shared/src/redis_listener.rs` **supprimé** (remplacé par `event_bus.rs`)
- `lib.rs` mis à jour, feature `streams` ajoutée à la dép `redis` dans `bots/shared/Cargo.toml` et `services/gateway/Cargo.toml`

#### Tests

- 3 nouveaux tests unitaires dans `event_bus::tests` :
  - `default_consumer_name_has_pid` (validité du format hostname-pid)
  - `parse_autoclaim_empty_reply` (parsing XAUTOCLAIM sans résultats)
  - `parse_autoclaim_single_entry` (parsing XAUTOCLAIM avec 1 entry)
- `cargo test --lib` : 16/16 sur sentinel-shared, 213/213 sur services/api (hors tests ML ONNX non-gated)

### Partie C — Batch writes DB ✅ **TERMINÉE**

> ✅ **Livré** : `BatchWriter<T>` générique + wrapper `BatchedPgLogRepository` et
> `BatchedPgAuditLogRepository`. Flush 500ms OR batch 100 entries, multi-row INSERT
> via `sqlx::QueryBuilder::push_values`. Sémantique at-most-once assumée.

#### Architecture livrée

- **Nouveau module** `services/api/src/adapters/outbound/batching/` avec 3 fichiers :
  - `batch_writer.rs` — `BatchWriter<T>` générique (tokio mpsc bounded + flusher task, `tokio::select!` entre interval tick et `recv()`)
  - `log_batcher.rs` — `BatchedPgLogRepository` qui wrap `PgLogRepository`, override `save` pour enqueue, délègue `find_all`/`delete_*` au repo synchrone
  - `audit_log_batcher.rs` — idem pour `AuditLog`
- **Config par défaut** (`BatchWriterConfig::default`) : `flush_interval=500ms`, `max_batch_size=100`, `channel_capacity=10_000`
- **Drop policy** : `try_send` non-bloquant ; si la queue est pleine, l'entry est dropped avec un warn (on ne bloque JAMAIS le request path)
- **Graceful shutdown** : à la fermeture du channel (drop du dernier Sender), le flusher draine le buffer restant avant d'exit
- **Fast-drain** : après `recv()`, la flusher utilise `try_recv()` en boucle pour absorber d'un coup toutes les entries en attente dans le channel jusqu'à `max_batch_size`, ce qui maximise la taille des batchs sous charge

#### Tables migrées

- **`logs`** (la plus hot) : middleware `api_logger_middleware` + `dashboard.rs` POST /api/logs + 2 events lifecycle dans `main.rs`. Volume estimé : 10-100 req/s.
- **`audit_logs`** : via `ManageAuditLogsService::save`. Volume plus faible mais gain confort + uniformité.

Autres tables event-heavy identifiées mais **non batchées** :
- `moderation_actions` : volume faible (une action = un `/warn` humain), pas de ROI justifiant le risque at-most-once
- `analytics_events` : table non trouvée dans le code — probablement prévue par `DB_OPTIMISATIONS.md` mais pas encore créée
- `user_activity_log` / `security_events` : écrits via `bot_persistence.rs` (handler HTTP direct) — hors scope de ce refactor repository-centric

#### Modifications main.rs

```rust
let log_repo = Arc::new(BatchedPgLogRepository::new(
    pg_pool.clone(),
    BatchWriterConfig::default(),
));
let audit_log_repo = Arc::new(BatchedPgAuditLogRepository::new(
    pg_pool.clone(),
    BatchWriterConfig::default(),
));
```

Le typage `Arc<dyn LogRepository>` / `Arc<dyn AuditLogRepository>` reste identique → zéro changement dans l'AppState, les services applicatifs, ou les handlers HTTP.

#### Trade-off sémantique

**At-most-once** assumé : en cas de crash de l'API entre l'enqueue et le flush, les entries en buffer sont perdues (max 500ms ou 100 entries). C'est le prix du gain de throughput. **Ne jamais utiliser ce pattern pour des écritures transactionnelles** (infractions, transactions économiques, etc.) — ces repositories restent synchrones.

#### Tests

- 3 nouveaux tests unitaires dans `batch_writer::tests` :
  - `flushes_when_batch_full` — trigger taille (batch 3, 5 items → 1 flush de 3, 2 restent en buffer)
  - `flushes_on_interval` — trigger temps (interval 50ms, 2 items → flush après 150ms)
  - `drains_on_channel_close` — drop du writer → flush final avant exit de la flusher task
- `cargo test --lib` : 216/216 (hors tests ML ONNX non-gated)

#### Gain attendu

Sous charge réelle : **10-50× throughput** sur les tables concernées. La charge DB sur `logs` devrait chuter drastiquement (1 INSERT de 100 rows vs 100 INSERTs de 1 row = environ ~100× moins de round-trips réseau, ~10× moins de WAL flushes). À valider avec `pg_stat_statements` sur la baseline prod.

### Livrable

- -70 à -90 % de charge DB sur les endpoints hot
- Workers scalables horizontalement (plusieurs instances possibles)
- 10-50× de throughput sur les tables event-heavy

---

## Phase 6 — Features moderation + workers secondaires

**Durée IA** : 1-2 jours
**Dépend de** : Phase 4 (workers de base), Phase 5 (Redis Streams pour le reminder-worker)

### Partie A — Workers secondaires 🟡 **partielle**

> ✅ **Livré** : `appeal-sla-worker` — escalade automatique des tickets d'appel
> de sanction en breach de SLA. Migration 106 + nouveau crate + docker-compose
> wiring.
>
> ⏸️ **Différés / skip** (justification après audit) :
> - **reminder-worker générique** : la fonctionnalité est **déjà livrée** par
>   le `moderation-worker/send_reminders.rs` enrichi en Phase 4 B.2. Pas de
>   second use-case pour justifier une abstraction générique — skip (éviter
>   la premature abstraction).
> - **audit-cache-worker** : ✅ **livré** (voir récapitulatif plus bas). Cache
>   migré vers Redis + event stream pour refresh.
> - **blackjack-cleanup-worker** : ✅ **livré**. La colonne
>   `blackjack_tables.last_activity` existait déjà en DB (migration 094) et
>   était déjà mise à jour par l'API, donc le refactor a été propre : query
>   DB + UPDATE + event stream, le bot consume pour supprimer le channel
>   Discord local.
> - **voice-afk-worker** : ⏸️ **non extractible** architecturalement. Le
>   `AfkTracker` de voice-bot stocke des `Instant` (monotonic, non sérialisable)
>   capturés en temps réel via les events voice_state_update Gateway. Le sweep
>   appelle `move_member()` qui nécessite Gateway+Http, et utilise
>   `ctx.cache` + `voice_owner_map` in-memory. Un worker externe n'a pas
>   accès à ces ressources. Le design local reste le bon choix ici.
> - **discord-audit-sync-worker** : nouveau scope business plus important
>   (intégration Discord audit log + réconciliation), mieux adressé en
>   session dédiée.
> - **export-worker** : ✅ **livré** (voir récapitulatif plus bas).

#### Récapitulatif `appeal-sla-worker`

- **Migration `106_appeal_sla.sql`** : ajoute `escalated_at TIMESTAMPTZ` à
  `tickets` (les colonnes `first_response_at`/`resolved_at` existaient déjà
  depuis migration 053). Index partiel `idx_tickets_appeal_sla_pending` ciblé
  sur `category='appel_sanction' AND status IN ('open','assigned') AND
  escalated_at IS NULL AND first_response_at IS NULL`.
- **Crate `services/workers/appeal-sla-worker/`** (full crate avec Dockerfile) :
  scan toutes les 120s (configurable via `APPEAL_SLA_SCAN_INTERVAL`), charge
  la config SLA par guild depuis `bot_guild_config` (`bot_name='ticket-bot'`,
  clés `sla_first_response_minutes` et `sla_escalation_minutes` de la
  migration 047, défauts 30 et 60 min).
- **Logique** :
  1. Charge toutes les configs SLA en une seule query (`HashMap<guild_id, (first_response, escalation)>`)
  2. Scanne les tickets candidats (pre-filtrage SQL large, affinage per-guild en Rust)
  3. Pour chaque ticket en breach : `UPDATE tickets SET escalated_at = NOW()
     WHERE id = $1 AND escalated_at IS NULL` (garde idempotence multi-workers)
  4. Publie un event `appeal_sla_escalated` via XADD sur `sentinel:events`
     (pattern Phase 5B) avec `{ticket_id, guild_id, author_id, author_name,
     title, created_at, age_minutes, sla_*_minutes}`
- **Event consumer** : non livré dans cette itération. Le
  `moderation-bot::handle_redis_moderation_event` actuel ne dispatche que
  `moderation_action` (early-return sur le reste). Gap connu depuis Phase 4
  (le `sanction_expiry_reminder` est aussi dans ce cas). Les deux seront
  ajoutés ensemble dans le flow Phase 6B features moderation.
- **docker-compose** : nouveau service `appeal-sla-worker` wired sur
  `pgbouncer` + `redis`, env `APPEAL_SLA_SCAN_INTERVAL=120`.
- **Validation** : `cargo check` clean, worker suit exactement le pattern
  `temp-roles-worker` (Phase 4 B.1).

#### Récapitulatif `export-worker`

Exports asynchrones de moderation data (infractions, audit_logs,
moderation_actions) en CSV/JSON pour éviter de bloquer l'API sur des
queries massives.

- **Migration `110_create_export_jobs.sql`** : table `export_jobs` similaire
  à `ai_jobs` (Phase 4 A). Colonnes `id`, `guild_id`, `requested_by`,
  `job_type` (CHECK in `infractions|audit_logs|moderation_actions`),
  `format` (CHECK in `csv|json`), `filters JSONB`, `status`, `result TEXT`,
  `result_rows`, `error_message`, `retries`, `max_retries`, timestamps.
  Index partiels `idx_export_jobs_pending WHERE status='pending'` et
  `idx_export_jobs_processing WHERE status='processing'` (timeout detector).

- **API endpoints** (`handlers/exports.rs`, direct sqlx) :
  - `POST /api/exports/jobs` — enqueue + 202 Accepted immédiat,
    **gated `Moderator+`** via `require_role_for_guild` (body-based),
    validation du `job_type` et `format`
  - `GET /api/exports/jobs/{id}` — status + `result` (si done)

- **Crate `services/workers/export-worker/`** (full crate avec Dockerfile) :
  - Scan toutes les 5s (configurable `EXPORT_SCAN_INTERVAL`)
  - Reset automatique des jobs `processing` zombies (> 300s)
  - **Claim atomique** via `UPDATE ... WHERE id = (SELECT ... FOR UPDATE
    SKIP LOCKED LIMIT 1) RETURNING` — scale horizontal possible sans
    collision
  - Exporters par type : `export_infractions`, `export_audit_logs`,
    `export_moderation_actions` (queries paramétrées avec `LIMIT 50000`
    garde-fou)
  - **Serialization CSV** avec escaping propre (`,`, `"`, `\n`) ou JSON
    via `serde_json`
  - **Retry exponentiel** : `retries++` jusqu'à `max_retries` (3), au-delà
    → status `'dead'` (DLQ logique) + `error_message` persisté

- **docker-compose** : service `export-worker` wired sur `pgbouncer`,
  env `EXPORT_SCAN_INTERVAL=5`. Pas de Redis (pas d'events émis).

- **Tests** : 5 tests unitaires sur `csv_escape` + `to_csv` (simple,
  comma, quote, newline, roundtrip).

- **Stockage inline volontaire** : résultat dans `export_jobs.result` TEXT.
  Pas de disk/S3 pour éviter la complexité. Limite pratique : 50k lignes
  par export (garde-fou `MAX_ROWS_PER_EXPORT`). Pour de gros exports (>25 MB
  = limite Discord attachment), un design ultérieur avec storage externe
  sera nécessaire.

- **Débloque MOD #8** : le pattern (enqueue + poll + inline result) est
  directement réutilisable pour le transcript des call rooms — ajouter un
  `job_type = 'call_transcript'` avec filters `{channel_id, time_range}`
  + l'exporter correspondant qui query `messages` ou le `channel history`
  Discord.

#### Récapitulatif `audit-cache-worker`

Refresh périodique du cache `watched_users` pour audit-bot. Avant Phase 6A,
audit-bot faisait une boucle interne `sleep(60s) + API call` dans `ready()`.
Ce pattern ne scale pas horizontalement (N replicas = N appels API dupliqués).

**Design cache→Redis** :

- **Worker** (`services/workers/audit-cache-worker/`) : toutes les 60s,
  query Postgres direct (`SELECT DISTINCT user_id FROM infractions UNION
  manual_watched_users`), push dans Redis key `audit:watched_users` (TTL
  300s fail-safe), puis publie un event `watched_users_refreshed` sur
  la stream `sentinel:events` (pattern Phase 5B).

- **Bot** (`audit-bot/handler/watched_users.rs`) : 2 nouvelles fonctions
  helper exportées vers `handler/mod.rs` :
  - `bootstrap_watched_users(ctx)` : appelé au startup. Lit Redis en
    priorité, fallback API une seule fois si Redis vide (premier deploy,
    worker pas encore démarré).
  - `handle_watched_refresh_event(ctx, payload)` : consume le stream
    event `watched_users_refreshed`, re-read le snapshot depuis Redis,
    rafraîchit le `DashSet<String>` local.
  - Consumer durable via `sentinel_shared::event_bus::listen_stream_group`
    avec group `audit-bot-watched-cache` (pattern Phase 5B).

- **Ancienne boucle supprimée** : le `tokio::spawn { loop { sleep(60s) +
  api.get_all_watched_user_ids() } }` dans `handler/mod.rs::ready()` est
  remplacé par `bootstrap + listen_stream_group`. Hot path (`is_watched()`)
  inchangé : toujours une lecture in-memory `DashSet::contains()`.

- **Scaling horizontal** : si audit-bot est déployé en N replicas, le
  consumer group `audit-bot-watched-cache` garantit qu'UN seul replica
  re-fetch par event (les autres sont idle sur le XREADGROUP). Pas de
  duplication d'appels API/DB.

- **docker-compose** : service `audit-cache-worker` wired sur `pgbouncer`
  + `redis`, env `AUDIT_CACHE_REFRESH_INTERVAL=60`.

**Gains** :
1. Extraction d'une responsabilité du bot → worker dédié
2. Scaling horizontal d'audit-bot devient possible sans N appels API
3. Hot path `is_watched()` inchangé (perf identique)
4. Fail-safe : TTL Redis 300s + fallback API au bootstrap

**Pattern cache→Redis validé** : peut être réutilisé pour
`voice-afk-worker` et `blackjack-cleanup-worker` qui étaient bloqués par
le même pattern in-memory. Le template est maintenant établi :
worker périodique → push Redis snapshot + stream event → bot consume +
update cache local.

### Partie B — Améliorations fonctionnelles moderation-bot 🟡 **wave 1 livrée**

Alignées avec `bots/moderation-bot/AMELIORATIONS.md`. Découpage en waves
pour éviter un scope ingérable.

#### Wave 1 ✅ (livré)

**[MOD #1]** — Rappels & expirations actives :
- **Refactor event dispatcher** : `bots/moderation-bot/src/handler.rs::handle_redis_moderation_event` passe d'un early-return mono-event à un match multi-events (`moderation_action`, `sanction_expiry_reminder`, `appeal_sla_escalated`). Débloque la valeur des Phase 4 B.2 et Phase 6A qui émettaient dans le vide.
- **Handler `sanction_expiry_reminder`** : DM au modérateur qui a posé la sanction, 1h avant expiration. Embed avec action, cible, temps restant, raison.
- **Handler `appeal_sla_escalated`** : post dans le salon de logs avec `@here` + embed rouge + infos du ticket en breach.
- **Commande `/expirations`** (`bots/moderation-bot/src/commands/expirations.rs`) : liste les sanctions temporaires actives (`sanction_reminders` pending), triées par proximité d'expiration, avec temps restant formaté (`Xj Yh`, `Xh Ymin`, `X min`). Lecture via `GET /api/reminders/{guild_id}`. 5 tests unitaires sur `format_duration`.

**[MOD #4]** — Confirmation interactive sur cibles à risque :
- **Nouveau module** `bots/moderation-bot/src/risk_check.rs` :
  - `check_target_risk(ctx, guild_id, target)` détecte 3 cas :
    - Compte Discord créé il y a < 7 jours
    - Cible est un bot
    - Cible a un rôle avec `MODERATE_MEMBERS` / `BAN_MEMBERS` / `KICK_MEMBERS` / `ADMINISTRATOR`
  - `RiskyPendingKey` + `DashMap<pending_id, RiskyPending>` pour stocker les actions en attente de confirmation (TTL 300s, purge lazy à chaque access)
  - 1 test unitaire sur `purge_expired`
- **Refactor `/ban`** : extraction de `execute_ban(...)` réutilisable. Avant exécution, call à `check_target_risk` ; si risque détecté, l'action est stockée + réponse ephemeral avec embed critique et 2 boutons Discord (`Confirmer` Danger / `Annuler` Secondary).
- **Button handlers** (`handle_risky_confirm`, `handle_risky_cancel`) dans `handler.rs` : ACK immédiat Discord (<3s), fetch du pending, dispatch sur `PendingKind::Ban` → `execute_ban(...)`, affichage du résultat. `PendingKind::Mute` défini mais pas câblé (wave 2).
- **Scope ban only dans cette wave** : le pattern est prêt à être étendu à `/mute` par simple ajout d'un `execute_mute` + intégration similaire dans `mute.rs`.

#### Wave 2 ✅ (livré)

**[MOD #4 ext]** — Confirmation interactive étendue à `/mute` :
- Refactor `commands/mute.rs` : extraction `execute_mute(...)` avec params (timeout_secs, duration_secs, duration_label, is_permanent), réutilisable depuis le dispatcher de confirmation
- `PendingKind::Mute { timeout_secs }` : la variante `Mute` porte maintenant le `timeout_secs` déjà pré-capé à 28j Discord (calcul fait une seule fois au parsing de la commande)
- Handler `handle_risky_confirm` dans `handler.rs` câble maintenant les 2 variantes (Ban + Mute) via `commands::mute::execute_mute(...)`. `PendingKind::Mute` n'est plus un placeholder.

**[MOD #5]** — `/compare user1 user2` :
- Nouvelle commande `commands/compare.rs` — 2 appels parallèles `api.get_history(...)` via `tokio::join!`, embed avec 3 sections : verdict comparatif ("X a plus de sanctions au total (8 vs 1)" / "même nombre"), bloc historique user1, bloc historique user2
- Validation : les 2 users doivent être différents (reply ephemeral sinon)
- Read-only, pas de nouvel endpoint API (réutilise `/api/moderation/history/{guild_id}/{user_id}` existant)
- 4 tests unitaires sur `build_comparison_line` (h1>h2, h1<h2, égalité) et `format_history_block`

**[MOD #7]** — `/modstats` (métriques par modérateur sur 30j) :
- **Nouvel endpoint API** `GET /api/moderation/modstats/{guild_id}` dans `handlers/moderation.rs` — approche pragmatique direct sqlx (comme `bot_persistence.rs`), bypass du use-case car aggregation simple read-only
- **Query SQL** avec `COUNT(*) FILTER (WHERE action_type = ...)` pour compter par type (`warn`, `mute_temp`/`mute_permanent`/`mute`, `ban_temp`/`ban_permanent`/`ban`, `kick`), `GROUP BY moderator_id`, `ORDER BY total DESC LIMIT 20`, fenêtre `NOW() - INTERVAL '30 days'`
- **Nouveau DTO** `ModStatsEntryDto` (7 champs : `moderator_id`, `moderator_name`, `total`, `warns`, `mutes`, `bans`, `kicks`)
- **Route** ajoutée dans `moderation_routes()` (router.rs)
- **Bot command** `commands/modstats.rs` — pas de paramètre, ephemeral, embed "Top 20" avec médailles 🥇🥈🥉 pour les 3 premiers, 🔸 pour les suivants. `format_modstats` testée (empty list, single entry, top-three medals).
- 3 tests unitaires sur `format_modstats`

#### Wave 3 ✅ (livré)

**[MOD #2]** — `/evidence add|list` :
- **Migration `107_create_evidence.sql`** : nouvelle table `moderation_evidence` (FK `action_id` → `moderation_actions` avec `ON DELETE CASCADE`, `url` TEXT, `description` TEXT optionnel, `uploaded_by` VARCHAR(20), `uploaded_by_name`, `uploaded_at`). Index sur `action_id` + `uploaded_at DESC`.
- **API** : 2 nouveaux handlers dans `moderation.rs` (direct sqlx, pas de use-case) :
  - `POST /api/moderation/evidence` : valide `action_id` UUID, `url` non vide (max 2000), `uploaded_by` Discord ID. Description tronquée à 500 chars.
  - `GET /api/moderation/evidence/{action_id}` : retourne la liste triée par `uploaded_at ASC`
- **Bot command** `commands/evidence.rs` : 2 sub-commands `add` et `list`, ephemeral. Embed avec `short_id` (8 premiers chars de l'UUID) pour la lisibilité.
- **`api_client::add_evidence` + `list_evidence`** + nouveau DTO `EvidenceEntry`.
- 2 tests unitaires sur `short_id`.

**[MOD #3]** — `/review add|list|resolve` :
- **Migration `108_create_review_queue.sql`** : table `review_queue` (FK `action_id`, `guild_id`, `added_by`, `reason`, `status` avec `CHECK` sur `'pending'|'approved'|'rejected'|'changed'`, `reviewer_*`, `added_at`, `resolved_at`). Index partiel `WHERE status='pending'` pour accélérer le listing.
- **API** : 3 nouveaux handlers :
  - `POST /api/moderation/review` : ajoute en status `'pending'`
  - `GET /api/moderation/review/{guild_id}/pending` : liste **avec JOIN sur `moderation_actions`** pour enrichir chaque entrée avec `action_type`, `target_name`, `action_reason` (évite 50 appels follow-up côté bot)
  - `PATCH /api/moderation/review/{id}/resolve` : marque comme résolue, garde `WHERE status='pending'` pour idempotence
- **Bot command** `commands/review.rs` : 3 sub-commands (`add`, `list`, `resolve`) avec choix Discord sur le statut. `resolve` fire-and-forget via `patch_fire_and_forget`.
- **`api_client`** : `add_review` / `list_pending_reviews` / `resolve_review` + DTO `ReviewQueueEntry`
- 1 test unitaire sur `short_id`

#### Résumé Phase 6B livré (6/8 features)

| Feature | Statut | Wave |
|---|---|---|
| MOD #1 Rappels & expirations | ✅ | 1 |
| MOD #2 `/evidence` | ✅ | 3 |
| MOD #3 `/review` | ✅ | 3 |
| MOD #4 Confirmation cibles à risque (/ban + /mute) | ✅ | 1 + 2 |
| MOD #5 `/compare` | ✅ | 2 |
| MOD #6 Templates de sanction | 🟡 mostly done (polish différé) | — |
| MOD #7 `/modstats` | ✅ | 2 |
| MOD #8 Transcript call rooms | ⏸️ bloqué (dépend export-worker) | — |

#### MOD #6 polish ✅ (livré)

**Commande `/template`** — gestion des reason templates depuis Discord (plus besoin de passer par la GUI desktop) :

- `/template list` — affiche les templates actuels (embed ephemeral)
- `/template add <label> <reason>` — ajoute un template (empêche les doublons par label, rejette `|` et `\n` dans les inputs)
- `/template remove <label>` — supprime par label exact (case-insensitive)

**Architecture** :
- `commands/template.rs` (nouveau fichier, 3 sub-commands + helpers `load_templates`/`save_templates`/`serialize_templates`)
- Nouvelle méthode `ApiClient::set_bot_config` dans `moderation-bot/api_client.rs` — fire-and-forget sur `POST /api/bots/config`
- Format serialization identique à `reason_templates::parse_templates` : `label|reason\n...`
- `default_member_permissions(Administrator)` côté Discord pour gater l'accès — le bot appelle l'API sans `X-Discord-Token` donc pass-through RBAC
- 3 tests unitaires sur `serialize_templates` (empty, single, roundtrip avec `parse_templates`)

#### MOD #8 transcript ✅ (livré)

Commande `/transcript channel:<salon>` dans moderation-bot — génère un
transcript texte des 100 derniers messages d'un salon et l'envoie en
pièce jointe ephemeral.

**Décision de scope** : implémenté **bot-side direct** via serenity, pas
via export-worker. Justification :
- Le bot a déjà accès Discord Gateway+HTTP, fetch en 1 appel
- Passer par export-worker = round-trip inutile (DB insert + poll + DB fetch)
- Le pattern export-worker reste dispo pour exports massifs DB-based
- Si besoin ultérieur (full channel history avec pagination, auto-capture
  post-call), migration possible vers le worker sans breaking change
  cote bot (juste ajouter `job_type='call_transcript'` dans la migration
  110 via ALTER CHECK)

**Architecture** :
- `commands/transcript.rs` — fetch via `channel_id.messages(http, GetMessages::limit(100))`
- Format texte simple : `[YYYY-MM-DD HH:MM:SS] Auteur: contenu`
- Annotations `[+N piece(s) jointe(s)]` et `[+N embed(s)]` si applicable
- `CreateAttachment::bytes(..., "transcript-{channel_id}-{timestamp}.txt")`
- **Defer immédiat** (Discord timeout 3s) avant fetch puis `create_followup`
- Gated `MODERATE_MEMBERS` Discord-side
- 3 tests unitaires sur `count_lines_hint`

**Limites volontaires MVP** :
- Max 100 messages (1 appel, pas de pagination — suffisant pour la
  plupart des call rooms < 50 messages)
- Attachments/embeds Discord ignorés (juste le placeholder)
- Taille max : 10 MB Discord free / 25 MB boost 2 (100 messages texte
  font typiquement < 50 KB, largement OK)

#### Différés

**Plus aucun différé sur Phase 6B features moderation** — 8/8 livrées.

#### Validation wave 1

- `cargo check` clean (warning intentionnel sur `PendingKind::Mute` — wave 2)
- `cargo test --bin moderation-bot` : 31/31 (30 existants + 5 `format_duration` + 1 `purge_expired` — sous le même binaire).

#### Validation wave 2

- `cargo check` clean sur `services/api` ET `bots/moderation-bot`
- `cargo test --bin moderation-bot` : **38/38** (wave 1 + 4 tests `compare` + 3 tests `modstats`)
- `cargo test --lib` API : **216/216** (hors tests ML ONNX non-gated)
- Commandes Discord désormais exposées : `/warn`, `/mute`, `/unmute`, `/ban`, `/unban`, `/history`, `/notes`, `/call`, `/context`, `/appeal`, `/export`, `/expirations`, `/compare`, `/modstats`, `/massmute`, `/massban` (15 commandes au total)

#### Validation wave 3

- `cargo check` clean sur `services/api` (migrations 107 + 108 compilées) ET `bots/moderation-bot`
- `cargo test --bin moderation-bot` : **41/41** (wave 2 + 2 tests `evidence::short_id` + 1 test `review::short_id`)
- `cargo test --lib` API : **216/216**
- Commandes Discord finales : wave 2 + `/evidence` + `/review` = **17 commandes au total**
- Tables ajoutées : `moderation_evidence` (FK CASCADE sur `moderation_actions`) + `review_queue` (FK CASCADE + CHECK constraint sur `status`)
- Pattern employé : direct sqlx dans handlers API (pas de use-case), approche pragmatique comme `bot_persistence.rs` — justifié par le scope simple read-write sans logique métier complexe

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

### Partie B — RBAC fin ✅ **TERMINÉE**

> ✅ **Livré** : 2 tables RBAC + middleware `rbac_middleware` + helper
> `require_role` + hiérarchie `Role` + 11 tests unitaires.
> ⏸️ **Différé** : UI desktop de gestion (scope frontend, session dédiée).

#### Migration 109 — 2 tables (pas 3)

Le design initial de la roadmap mentionnait 3 tables (`api_users`, `api_user_guilds`, `roles`) mais un `CHECK` constraint sur 4 valeurs statiques est plus simple qu'une table `roles` dédiée. Choix pragmatique.

```sql
CREATE TABLE api_users (
    discord_user_id VARCHAR(20) PRIMARY KEY,
    display_name TEXT NOT NULL,
    avatar_url TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_seen_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE api_user_guilds (
    discord_user_id VARCHAR(20) NOT NULL REFERENCES api_users(discord_user_id) ON DELETE CASCADE,
    guild_id VARCHAR(20) NOT NULL,
    role TEXT NOT NULL CHECK (role IN ('owner', 'admin', 'moderator', 'viewer')),
    granted_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    granted_by VARCHAR(20),
    PRIMARY KEY (discord_user_id, guild_id)
);
CREATE INDEX idx_api_user_guilds_guild ON api_user_guilds (guild_id, role);
```

#### Hiérarchie des rôles

| Rôle | Numérique | Permissions |
|---|---|---|
| `owner` | 3 | Full access + gestion RBAC (ajouter/retirer des rôles) |
| `admin` | 2 | Full CRUD, hors RBAC |
| `moderator` | 1 | Sanctions, tickets, notes, lecture |
| `viewer` | 0 | Read-only |

L'`enum Role` Rust dérive `PartialOrd` sur les discriminants — l'opérateur `>=` donne directement la vérification "rôle au moins X".

#### Nouveau middleware `rbac_middleware`

- **Emplacement** : `services/api/src/adapters/inbound/http/middleware/rbac.rs`
- **Ordre** : s'exécute APRÈS `guild_auth_middleware` (Phase 2B) — il réutilise le même header `X-Discord-Token`. Le router chain est désormais : `rate_limit` → `auth` → `guild_auth` → `rbac`.
- **Flow** :
  1. Pass-through si `X-Discord-Token` absent (bot/internal)
  2. Fetch `user_id` via cache Redis `user_id:<token_hash>` (TTL 10 min) ou fallback `GET /users/@me`
  3. Upsert dans `api_users` (best-effort, ne bloque pas sur erreur DB)
  4. Extrait `guild_id` de l'URI via l'heuristique snowflake (17-20 chiffres)
  5. Lookup `api_user_guilds WHERE (discord_user_id, guild_id)`
  6. **Fallback** : si pas de row mais le user est dans la guild Discord (guild_auth l'a déjà validé), rôle par défaut = `viewer` (POLA)
  7. Stocke `RoleContext { discord_user_id, role, guild_id }` dans les extensions de la requête

#### Nouvelle méthode `DiscordApiService::get_user_me`

- Appel `GET /users/@me` avec `Authorization: Bearer <access_token>`
- Scope OAuth2 requis : `identify`
- Retourne `DiscordUser { id, username, avatar }`

#### Helper `require_role` pour les handlers

```rust
use axum::Extension;
use crate::adapters::inbound::http::middleware::rbac::{Role, RoleContext, require_role};

pub async fn delete_config(
    Extension(ctx): Extension<RoleContext>,
    // ...
) -> Result<Json<...>, ApiError> {
    require_role(&ctx, Role::Admin)?;  // 403 si < Admin
    // ... logique admin-only
}
```

**État actuel** : le middleware injecte `RoleContext` pour TOUS les handlers, mais aucun handler existant n'appelle encore `require_role`. C'est volontaire — migration progressive, chaque handler peut ajouter la gate quand nécessaire sans changement breaking.

#### Bootstrap

**Pas d'auto-promote**. Les premiers `owner` doivent être seedés en SQL direct au déploiement initial :

```sql
INSERT INTO api_users (discord_user_id, display_name)
VALUES ('123456789012345678', 'Alice');
INSERT INTO api_user_guilds (discord_user_id, guild_id, role)
VALUES ('123456789012345678', '987654321098765432', 'owner');
```

Éviter l'auto-promote sur premier login sinon n'importe qui peut prendre le contrôle d'une guild nouvellement onboardée.

#### Tests

- 11 tests unitaires (`adapters::inbound::http::middleware::rbac::tests`) :
  - Ordering hiérarchique (`Owner > Admin > Moderator > Viewer`)
  - `satisfies` (own level + higher + lower)
  - `from_str` valid / invalid / roundtrip
  - `require_role` accepts equal / higher, rejects lower / no role
  - `extract_guild_id_from_path` avec / sans snowflake
- Total API : **227/227** (216 + 11 RBAC, hors ML ONNX non-gated)

#### Endpoints CRUD RBAC ✅ **livrés** (follow-up immédiat)

Nouveau handler `handlers/rbac.rs` avec 5 endpoints direct-sqlx, tous gated via `require_role` :

| Endpoint | Gate | Description |
|---|---|---|
| `POST /api/rbac/guilds/{guild_id}/users/{user_id}` | `Owner` | Grant un rôle (body: `{role, display_name?}`) — upsert `api_users` + insert `api_user_guilds`, 409 si doublon |
| `PATCH /api/rbac/guilds/{guild_id}/users/{user_id}` | `Owner` | Update le rôle (body: `{role}`) — **garde-fou lockout** : un owner ne peut pas se rétrograder lui-même |
| `DELETE /api/rbac/guilds/{guild_id}/users/{user_id}` | `Owner` | Revoke — **garde-fou dernier owner** : refus si le target est le dernier owner de la guild |
| `GET /api/rbac/guilds/{guild_id}/users` | `Admin+` | Liste JOIN avec `api_users`, triée par rôle (owner → viewer) puis nom |
| `GET /api/rbac/me/{guild_id}` | aucun (tout rôle) | Retourne le rôle effectif du caller — utilisé par le desktop pour savoir quoi afficher/masquer |

**Ajouts infrastructure** :
- `DomainError::Forbidden(String)` → mapping HTTP 403 dans `errors.rs`
- `require_role` maintenant utilisé pour la première fois en production (gate les 4 endpoints d'écriture RBAC)
- Pattern validé : le helper retourne `StatusCode::FORBIDDEN`, converti en `DomainError::Forbidden` via `status_to_err`

**Bootstrap initial toujours manuel SQL** (un owner doit exister pour que les endpoints soient utilisables), mais le flow suivant est automatisé via les endpoints.

#### Gates progressifs wave 1 ✅ (livré)

Premier batch de handlers destructifs gatés via `require_role` avec le pattern
`Option<Extension<RoleContext>>` pour préserver le pass-through bot/internal :

| Handler | Gate | Path |
|---|---|---|
| `rules::delete_rule` | Admin | `DELETE /rules/{guild_id}/{rule_id}` |
| `discord_roles::delete_role` | Admin | `DELETE /api/discord-roles/{guild_id}/{role_id}` |
| `levels::delete_reward` | Admin | `DELETE /rewards/{guild_id}/{level}` |
| `watched_users::remove_watched_user` | Moderator | `DELETE /api/watched-users/{guild_id}/{user_id}` |

**Pattern clé** : `rbac: Option<Extension<RoleContext>>`
- **Présent** (appel desktop avec `X-Discord-Token`) → check `require_role` enforcé
- **Absent** (appel bot/worker/internal avec API key uniquement) → pass-through non-breaking
- Retour 403 `Forbidden` si rôle insuffisant

#### Gates progressifs wave 2 ✅ (livré)

**Nouveau helper `require_role_for_guild`** dans `rbac.rs` :
- Signature async, prend `state` + `ctx` + `guild_id` explicite + `required`
- Fait un `lookup_role` DB direct (le middleware ne peut pas extraire le guild du body)
- Fallback `Viewer` identique au middleware si erreur DB (fail-safe)

**5 handlers gatés** (mix path-based et body-based) :

| Handler | Gate | Helper utilisé | Raison |
|---|---|---|---|
| `bot_config::set_config` | Admin | `require_role_for_guild` (body) | Config bot = policy |
| `bot_config::delete_config` | Admin | `require_role_for_guild` (body) | Idem |
| `purge::purge_infractions` | **Owner** | `require_role_for_guild` (body) | Bulk delete = danger max |
| `purge::purge_audit_logs` | **Owner** | `require_role_for_guild` (body) | Idem |
| `strikes::reset_strikes` | Moderator | `require_role` (path) | Reset user strikes |
| `guild_members::remove_member` | Moderator | `require_role` (path) | Cache local removal |

**Cas spécial `purge::purge_logs`** : endpoint **global** (pas scoped par guild). Documenté comme nécessitant un concept "superadmin" futur. Pour l'instant : le `rbac: Option<Extension>` est récupéré mais pas checké — si desktop (token présent), l'appel passe car le middleware guild_auth/rbac l'a déjà validé comme user legit ; si bot (pas de token), pass-through comme les autres handlers.

#### Gates progressifs wave 3 ✅ (livré)

5 handlers path-based supplémentaires gatés :

| Handler | Gate | Path |
|---|---|---|
| `voice_channels::delete_theme` | Admin | `DELETE /themes/{guild_id}/{theme_id}` |
| `voice_channels::remove_from_whitelist` | Moderator | `DELETE /whitelist/{guild_id}/{owner_id}/{target_id}` |
| `role_panels::delete_auto_role` | Admin | `DELETE /role-panels/auto-roles/{guild_id}/{role_id}` |
| `games::delete_game` | Admin | `DELETE /games/{guild_id}/{game_id}` |
| `bot_persistence::delete_temp_role` | Moderator | `DELETE /api/temp-roles/{guild_id}/{user_id}/{role_id}` |

**Note sur `delete_temp_role`** : cet endpoint est appelé à la fois depuis le desktop ET depuis `community-bot` (qui consume l'event `temp_role_expire` de Phase 4 B). Le pattern `Option<Extension<RoleContext>>` garantit que le bot (pas de `X-Discord-Token`) continue en pass-through, tandis que les appels desktop sont gatés à Moderator+.

#### État de la couverture RBAC

**Handlers gatés (14 au total)** :

*Admin+* (8) :
- `rules::delete_rule`
- `discord_roles::delete_role`
- `levels::delete_reward`
- `bot_config::set_config`
- `bot_config::delete_config`
- `voice_channels::delete_theme`
- `role_panels::delete_auto_role`
- `games::delete_game`

*Owner+* (2) :
- `purge::purge_infractions`
- `purge::purge_audit_logs`

*Moderator+* (5) :
- `watched_users::remove_watched_user`
- `strikes::reset_strikes`
- `guild_members::remove_member`
- `voice_channels::remove_from_whitelist`
- `bot_persistence::delete_temp_role`

#### Gates progressifs wave 4 ✅ (livré)

**Pattern "ressource-id-based"** : pour les handlers dont le path n'a que l'ID de ressource (pas le `guild_id`), on fetch d'abord le `guild_id` via sqlx direct, puis on appelle `require_role_for_guild`. Pas de nouveau helper — le pattern est suffisamment simple pour rester inline (3-5 lignes par handler).

3 handlers gatés :

| Handler | Gate | Fetch source |
|---|---|---|
| `infractions::delete_infraction` | Moderator | `find_by_id` (use-case existant) |
| `notes::delete_note` | Moderator | `SELECT guild_id FROM user_notes WHERE id = $1` (sqlx direct) |
| `coude::cancel_combat` | Moderator | `SELECT guild_id FROM coude_combats WHERE id = $1` (sqlx direct) |

**Note design** : pour `infractions::delete_infraction`, l'appel `find_by_id` existait déjà (utilisé pour envoyer le DM après suppression). On en profite pour extraire le `guild_id` sans double round-trip DB. Pour `notes` et `coude`, un round-trip supplémentaire est acceptable (opération destructive peu fréquente).

**Fail-open safe** : si la ressource n'existe pas (fetch retourne None), on laisse le handler delegate à son use-case qui retournera 404 NotFound — on ne masque pas l'erreur avec un 403 Forbidden trompeur.

#### État final de la couverture RBAC

**17 handlers gatés** au total :

*Admin+* (8) :
- `rules::delete_rule`, `discord_roles::delete_role`, `levels::delete_reward`
- `bot_config::set_config`, `bot_config::delete_config` (body-based)
- `voice_channels::delete_theme`, `role_panels::delete_auto_role`, `games::delete_game`

*Owner+* (2) :
- `purge::purge_infractions`, `purge::purge_audit_logs` (body-based)

*Moderator+* (7) :
- `watched_users::remove_watched_user`, `strikes::reset_strikes`, `guild_members::remove_member`
- `voice_channels::remove_from_whitelist`, `bot_persistence::delete_temp_role`
- `infractions::delete_infraction`, `notes::delete_note`, `coude::cancel_combat` (resource-id-based)

#### Gates progressifs wave 5 ✅ (livré — clôture de la couverture RBAC)

**6 handlers** gatés, tous pattern ressource-id-based :

| Handler | Gate | Fetch source |
|---|---|---|
| `blackjack::close_table` | Moderator | `SELECT guild_id FROM blackjack_tables WHERE id = $1::uuid` |
| `voice_channels::delete_channel` | Moderator | `SELECT guild_id FROM voice_channels WHERE channel_id = $1` |
| `voice_channels::remove_co_admin` | Moderator | idem |
| `voice_channels::unban_from_channel` | Moderator | idem |
| `voice_channels::revoke_invite_link` | Moderator | idem |
| `role_panels::delete_panel` | Admin | `SELECT guild_id FROM role_panels WHERE id = $1` |

**Nouveau helper privé `gate_by_channel_id`** dans `voice_channels.rs` : wrap le fetch + `require_role_for_guild` pour les 4 handlers voice qui partagent le même pattern (voice channel → fetch guild → check). Pas exporté — spécifique au module, réduit la duplication inline sans créer une abstraction publique prématurée.

#### État final de la couverture RBAC (clôturée)

**23 handlers destructifs gatés** — couverture complète des endpoints path-based et ressource-id-based :

*Admin+* (9) :
- `rules::delete_rule`, `discord_roles::delete_role`, `levels::delete_reward`
- `bot_config::set_config`, `bot_config::delete_config` (body)
- `voice_channels::delete_theme`, `role_panels::delete_auto_role`, `games::delete_game`
- `role_panels::delete_panel` (resource-id)

*Owner+* (2) :
- `purge::purge_infractions`, `purge::purge_audit_logs` (body)

*Moderator+* (12) :
- `watched_users::remove_watched_user`, `strikes::reset_strikes`, `guild_members::remove_member`
- `voice_channels::remove_from_whitelist`, `bot_persistence::delete_temp_role`
- `infractions::delete_infraction`, `notes::delete_note`, `coude::cancel_combat`
- `blackjack::close_table`
- `voice_channels::delete_channel`, `remove_co_admin`, `unban_from_channel`, `revoke_invite_link`

**Concept superadmin ✅ livré** — `/purge/logs` est maintenant gaté :
- Nouvelle variable env `SUPERADMIN_USER_IDS` (liste comma-separated de Discord user IDs)
- Exposée via `AppConfig::superadmin_user_ids` → `AppState::superadmin_user_ids: Arc<Vec<String>>`
- Nouveau helper `require_superadmin(state, ctx)` dans `rbac.rs` — check contre la liste statique
- Si la liste est vide, TOUS les appels sont refusés par sécurité (fail-closed volontaire)
- Gate appliqué sur `purge::purge_logs` — les appels bot/internal sans token restent en pass-through, les appels desktop avec token doivent être dans la liste superadmin

**3 patterns RBAC établis et documentés** :
1. **Path-based** (guild_id dans l'URL) → `require_role(&ctx, Role::X)` — extraction automatique par middleware
2. **Body-based** (guild_id dans le JSON body) → `require_role_for_guild(&state, &ctx, &dto.guild_id, Role::X)` — lookup DB explicite
3. **Ressource-id-based** (id de ressource sans guild_id) → fetch `SELECT guild_id FROM {table}` puis `require_role_for_guild` (inline, 3-5 lignes)

**Phase 7 B définitivement terminée.** Reste uniquement l'UI desktop RBAC (scope frontend) pour clore le cycle complet de gestion RBAC par un owner.

#### Différés restants

- **UI desktop RBAC** : pages pour `owner`/`admin` qui consomment les 5 endpoints CRUD (lister, grant, update, revoke). Scope frontend Vue/Tauri, session dédiée.
- **Gates wave 2** : `bot_config` writes, `purge/*`, `strikes::reset_strikes`, `guild_members::remove_member`, `role_panels::delete_panel`, `games::delete_game`, `bot_persistence::delete_temp_role`, `blackjack::close_table`. **Contrainte** : les endpoints qui lisent `guild_id` depuis le **body** (`bot_config`, `purge/*`) nécessitent un helper `require_role_for_guild(state, ctx, guild_id, role)` qui fait un lookup DB explicite (le middleware ne voit pas les bodies). À ajouter avant de gater ce groupe.
- **Handlers avec `id` mais sans `guild_id` dans le path** (`/infractions/{id}`, `/notes/{id}`, `/blackjack/tables/{table_id}`) : même contrainte — soit restructurer l'URL pour inclure `guild_id`, soit fetch d'abord la ressource pour récupérer son guild, soit ajouter un helper `require_role_for_resource(state, ctx, resource_id, role)`.

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
