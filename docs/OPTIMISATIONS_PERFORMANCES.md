# Optimisations Performances & Scalabilité

Document d'analyse et propositions d'optimisations performances/scalabilité pour DiscordSentinel.

**Contexte** : projet Rust workspace avec ~16 bots Discord (Serenity), API Axum hexagonale, 6 workers, PostgreSQL, Redis, déployé sur serveur Linux.

---

## 🔴 Tier 1 — Gains massifs, effort raisonnable

### 1. Unifier les API clients bots→API (HTTP keep-alive pool partagé)

**Problème** : chaque bot réimplémente son `api_client.rs` avec `reqwest::Client::new()` parfois recréé à chaque appel. Pas de pool HTTP keep-alive → handshake TCP+TLS à chaque requête.

**Fix** : une seule instance `Arc<reqwest::Client>` par bot, configurée :

```rust
reqwest::Client::builder()
    .pool_max_idle_per_host(32)
    .pool_idle_timeout(Duration::from_secs(90))
    .tcp_keepalive(Duration::from_secs(60))
    .http2_prior_knowledge()
    .timeout(Duration::from_secs(10))
    .build()
```

**Gain** : **-50 à -80 % de latence** sur les appels API fréquents (heartbeat, logs, moderation actions).

---

### 2. gRPC interne pour trafic haute-fréquence

**Problème** : les bots parlent à l'API en REST JSON. Pour du trafic interne répétitif (heartbeats, logs, events), c'est lourd : sérialisation JSON coûteuse, pas de multiplexing, pas de streaming natif.

**Fix** : **tonic (gRPC)** pour les endpoints internes (`/logs`, `/heartbeat`, `/moderation/actions`, `/ai/jobs`). Garder REST pour le desktop Tauri et les clients externes.

**Bénéfices** :
- Sérialisation Protobuf = **3-5× plus rapide** que JSON
- Multiplexing HTTP/2 natif
- Streaming bidirectionnel pour l'IA et les events
- Contrats typés partagés via `.proto`

**Gain** : **2-5× de throughput** sur les routes internes.

---

### 3. Redis pub/sub → Redis Streams avec consumer groups

**Problème** : Redis pub/sub est utilisé par seulement 3 bots et **sans persistance**. Si un consumer est down, les messages sont perdus. Impossible de scaler un worker en plusieurs instances sans dédoublement de traitement.

**Fix** : migrer vers **Redis Streams** avec consumer groups.

**Avantages** :
- Persistance des events (relecture possible après crash)
- Load-balancing automatique entre N instances d'un même worker
- Scaling horizontal trivial : ajouter une instance = elle rejoint le consumer group
- ACK explicite des messages traités
- Dead-letter queue native via `XCLAIM`

**Gain** : brique clé pour scaler les workers au-delà d'une instance. **Scalabilité horizontale débloquée**.

---

### 4. Cache Serenity tailoré par bot

**Problème** : par défaut Serenity cache tout (guilds, members, messages, presences, voice states). Sur 16 bots connectés aux mêmes guilds = **16× la même donnée en RAM**.

**Fix** : `CacheSettings` strict par bot selon son besoin réel.

| Bot | Cache nécessaire |
|---|---|
| `blackjack-bot` / `coude-bot` | Minimal (guild id seulement) |
| `moderation-bot` | Pas de messages ni presences |
| `voice-bot` | Pas de messages |
| `automod-bot` | Pas de presences ni voice |
| `welcome-bot` | Members uniquement |
| `progression-bot` | Members + messages récents |

**Gain** : **-30 à -60 % de RAM** sur l'ensemble des bots + moins de pression GC sur tokio.

---

## 🟡 Tier 2 — Scalabilité horizontale

### 5. Sharding Discord propre

**Problème** : aujourd'hui chaque bot est mono-shard. À >2500 guilds ou gateways saturés, Discord impose le sharding.

**Fix** :
- `ShardManager` Serenity configuré dynamiquement selon le nombre de guilds
- Chaque shard dans son propre process (scalabilité horizontale)
- État partagé entre shards via Redis (cache distribué)

**Urgence** : pas immédiat si mono-serveur moyen, **critique** dès multi-serveurs.

---

### 6. PgBouncer + pools PostgreSQL centralisés

**Problème** : si les bots font des queries directes, chaque bot a son pool SQLx. 16 bots × 5 connexions = **80 connexions** rien que pour les bots, avant API + workers. Postgres fatigue dès ~200 connexions.

**Fix** :
- **Bots ne parlent pas à Postgres directement** — seulement via l'API (respect de l'architecture hexagonale)
- Déployer **PgBouncer en transaction pooling** devant Postgres → milliers de clients pour quelques dizaines de connexions backend
- Pool SQLx côté API : `max_connections = 20-30` max
- Pool SQLx côté workers : `max_connections = 5-10` par worker

**Gain** : **5-10× plus de clients simultanés** sans fatiguer Postgres.

---

### 7. Cache-aside Redis systématique devant la DB

**Problème** : l'API recharge souvent les mêmes données (configs guild, user profiles, permissions). Le `cache-worker` existant refresh, mais le pattern n'est pas généralisé.

**Fix** : pattern **cache-aside** systématique sur les endpoints hot :

```rust
async fn get_guild_config(id: GuildId) -> Result<GuildConfig> {
    if let Some(cached) = redis.get(&format!("guild_config:{id}")).await? {
        return Ok(cached);
    }
    let from_db = db.fetch_guild_config(id).await?;
    redis.set_ex(&format!("guild_config:{id}"), &from_db, 60).await?;
    Ok(from_db)
}
```

- TTL court (30-60s) pour configs qui changent rarement
- Invalidation via pub/sub lors des writes
- Clés versionnées pour éviter les conflits

**Gain** : **-70 à -90 % de charge DB** sur les endpoints fréquents.

---

## 🟢 Tier 3 — Optimisations fines

### 8. Allocateur jemalloc/mimalloc

**Fix** : utiliser jemalloc comme allocateur global dans tous les binaires long-running.

```toml
[dependencies]
tikv-jemallocator = "0.6"
```

```rust
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;
```

**Gain** : **-15 % RAM** + réduction drastique de la fragmentation sur services qui tournent des semaines.

---

### 9. tokio-metrics + tasks nommées

**Fix** : activer `tokio-metrics` pour détecter :
- Tasks qui bloquent le runtime (CPU-bound dans async)
- Starvation du pool
- Tasks qui leak
- Latence poll / idle

**Gain** : outil de diagnostic critique pour debugger les ralentissements en prod.

---

### 10. Batch writes vers la DB

**Problème** : les events (moderation, audit, analytics) sont écrits un par un. Lourd pour Postgres en haut volume.

**Fix** :
- Buffer en mémoire + flush toutes les 500ms OU 100 events
- `INSERT ... VALUES (...), (...), (...)` plutôt que N inserts
- `COPY FROM STDIN` pour les très gros volumes (analytics)
- Utiliser `sqlx::query!` avec tableau de tuples

**Gain** : **10-50× de throughput** sur les tables event-heavy.

---

### 11. Compression HTTP & Redis

**Fix** :
- **Axum** : tower-http `CompressionLayer` avec gzip/brotli pour réponses > 1 KB
- **Redis** : compression zstd/lz4 côté client pour valeurs > 4 KB
- **gRPC** : compression gzip native (activée dans tonic)

**Gain** : **-60 à -80 % de bande passante interne**.

---

### 12. Audit index Postgres + EXPLAIN ANALYZE

**Problème** : avec 98 migrations, il y a certainement des index manquants ou redondants.

**Checklist** :
- [ ] Toutes les FK sont indexées
- [ ] Colonnes `created_at` / `updated_at` utilisées en range queries indexées
- [ ] Colonnes `guild_id` + `user_id` en index composites
- [ ] `pg_stat_statements` activé pour identifier top queries lentes
- [ ] `pg_stat_user_indexes` pour détecter les index jamais utilisés (à supprimer)
- [ ] Vérifier les query plans sur les endpoints hot (history, export, stats)
- [ ] Partitionnement des grosses tables event-heavy par `created_at` (mensuel)

**Gain** : souvent **10-100×** sur certaines queries mal indexées.

---

## 📊 Récapitulatif impact/effort

| # | Optimisation | Effort | Gain perf | Gain scalabilité |
|---|---|---|---|---|
| 1 | HTTP keep-alive pool partagé | Faible | 🔴🔴🔴 | 🟡 |
| 2 | gRPC interne | Moyen | 🔴🔴🔴 | 🔴🔴 |
| 3 | Redis Streams + consumer groups | Moyen | 🟡 | 🔴🔴🔴 |
| 4 | Cache Serenity tailored | Faible | 🔴🔴 | 🔴 |
| 5 | Sharding Discord | Élevé | 🟡 | 🔴🔴🔴 |
| 6 | PgBouncer + pool centralisé | Faible | 🟡 | 🔴🔴🔴 |
| 7 | Cache-aside Redis systématique | Moyen | 🔴🔴🔴 | 🔴🔴 |
| 8 | jemalloc | Très faible | 🔴 | 🟡 |
| 9 | tokio-metrics | Faible | 🔴 (diag) | 🟡 |
| 10 | Batch writes DB | Moyen | 🔴🔴🔴 | 🔴🔴 |
| 11 | Compression HTTP/Redis | Faible | 🔴 | 🔴 |
| 12 | Audit index Postgres | Moyen | 🔴🔴🔴 | 🔴🔴 |

Légende : 🔴 = impact notable, 🔴🔴 = impact élevé, 🔴🔴🔴 = impact massif, 🟡 = impact mineur

---

## 🗺️ Roadmap recommandée

### Phase 1 — Quick wins (1 semaine)

Objectif : gains immédiats à effort minimal, sans refonte.

1. **#1** HTTP keep-alive pool partagé → refactor `BaseApiClient` dans `bots/shared`
2. **#4** Cache Serenity tailored par bot → 1 `CacheSettings` par `main.rs`
3. **#8** jemalloc → ajout global à tous les binaires release
4. **#11** Compression HTTP → `CompressionLayer` sur Axum

### Phase 2 — Chantier infrastructure (2-3 semaines)

Objectif : débloquer la scalabilité horizontale.

5. **#6** PgBouncer devant Postgres → ajout au docker-compose, ajustement pools SQLx
6. **#3** Migration Redis pub/sub → Redis Streams → refactor `sentinel-shared` + workers consommateurs
7. **#7** Cache-aside systématique → middleware Redis dans la couche adapters de l'API
8. **#9** tokio-metrics → instrumentation de tous les binaires

### Phase 3 — Gros chantiers (1-2 mois)

Objectif : perf finale et throughput maximal.

9. **#2** gRPC interne → migration progressive endpoint par endpoint (logs en premier)
10. **#10** Batch writes DB → buffering côté API pour tables event-heavy
11. **#12** Audit complet index Postgres + tuning + partitionnement

### Phase 4 — Scaling au besoin

Objectif : à déclencher quand les limites arrivent.

12. **#5** Sharding Discord → quand on dépasse 2500 guilds ou qu'on vise multi-serveurs

---

## 🎯 Priorités absolues si budget limité

Si tu ne fais que **3 choses** :

1. **#6 PgBouncer** — débloque la scalabilité DB avec quasi zéro effort
2. **#1 HTTP pool partagé** — améliore instantanément la latence de tout le système
3. **#12 Audit index Postgres** — souvent les plus gros gains cachés, ROI imbattable

Ces trois optimisations apportent typiquement **70 % du gain total** pour **20 % de l'effort**.

---

## 📎 Liens connexes

- [`WORKERS_PROPOSES.md`](./WORKERS_PROPOSES.md) — propositions de workers (extraction IA, reminders, etc.)
- [`ESTIMATION_RAM_PROD.md`](./ESTIMATION_RAM_PROD.md) — estimation RAM en prod, tuning build Rust
- [`bots/moderation-bot/AMELIORATIONS.md`](../bots/moderation-bot/AMELIORATIONS.md) — améliorations fonctionnelles moderation-bot
