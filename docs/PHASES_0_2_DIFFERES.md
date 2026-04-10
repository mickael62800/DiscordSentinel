# Phases 0 → 2 — Tâches non faites ou partiellement faites

Ce document liste, phase par phase, **ce qui n'a pas été livré** et **pourquoi**, pour que tu puisses décider quoi reprendre dans une itération future.

---

## Phase 0 — Observabilité ✅ **TERMINÉE intégralement**

Aucune tâche skippée. Tout le scope ROADMAP a été livré :

- `pg_stat_statements` activé (docker-compose + migration 099)
- Middleware Prometheus sur l'API (`http_requests_total`, `http_request_duration_seconds` avec `MatchedPath`)
- Endpoint `/metrics`
- `tokio-metrics` sampler sur API + 6 workers (`init_observability` partagé)
- Stack Grafana + Prometheus (`docker-compose --profile monitoring`)
- Dashboard `infra/grafana/dashboards/sentinel-baseline.json`
- Template `docs/BASELINE_METRICS.md`

**Seule limitation assumée** : les bots Discord ne sont pas instrumentés. Justification documentée dans la ROADMAP : ils sont idle la plupart du temps, l'instrumentation aurait peu de valeur. Si on a un jour un problème de perf bot, c'est facile à ajouter (le helper `init_observability` est déjà dans `worker-common`).

---

## Phase 1 — Quick wins zéro-risque ✅ **TERMINÉE (avec 1 différé mineur)**

### ❌ Compression payload Redis

**Ce qui n'a pas été fait** : compresser les valeurs stockées dans Redis (zstd ou lz4) au moment du `SET`/`GET`.

**Pourquoi différé** :
- Beaucoup plus invasif que prévu : il faut modifier **toutes** les opérations Redis dans le code applicatif (API + cache-worker + workers de cache à venir) pour wrapper en compress/decompress.
- Pas un changement « quick win » : ça touche tout le path Redis, donc régression possible partout.
- ROI faible **avant Phase 5** (cache-aside systématique). Tant que Redis ne stocke que ~quelques MB, la bande passante locale Docker n'est pas un bottleneck.

**Quand le reprendre** : en Phase 5, en même temps que la mise en place de `cached_read<T>` — comme ça on a un seul point d'instrumentation (le wrapper `cached_read` fait compress + serde) au lieu de patcher 50 callsites.

---

## Phase 2 — Fondations DB + sécurité multi-tenant ⚠️ **PARTIELLE**

C'est la phase avec le plus de différés. 6 sous-parties sur 7 sont livrées intégralement, mais à l'intérieur de **A.3**, **A.4** et **A.5** certaines sous-tâches ont été volontairement skippées ou réduites.

### A.1 — Quick wins schéma DB ✅ **complet**

Rien skippé. Migration 100 + 101 livrent tout : index dupliqués droppés, partials soft-delete, GIN sur JSONB, colonnes mortes, conversion `TEXT → VARCHAR(20)` en idempotent.

---

### A.2 — Vues matérialisées + user_cache ✅ **complet, mais avec un écart d'architecture**

#### ⚠️ Pas de `user-cache-worker` séparé

**Ce que la ROADMAP prévoyait** : créer un nouveau crate `user-cache-worker` dédié à la synchro des usernames Discord.

**Ce que j'ai fait à la place** : ajouté **deux jobs** au `cache-worker` existant :
- `refresh_leaderboards` (5 min) — refresh CONCURRENTLY des MV
- `sync_user_cache` (15 min) — agrège les usernames depuis 4 tables hot via `INSERT INTO user_cache ... SELECT DISTINCT ON (...) ... ON CONFLICT ... DO UPDATE ... WHERE updated_at < EXCLUDED.updated_at`

**Pourquoi cet écart** :
- Créer un nouveau crate ajoute du bagage (Cargo.toml, Dockerfile, entrée docker-compose, démarrage indépendant) pour ~100 lignes de logique métier.
- Les deux jobs sont périodiques et non-bloquants → ils cohabitent sans conflit dans cache-worker.
- Plus simple à monitorer (un seul container).

**Si tu veux vraiment un crate dédié** : c'est une extraction triviale (copier le pattern `temp-roles-worker`), donc reprenable à tout moment.

---

### A.3 — Breaking changes ⚠️ **2 sur 4 sous-changements faits**

#### ✅ Fait

1. **Enums Postgres** (`coude_class`, `moderation_gravity`, `voice_channel_kind`) avec wrappers Rust `#[derive(sqlx::Type)]`. Les entités, repos et DTOs ont été migrés.
2. **`discord_roles.permissions` `TEXT → BIGINT`** avec conversion défensive et DTO HTTP qui sérialise en `String` pour la safety JS (`Number.MAX_SAFE_INTEGER`).

#### ❌ Différé : `bot_guild_config.config_value TEXT → JSONB`

**Pourquoi** :
- Audit du codebase → **23 fichiers callers** dans les bots qui consomment cette config via `BaseApiClient::get_guild_config()` qui retourne actuellement `HashMap<String, String>`.
- Le passage en JSONB casse cette signature : soit on retourne `HashMap<String, serde_json::Value>` (refonte de 23 fichiers), soit on garde `String` côté API en re-sérialisant (mais alors le gain JSONB est nul côté queries).
- Le **gain réel court terme est marginal** : la table est petite (centaines de lignes), pas de query analytique sur `config_value`. Le seul gain serait la possibilité d'indexer en GIN + de faire des queries `@>` sur les configs, ce dont personne n'a besoin aujourd'hui.

**Comment le reprendre proprement** :
1. Faire le changement en deux temps : d'abord ajouter une colonne `config_value_json JSONB` à côté de la `TEXT`, doublement écrire pendant une période de transition.
2. Migrer les 23 callsites bots un par un.
3. Une fois tous migrés, dropper la colonne `TEXT`.

C'est un travail de **0.5 à 1 jour IA** mais avec un **blast radius énorme** : ça mérite une session dédiée et une review attentive.

#### ❌ Différé : contraintes `NOT NULL` et `CHECK`

**Pourquoi** :
- L'audit a trouvé 15+ colonnes texte avec `DEFAULT ''` au lieu de `NOT NULL` strict (ex : `user_stats.username`, `user_wallets.username`, `voice_sessions.channel_name`, etc.).
- Ajouter `SET NOT NULL` sur des données existantes peut **échouer au runtime de la migration** si une seule ligne contient `NULL` ou `''`. Risque de migration qui rollback en cas d'incident.
- Il faut d'abord **auditer manuellement** les données existantes en prod (`SELECT COUNT(*) WHERE col IS NULL`) avant d'oser les contraindre.

**Comment le reprendre** :
1. Sur l'environnement prod, lancer les `SELECT COUNT(*) WHERE col IS NULL` pour chaque colonne candidate.
2. Backfill les valeurs manquantes (`UPDATE ... SET col = '' WHERE col IS NULL`).
3. Puis `ALTER TABLE ... ALTER COLUMN col SET NOT NULL` une fois sûr.

C'est essentiellement du **travail manuel d'audit data**, pas du code. À faire avec accès à la prod.

#### ❌ Non fait : `infractions.action TEXT → ENUM`

**Pourquoi** :
- Le code Rust a **déjà** un enum `Action` (`domain/value_objects/action.rs`) avec `from_str_lossy()`.
- L'ajouter en ENUM Postgres apporterait juste une validation au niveau insert, mais le code l'a déjà.
- ROI **vraiment marginal** par rapport au coût (renommer la migration, drop/recreate la table partitionnée 104, etc.).

---

### A.4 — Partitionnement ⚠️ **4 tables sur 9 partitionnées**

#### ✅ Partitionné

`infractions`, `audit_logs`, `user_activity_log`, `logs` — les 4 tables hot avec le plus de gain temporel.

#### ❌ Non partitionné

| Table | Raison |
|---|---|
| `moderation_actions` | Volume **beaucoup plus faible** que les 4 hot. Peu de queries `WHERE created_at BETWEEN ...`. Gain marginal vs effort de migration. |
| `security_events` | Idem moderation_actions, faible volume. |
| `daily_activity` | **Pas de colonne `created_at`** — utilise `day DATE` comme clé naturelle. Le partitionner nécessiterait soit d'ajouter `created_at`, soit de partitionner sur `day`. Pas de gain car la table est déjà naturellement granulaire (1 ligne par guild × jour). |
| `hourly_activity` | Même cas que `daily_activity` (clé `(day, hour)`, pas de timestamp). |
| `coude_casino_log` | **PK `BIGSERIAL`** incompatible avec partitionnement direct (séquences ne sont pas partition-aware). Refactorer en UUID + DEFAULT serait coûteux pour une table à faible volume. |

**Comment reprendre** :
- Pour `moderation_actions` et `security_events` : c'est juste la même recette que les 4 déjà partitionnées (RENAME → CREATE partitionné → INSERT SELECT → DROP). 30 min IA.
- Pour `daily_activity`/`hourly_activity` : décider d'abord si on **a vraiment besoin** de partitionner ces tables agrégées. Probablement non.
- Pour `coude_casino_log` : refactor BIGSERIAL → UUID = 1h IA + adaptation des callers Rust.

---

### A.5 — Tuning RAM Postgres/Redis ⚠️ **9 étapes sur 10**

#### ✅ Fait

Les 8 premières étapes sont dans `docker-compose.yml` :
- Postgres : `shared_buffers=4GB`, `effective_cache_size=10GB`, `work_mem=64MB`, `maintenance_work_mem=1GB`, `wal_buffers=16MB`, `checkpoint_completion_target=0.9`, `max_wal_size=4GB`, `min_wal_size=1GB`
- Redis : `maxmemory=2gb`, `maxmemory-policy=allkeys-lru`

Les étapes 6 (Huge Pages Linux) et 7 (swappiness) sont **du sysctl host**, pas du Docker — à faire manuellement sur le serveur prod, pas pertinent en local.

#### ❌ Différé : Étape 9 — Cache Rust in-memory `moka` dans l'API

**Ce que la ROADMAP prévoyait** :
- Ajouter `moka = { version = "0.12", features = ["future"] }` à `services/api/Cargo.toml`
- Créer `services/api/src/adapters/outbound/cache/moka_cache.rs`
- Wrapper sur les repositories hot : `GuildConfigRepository` (TTL 5min), `PermissionsRepository` (TTL 2min), `UserProfileRepository` (TTL 10min)
- Invalidation via Redis pub/sub lors des writes

**Pourquoi différé** :
- C'est du **code applicatif Rust non-trivial** (~300-500 lignes), pas du tuning de config.
- Nécessite de revoir l'invalidation : si une write part du même process API, l'invalidation in-memory est triviale, mais si elle vient d'un worker, il faut absolument le pub/sub Redis.
- Le **vrai pattern** est intriqué avec **Phase 5** (cache-aside systématique avec helper `cached_read<T>`). Faire moka maintenant puis le re-câbler en Phase 5 = double travail.
- Sans Phase 5, on n'a pas non plus identifié les endpoints hot via `pg_stat_statements` réel (Phase 0 a posé l'outil mais pas encore capturé de baseline en charge).

**Comment le reprendre** : à intégrer dans **Phase 5 partie A** (« Cache-aside systématique »). À ce moment-là, on aura :
1. Une baseline `pg_stat_statements` réelle pour cibler les bons endpoints
2. Un helper unique `cached_read<T>` qui combine moka L1 + Redis L2 + pub/sub d'invalidation
3. Une seule passe de modification des repos au lieu de deux

---

### A.6 — PgBouncer ✅ **complet**

Rien skippé. Image `edoburu/pgbouncer` ajoutée, mode `transaction`, `pool_size=25`, `max_client_conn=1000`, `max_prepared_statements=100` (compat sqlx), tous les services repointés sur `pgbouncer:5432`.

⚠️ **À surveiller en prod** : si sqlx fait des prepared statements inhabituels, le `max_prepared_statements=100` peut être trop bas. À ajuster sur retour terrain.

---

### B — Multi-tenant ✅ **complet**

Rien skippé du scope ROADMAP. Middleware `guild_auth_middleware`, cache Redis 5 min, `DiscordApiService.get_user_guilds`, propagation du token côté desktop via `ApiAdapter::set_discord_token` après OAuth2.

⚠️ **Limites assumées documentées** :
- Le middleware est **pass-through si `X-Discord-Token` est absent** — c'est volontaire pour ne pas casser les appels bots/internes qui n'ont pas de session OAuth2. Un attaquant qui aurait l'API key bot pourrait donc bypasser le filtrage en omettant le header. Mitigation : la validation `auth_middleware` (Bearer API key) reste obligatoire devant.
- Le hash du token Discord pour la clé Redis utilise `DefaultHasher` (non-cryptographique). C'est suffisant pour éviter de stocker le token en clair en mémoire Redis, mais ça ne le « protège » pas. Le token reste sensible et passe via TLS sur HTTPS en prod.
- **Pas de tests E2E** du middleware — seulement 5 tests unitaires sur l'extraction du `guild_id` et le hash. Un test bout-en-bout avec un mock Discord API serait pertinent.

---

## Synthèse — Décisions clés à prendre

| Item | Phase | Effort estimé | Pré-requis | Priorité |
|---|---|---|---|---|
| Compression payload Redis | 1 | 0.5j IA | Phase 5 cache-aside en cours | basse (faire avec Phase 5) |
| `bot_guild_config TEXT → JSONB` | 2.A.3 | 1j IA | Décision sur le format JSON cible | basse (gain marginal) |
| `NOT NULL` / `CHECK` constraints | 2.A.3 | 0.5j manuel + 0.5j IA | Audit data prod | moyenne (qualité long terme) |
| Partitionner `moderation_actions` + `security_events` | 2.A.4 | 0.5j IA | Aucun | basse (faible volume) |
| Cache moka in-memory dans l'API | 2.A.5 | 1j IA | Phase 5 + baseline pg_stat_statements réelle | **moyenne (vrai gain perf)** |
| Tests E2E du middleware multi-tenant | 2.B | 0.5j IA | Mock Discord API | moyenne (sécurité) |
| `user-cache-worker` séparé | 2.A.2 | 0.5j IA | Aucun | très basse (cohabitation cache-worker OK) |

### Recommandation

Avant de relancer une session sur ces différés, **valider en prod le gain des phases 0-2 actuelles** via la baseline `docs/BASELINE_METRICS.md`. Plusieurs des items différés (cache moka notamment) deviennent évidents ou inutiles selon ce que la baseline révèle.

Si tu veux quand même reprendre quelque chose tout de suite, l'ordre logique est :
1. **Cache moka** (le seul vrai gain perf restant pour Phase 2)
2. **NOT NULL constraints** après audit data prod (qualité long terme)
3. Le reste est du nice-to-have.
