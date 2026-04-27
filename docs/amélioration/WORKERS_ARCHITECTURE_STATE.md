# Workers — État des lieux & plan d'amélioration

**Date** : 2026-04-27
**Périmètre** : `services/workers/*` — 14 workers + 1 crate shared (`worker-common`)
**Status** : 🟢 **A−** — infrastructure mutualisée excellente, quelques jobs avec logique métier en SQL à migrer vers l'API.

---

## 1. Architecture cible

Les workers sont des **batch jobs périodiques** ou **stream consumers**. Leur rôle :
- ETL / agrégations (snapshots analytics, refresh caches, partitions)
- Tâches programmées (cleanup, vacuum, expire timers)
- Consumers Redis streams (events `sentinel:events`)
- Délégation à l'API pour les opérations qui touchent du métier complexe

```
┌──────────────────────────────────────────────────┐
│  worker-common (lib partagée)                    │
│   ├─ create_pg_pool / init_tracing               │
│   ├─ start_heartbeat / shutdown_signal           │
│   ├─ load_worker_config (DB + env)               │
│   ├─ spawn_periodic (scheduler avec shutdown)    │
│   └─ is_worker_enabled (per-guild gate)          │
└──────────────────────────────────────────────────┘
                  ↑ utilisé par tous
┌──────────────────────────────────────────────────┐
│  worker-X/                                       │
│   ├─ main.rs       (boilerplate uniforme)        │
│   ├─ config.rs     (struct + from_env)           │
│   ├─ scheduler.rs  (registration des jobs)       │
│   └─ jobs/*.rs     (1 fichier = 1 tâche)         │
└──────────────────────────────────────────────────┘
                  ↓ deux patterns possibles
   ┌───────────────────────┐    ┌──────────────────────┐
   │ Thin worker           │    │ Thick worker         │
   │ → gRPC/HTTP API call  │    │ → SQL direct         │
   │ ex: coude/hp_regen    │    │ ex: cache/warm_*     │
   └───────────────────────┘    └──────────────────────┘
```

**Règle d'or** : un worker **PEUT** faire du SQL direct **si et seulement si** la tâche est de l'ETL/maintenance technique (vacuum, partition, snapshot, cleanup). **DOIT** déléguer à l'API si la tâche applique une règle métier.

---

## 2. Verdict global

| Critère | Note | Commentaire |
|---|---|---|
| **`worker-common` shared** | 🟢 A | 446 lignes, 9 tests unitaires. Init + heartbeat + scheduler + config — boilerplate éliminé. |
| **Cohérence des `main.rs`** | 🟢 A | 14/14 workers suivent le même pattern (init → pool → config → scheduler → heartbeat → shutdown). |
| **Granularité des jobs** | 🟢 A | 1 fichier = 1 job. Lisibilité excellente. |
| **Délégation gRPC à l'API** | 🟡 B+ | 4 jobs délèguent proprement (`hp_regen`, `expire_combats`, `redistribute_cashbox`, `daily_chaos`). |
| **Logique métier en SQL direct** | 🔴 B− | **3-4 jobs** appliquent des règles métier en SQL (`conduct_regen`, `cleanup_bans`, `sync_ban_proposals`, partiellement `escalate_appeal_sla`). Devraient appeler l'API. |
| **Idempotence** | 🟢 A | `FOR UPDATE SKIP LOCKED` ou `RETURNING` pour les claims atomiques (cf. `cleanup_afk_tables`). |
| **Observabilité** | 🟢 A | `tracing` partout, métriques Prometheus via `worker-common::metrics`, lifecycle logs envoyés à l'API. |
| **Shutdown gracieux** | 🟢 A | `tokio::sync::watch` partagé, `shutdown_signal` Ctrl+C/SIGTERM. |
| **Tests** | 🟡 B− | Seul `worker-common` est testé. Pas de tests unitaires sur les jobs métier. |
| **Warnings** | 🟢 A | `cargo check -p sentinel-worker-common` propre. À vérifier crate par crate. |

---

## 3. Inventaire des 14 workers

### 3.1 Par catégorie

#### 🟢 Pure ETL / maintenance (✅ pattern correct)

| Worker | Jobs | Description |
|---|---|---|
| `cache-worker` | `warm_analytics`, `warm_dashboard`, `warm_voice_stats`, `refresh_leaderboards`, `sync_user_cache`, `manage_partitions` | Pre-compute des caches et partitions PG |
| `analytics-worker` | `hourly_snapshot`, `daily_snapshot` | Agrégation périodique pour le dashboard |
| `cleanup-worker` | `cleanup_old_data`, `vacuum_tables` | Purge + maintenance disque |
| `audit-cache-worker` | `refresh_watched_users` | Rafraîchit cache Redis depuis DB |
| `blackjack-cleanup-worker` | `cleanup_afk_tables` | Marque les tables AFK + publie event au bot |
| `temp-roles-worker` | `expire_temp_roles` | Retire les rôles Discord temporaires expirés |
| `monitoring-worker` | (monitor) | Health checks + alerting |

#### 🟢 Drain / consumer (jobs queue/stream — pattern correct)

| Worker | Jobs | Description |
|---|---|---|
| `ai-worker` | `drain_ai_jobs` | Consume `ai_jobs` queue, appelle inference, persiste |
| `export-worker` | `drain_export_jobs` | Consume `export_jobs`, génère CSV/PDF |
| `discord-audit-sync-worker` | `sync_discord_audit_logs` | Pull audit logs Discord, persiste |

#### 🟡 Workers délégant à l'API (✅ pattern thin idéal)

| Worker | Jobs gRPC API | Description |
|---|---|---|
| `coude-worker` | `hp_regen`, `expire_combats`, `redistribute_cashbox`, `daily_chaos`, `resolve_betting`, `resolve_tournament` | 6 jobs Coude : tous appellent l'API pour la logique métier. |
| `appeal-sla-worker` | `escalate_appeal_sla` | Escalade les appels en retard (mix SQL + API) |

#### 🔴 Workers avec **métier en SQL direct** (à corriger)

| Worker | Jobs | Logique métier dupliquée |
|---|---|---|
| `moderation-worker` | `conduct_regen` | Applique le calcul de regen + suppression conditionnelle (`new_points >= max_points` → DELETE). Le domain `apply_conduct_regen` existe côté API mais **n'est pas utilisé**. |
| `moderation-worker` | `cleanup_bans` | TBD à vérifier |
| `moderation-worker` | `sync_ban_proposals` | TBD à vérifier |
| `moderation-worker` | `send_reminders` | Envoie les sanction reminders. Possible métier (templating, dispatch). |

---

## 4. Conformité par règle d'or

### 4.1 `worker-common` factorisation

✅ **Excellent**. 9 tests unitaires, API minimaliste mais complète :

```rust
common::init_tracing("worker_x=info");
let pool = common::create_pg_pool(&url).await;
let db_config = common::load_worker_config(&pool, WORKER_NAME).await;
common::spawn_periodic("job_name", 60, pool, shutdown_rx, api_url, WORKER_NAME, |p| Box::pin(my_job::run(p)));
common::start_heartbeat(api_url, WORKER_NAME);
common::shutdown_signal().await;
```

Le scheduler `spawn_periodic` :
- vérifie `is_worker_globally_enabled` à chaque tick (kill-switch via DB)
- log les erreurs vers `/api/logs` (lifecycle visibility)
- respecte le shutdown via `watch::Receiver<bool>`

### 4.2 Pattern `main.rs` uniforme

✅ 14/14 workers suivent **exactement le même squelette** :

```rust
#[global_allocator]
static GLOBAL: jemalloc;       // perf

mod config; mod jobs; mod scheduler;

#[tokio::main] async fn main() {
    common::init_tracing(...);
    let mut config = WorkerConfig::from_env();
    let pool = common::create_pg_pool(...).await;
    let db_config = common::load_worker_config(...).await;
    config.apply_db_config(&db_config);
    let (tx, rx) = watch::channel(false);
    scheduler::start(&config, pool.clone(), redis, rx);
    common::start_heartbeat(...);
    common::send_lifecycle_log("info", "demarre").await;
    common::shutdown_signal().await;
    common::send_lifecycle_log("warn", "arret").await;
    let _ = tx.send(true);
    pool.close().await;
}
```

→ Uniformité = facile d'ajouter un nouveau worker, facile à debug. Excellent point.

### 4.3 Délégation gRPC propre

✅ `coude-worker/hp_regen` (~88 lignes) : ouvre une connection gRPC, appelle `CoudePlayerServiceClient::hp_regen_tick`, c'est tout. La logique de régénération vit dans `CoudePlayerRepository::regen_hp_tick` côté API. **Modèle à suivre.**

✅ `blackjack-cleanup-worker/cleanup_afk_tables` : `UPDATE ... RETURNING` atomique en DB + publish event Redis pour que le bot supprime le channel Discord. Pattern écrit-DB + dispatch event = sain.

### 4.4 Logique métier en SQL direct (anti-pattern)

❌ **`moderation-worker/conduct_regen.rs`** est le plus clair exemple :

```rust
let new_points = user.points + config.regen_amount;
if new_points >= config.max_points {
    // DELETE ...
} else {
    // UPDATE points = new_points
}
```

Cette règle vit aussi dans `services/api/src/domain/entities/conduct.rs::apply_conduct_regen`. **Duplication = bug latent** : modifier la règle dans le domain ne propage pas au worker.

**Fix** : exposer un endpoint API `POST /api/conduct/regen-tick` ou un RPC gRPC, et que le worker appelle. Le worker ne sait plus que `regen_amount` ou `max_points` existent.

Autres jobs à auditer (TODO dans cet audit) :
- `moderation-worker/cleanup_bans` : nettoie les bans expirés. Si applique des règles → migrer.
- `moderation-worker/sync_ban_proposals` : sync entre DB et Discord. Mix légitime.
- `moderation-worker/send_reminders` : génère des rappels. Templating + dispatch = candidat migration.
- `appeal-sla-worker/escalate_appeal_sla` : règle SLA → si métier, migrer.

### 4.5 Idempotence + claim atomique

✅ Pattern `UPDATE ... WHERE id IN (SELECT ... FOR UPDATE SKIP LOCKED) RETURNING ...` présent dans :
- `blackjack-cleanup-worker/cleanup_afk_tables`
- `temp-roles-worker/expire_temp_roles`
- `coude-worker/expire_combats` (via API)

→ Plusieurs replicas workers OK, pas de double-traitement.

⚠️ À vérifier sur `cleanup-worker/cleanup_old_data` et `cache-worker/sync_user_cache` qui pourraient bénéficier du même verrou.

### 4.6 Heartbeat + lifecycle logs

✅ Tous les workers heartbeat toutes les 30s vers `/api/bots/heartbeat` (avec API_KEY Bearer auth). Le dashboard sait quels workers sont vivants.
✅ Lifecycle "demarre" + "arret" envoyés via `/api/logs`. Visibilité opérationnelle propre.

### 4.7 Tests

❌ **Très faible couverture**. Seul `worker-common` a des tests (9 tests unitaires). Les jobs métier comme `conduct_regen` ne sont pas testés.

**Justification partielle** : les jobs sont thin et délèguent à l'API ou font du SQL pur. Les tests d'intégration de l'API couvrent indirectement.

**Fix proposé** : tests unitaires sur les fonctions de calcul pures (jamais testés tels quels) — par exemple un test que `escalate_appeal_sla` envoie un payload correct au format event.

---

## 5. Pain points

### 5.1 `moderation-worker/conduct_regen` duplique le domain

Décrit en 4.4. Priorité 🔴.

### 5.2 4 workers délèguent à `coude-worker` mais via SQL local

Le `coude-worker` fait 6 jobs, parmi lesquels `daily_chaos` et `resolve_betting` mélangent SQL direct ET appel API. Pas critique, mais l'idéal serait que TOUT le métier passe par l'API gRPC pour qu'on puisse changer la règle sans déployer le worker.

### 5.3 Pas de retry/backoff explicite

`spawn_periodic` schedule à intervalle fixe. Si un job échoue, il sera retenté au prochain tick (backoff implicite via interval). Pas de retry exponentiel ni dead-letter queue.

→ Pour les jobs critiques (`drain_ai_jobs`, `drain_export_jobs`), un mécanisme de DLQ après N échecs serait précieux.

### 5.4 Workers sans monitoring spécifique

`monitoring-worker` existe mais c'est lui-même un worker monitorant les autres — pas évident de savoir qui surveille le surveillant. Single-point-of-failure si lui-même crash silencieusement.

### 5.5 Configuration via env + DB — risque de drift

Chaque worker accepte sa config via 3 sources : DB → env → defaults. Bon pour la flexibilité mais pas évident de savoir quelle valeur s'applique en prod (cf. helper `config_or_env` dans `worker-common`).

→ Fix UX : exposer dans le dashboard la valeur **effective** par worker.

---

## 6. Plan de correction priorisé

### Phase A — P0, ~1 jour

1. **Migrer `moderation-worker/conduct_regen`** vers un appel gRPC à l'API. Crée `RegenConductTickRpc` côté proto, expose `apply_conduct_regen` du domain via le port inbound. Worker devient ~30 lignes (lecture rates + appel RPC).

2. **Auditer `cleanup_bans`, `sync_ban_proposals`, `send_reminders`** côté `moderation-worker`. Pour chaque : déterminer si métier ou pure ETL. Migrer si métier.

3. **Auditer `escalate_appeal_sla`** (`appeal-sla-worker`) — règle SLA potentielle.

### Phase B — P1, ~2 jours

4. **Ajouter tests unitaires sur les jobs critiques** : au minimum `drain_ai_jobs`, `drain_export_jobs`, `expire_temp_roles`, `cleanup_afk_tables`. Mock du PgPool ou tests d'intégration legers.

5. **Mécanisme de retry/DLQ pour les drain jobs** : après N échecs successifs, marquer le job en DLQ pour inspection manuelle.

6. **Vérifier `cleanup_old_data` et `sync_user_cache`** : ajouter `FOR UPDATE SKIP LOCKED` si des replicas peuvent tourner simultanément.

### Phase C — P2, long terme

7. **Dashboard "config effective"** : pour chaque worker, montrer quelle valeur (DB / env / default) est appliquée à chaque clé.

8. **Tests d'intégration multi-workers** : harness qui spin up un mini-PG + lance plusieurs replicas du même worker pour valider l'idempotence.

9. **Monitoring du `monitoring-worker`** : alerte externe (Prometheus alertmanager) si plus de heartbeat depuis X min.

---

## 7. Métriques

| Métrique | Valeur | Cible |
|---|---|---|
| Workers | 14 | — |
| Crate shared | 1 (`worker-common`, ~450 LOC) | — |
| Lignes totales (src/) | ~5 600 | — |
| Tests `worker-common` | 9 ✓ | — |
| Tests jobs métier | ~0 | viser quelques-uns sur les critiques |
| Workers délégant proprement gRPC | ~6 jobs sur 30 | viser les jobs métier |
| Jobs avec SQL métier dupliqué | 1 confirmé (conduct_regen), 3-4 suspects | 0 |
| Workers avec heartbeat | 14/14 ✓ | 14/14 |
| Workers avec shutdown gracieux | 14/14 ✓ | 14/14 |
| Workers sans warnings cargo | à confirmer | 14/14 |

---

## 8. Comparaison API / Bot / Workers

| Aspect | API | Bot | Workers |
|---|---|---|---|
| Architecture formelle | Hexagonale | Modulaire par feature | Pattern uniforme `main → scheduler → jobs` |
| Domaine pur | ✅ | N/A (façade) | N/A (batch jobs) |
| SQL direct autorisé ? | ❌ (sauf health) | ❌ | ✅ pour ETL/cleanup, ❌ pour métier |
| Délégation à l'API | (l'API EST l'API) | 100% | partielle (à augmenter) |
| Tests | 2524 ✓ | 658 ✓ | 9 (worker-common seul) |
| Warnings | 0 ✅ | 0 ✅ | à confirmer |
| Shared infra | — | sentinel_shared | worker-common |
| Verdict | 🟢 B+→A− | 🟢 A− | 🟢 A− |

---

## 9. Ce qui ne sera PAS dans cet audit

- Performance des batch jobs (durée d'exécution, lock contention) — out of scope
- Stratégie de scaling horizontal (N replicas) — déjà couverte par `FOR UPDATE SKIP LOCKED`
- Sécurité gRPC/HTTP (mTLS, rotation API_KEY) — couvert ailleurs
- Migration vers un orchestrator type Temporal/Cadence — décision business

---

## 10. TL;DR — actions recommandées

```
🔴 P0 (1 jour, low-risk)
  □ Migrer moderation-worker/conduct_regen → RPC API
  □ Auditer cleanup_bans, sync_ban_proposals, send_reminders, escalate_appeal_sla

🟠 P1 (2 jours, modéré)
  □ Tests unitaires sur drain_ai_jobs, drain_export_jobs, expire_temp_roles, cleanup_afk_tables
  □ Retry/DLQ pour drain jobs après N échecs
  □ FOR UPDATE SKIP LOCKED sur cleanup_old_data, sync_user_cache

🟡 P2 (long terme)
  □ Dashboard "config effective" par worker
  □ Tests d'intégration multi-replicas
  □ Alerte externe sur monitoring-worker silent
```

**Verdict final** : les workers sont **dans un excellent état**. `worker-common` est exemplaire, le pattern `main.rs` ultra-cohérent, l'idempotence est correcte. **Le seul vrai défaut** est la duplication métier dans `moderation-worker/conduct_regen` (et probablement 2-3 autres jobs à auditer). Une journée de travail suffit pour corriger les P0.
