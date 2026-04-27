# API — État des lieux hexagonal & plan d'amélioration

**Date** : 2026-04-27
**Périmètre** : `services/api` (43 ports inbound, 57 ports outbound, ~40 services application, ~50 entités domain)
**Status** : 🟢 **B+ → A−** après les 4 phases de migration récentes (Phase 1/2/3 + leftovers + cleanup)

---

## 1. Architecture cible (rappel)

```
┌─────────────────────────────────────────────────────┐
│  adapters/inbound  (HTTP, gRPC, WS)                 │  ← traduction DTO ↔ use case
│         ↓                                           │
│  ports/inbound  (UseCase traits)                    │  ← contrats publics
│         ↓                                           │
│  application  (Service<UseCase>)                    │  ← orchestration
│         ↓                ↓                          │
│  domain  ←  ports/outbound (Repository traits)      │  ← métier pur + contrats DB
│                          ↓                          │
│              adapters/outbound  (Postgres, Redis,   │
│              Discord HTTP, Inference, Job client)   │
└─────────────────────────────────────────────────────┘
```

Règles d'or :
1. Le **domain** ne dépend de rien (ni `sqlx`, ni `redis`, ni `reqwest`, ni `tokio`, ni `serde`).
2. Les **ports** exposent uniquement des types domain ou primitives (`String`, `i64`, `Uuid`).
3. L'**application** orchestre via des `Arc<dyn Port>`. Pas de SQL ni HTTP direct.
4. Les **adapters** traduisent — handlers fins, repos qui mappent SQL → entités domain.

---

## 2. Verdict global

| Couche | Note | Commentaire |
|---|---|---|
| **Domain** | 🟢 A− | 1 200+ lignes de fonctions pures testées. 4 dérives `sqlx::Type/FromRow` à éliminer. |
| **Ports inbound** | 🟢 A | 43 traits clairs avec `async_trait`. Plus de `unimplemented!()` sauf dans `manage_wallet.rs` (6 résidus). |
| **Ports outbound** | 🟢 A− | 57 repos, signatures propres. 4 leaks `serde_json::Value`. Granularité variable (certains très larges). |
| **Application** | 🟡 B+ | Bonne séparation. **2 services en violation** (`export_service`, `manage_stats_service`). 14+ couplages use-case → use-case (variable selon les cas). |
| **Adapters inbound (HTTP)** | 🟡 B | Plupart fins. **20 handlers** font du SQL direct `sqlx::query` sur `state.pg_pool` — violation. |
| **Adapters inbound (gRPC)** | 🟢 A | Tous fins, utilisent `domain_to_status` correctement. |
| **Adapters outbound** | 🟢 A | Postgres adapters propres (mappent `Row → Entity`), pas de logique métier. |
| **Erreurs** | 🟢 A | `DomainError` enum exhaustif (10 variants), mappé HTTP/gRPC. `NotImplemented` (501) ajouté en Phase 1. |
| **Tests** | 🟢 A− | 2524 lib + 658 bot + nombreux integration. Mocks propres dans `tests/`. |

---

## 3. Conformité par règle d'or

### 3.1 Domain pur

✅ **Bon** : `find -path src/domain` + `grep "use sqlx\|use redis\|use reqwest"` → **0 match** sur les imports propres.

⚠️ **À nettoyer** : 4 dérives macro qui couplent au driver SQL (audit P2 #7) :

| Fichier | Dérive |
|---|---|
| `domain/value_objects/coude_class.rs:8` | `sqlx::Type` |
| `domain/value_objects/moderation_gravity.rs:6` | `sqlx::Type` |
| `domain/value_objects/voice_channel_kind.rs:6` | `sqlx::Type` |
| `domain/entities/discord_role.rs:4` | `sqlx::FromRow` |

**Fix** : déplacer ces dérives dans des wrappers côté `adapters/outbound/postgres` (newtype pattern). Décision pas urgente — l'enum est PG-native, le couplage est minime.

### 3.2 Ports outbound — pas de fuite driver

✅ **57/57 traits** exposent uniquement domain types ou primitives.

⚠️ **4 fuites `serde_json::Value`** :

| Port | Champ | Sévérité |
|---|---|---|
| `BlackjackTableRepository::create(deck_json)` | param | 🟠 Devrait être `Vec<Card>` (domain) |
| `BlackjackTableRepository::list_games() -> Vec<Value>` | retour | 🟠 Devrait être `Vec<BlackjackGameSummary>` |
| `ManageAuditLogsUseCase::CreateAuditLogCommand.details` | param | 🟢 OK — log polymorphe par nature, à documenter |
| `ManageMembersUseCase::SyncMembersCommand.roles` | param | 🟡 Probablement `Option<Vec<String>>` |

### 3.3 Application — pas de SQL ni HTTP direct

❌ **2 violations P0** :

#### A. `application/export_service.rs` utilise `sqlx::PgPool`

```rust
use sqlx::PgPool;  // ← ligne 8

pub struct ExportService {
    pool: PgPool,  // ← service tient une référence DB directe
    ...
}
```

L'export bypass complètement la couche ports. Justification possible : agrégations cross-tables ad-hoc. **Fix** : créer un port `ExportQueryRepository` ou plusieurs (`InfractionExportRepository`, `WalletExportRepository`) avec des méthodes spécialisées.

#### B. `application/manage_stats_service.rs` utilise `redis::AsyncCommands`

```rust
use redis::AsyncCommands;  // dans 2 fonctions (l. 42, 196)
```

Le service écrit/lit des compteurs Redis directement. **Fix** : extraire un port `StatsCachePort` (déjà partiellement présent via `cache::CachePort` — l'utiliser).

✅ **Acceptable** : `application/manage_wallet_service.rs:33` utilise `sqlx::Transaction` mais c'est intentionnel — c'est le pattern des `*_tx` methods documenté dans le port `ManageWalletUseCase` (permet aux call sites composites de partager une tx).

### 3.4 Couplage use-case → use-case

📊 **15 services dépendent d'autres use cases** (en `Arc<dyn ManageXxxUseCase>`) :

| Service | Use cases injectés | Justification |
|---|---|---|
| `analyze_image_service` | conduct_uc | 🟢 légitime (workflow modération) |
| `analyze_message_service` | conduct_uc | 🟢 légitime |
| `manage_coude_combats_service` | players_uc (optionnel) | 🟡 audit P0 #2 résolu pour bets, ici reste |
| `expire_combats_batch_service` | bets_uc | 🟡 |
| `blackjack_service` | wallet_uc | 🟢 légitime (le wallet est central) |
| `manage_coude_economy_service` | wallet_uc, taunts_uc | 🟢 |
| `manage_coude_heist_service` | inventory_uc | 🟡 |
| `manage_coude_social_service` | wallet_uc | 🟢 |
| `manage_slot_service` | wallet_uc | 🟢 |
| `manage_wallet_service` | taunts_uc | 🟢 |
| `manage_watched_users_service` | infractions_uc, moderation_uc, security_uc, conduct_uc, notes_uc | 🔴 5 use cases — **god service** |
| `play_tout_ou_rien_service` | wallet_uc | 🟢 |
| `manage_wheel_service` | wallet_uc | 🟢 |
| `play_travaux_service` | wallet_uc | 🟢 |
| `manage_moderation_service` | conduct_uc, strikes_uc | 🟢 |

**`ManageWalletUseCase`** est partagé par 8 services — c'est légitime, le wallet est un agrégat central. Le pattern est sain car il porte de vraies règles métier (faillite/jackpot).

**🔴 `ManageWatchedUsersService`** : un service qui injecte 5 autres use cases est un signal d'orchestration qui devrait peut-être vivre côté handler ou dans un *workflow* dédié. À revoir.

### 3.5 Handlers HTTP fins

❌ **20 handlers font du SQL direct** sur `state.pg_pool` :

Les plus gros offenders :
- `handlers/blackjack/game.rs` : `purge_all` fait 2x `sqlx::query("DELETE...")` direct
- `handlers/dashboard.rs`, `handlers/exports.rs`, `handlers/system.rs` : agrégations ad-hoc
- `handlers/coude/tournaments.rs`, `handlers/coude/prestige.rs` : queries complexes
- `handlers/voice_channels.rs`, `handlers/tickets.rs`, `handlers/notes.rs`, `handlers/security.rs`
- `handlers/guild_members.rs`, `handlers/bot_persistence.rs`, `handlers/rbac.rs`, `handlers/ai_jobs.rs`
- `handlers/moderation.rs`, `handlers/health.rs` (ce dernier est OK — health check)
- `handlers/ws/middleware/rbac.rs`, `handlers/errors_helpers.rs`

**Fix** : créer ou étendre les ports outbound correspondants. Souvent il suffit d'ajouter une méthode au repo existant.

### 3.6 `unimplemented!()` panics restants

✅ **Phase 1 P0 #1** réglé pour `CoudeCombatRepository` et `ManageCoudeCombatsUseCase`.

❌ **6 résidus** dans `ports/inbound/manage_wallet.rs` (default impls) :

```
manage_wallet.rs:155  fn get_or_create — unimplemented!()
manage_wallet.rs:160  fn list_by_guild — unimplemented!()
manage_wallet.rs:169  fn leaderboard — unimplemented!()
manage_wallet.rs:179  fn get_transactions — unimplemented!()
manage_wallet.rs:191  fn reset_wallet — unimplemented!()
manage_wallet.rs:200  fn reset_all_wallets — unimplemented!()
```

**Fix trivial** : remplacer par `Err(DomainError::NotImplemented(...))` (même pattern que coude). 1 commit, ~20 lignes.

### 3.7 Adapters outbound

✅ **Excellent**. Tous les `Pg*Repository` (50+) :
- prennent un `PgPool` en `new()`
- mappent SQL → entité domain via `query_as`
- traduisent `sqlx::Error` en `DomainError::Internal` via le helper `pg_err`
- ne font aucune logique métier

Exception notable et **acceptable** : `PgCoudePlayerRepository::add_xp` applique le barème de level-up (déterministe, déterminé par les helpers domain `coude_xp_for_level` / `coude_title_for_level`). C'est documenté dans le port et reste un pattern sain.

### 3.8 Bonnes pratiques observées

✅ Le moteur de combat (`domain/services/coude_combat_engine/`) est **pur**, **sans `async`**, testable sans I/O.
✅ Les use cases sont définis comme `trait` async, jamais comme `struct` direct.
✅ `DomainError` couvre toutes les classes d'erreur (404, 400, 422, 403, 409, 429, 504, 500, 501).
✅ Mocks de tests dans `application/tests/` — repo + use case complets, isolés.
✅ Les broadcasters WS (`EventBroadcaster`) sont aussi en port, injectés.

---

## 4. Plan de correction priorisé

### Phase A — P0, ~1 jour

1. **Remplacer les 6 `unimplemented!()` de `manage_wallet.rs`** par `DomainError::NotImplemented`. Trivial.
2. **Extraire `application/export_service.rs::PgPool` derrière un port**. Créer `ExportQueryRepository` avec les méthodes utilisées. ~50 lignes.
3. **Extraire `application/manage_stats_service.rs::redis::AsyncCommands` derrière `CachePort`** (qui existe déjà). Refactor des 2 blocs (l. 42 et 196).

### Phase B — P1, ~2-3 jours

4. **Réduire les 20 handlers HTTP qui font du SQL direct**. Pour chaque : ajouter une méthode au repo concerné (ou créer un repo dédié). Itératif, un par un.
5. **Découper `ManageWatchedUsersService`** (5 use cases injectés). Soit transformer en orchestrateur explicite sans les injections (handlers spécifiques par use case), soit légitimer si c'est vraiment un workflow.
6. **Typer les 4 fuites `serde_json::Value`** :
   - `BlackjackTableRepository::create(deck_json)` → `Vec<Card>`
   - `BlackjackTableRepository::list_games` → `Vec<BlackjackGameSummary>` (nouvelle entité)
   - `manage_members::SyncMembersCommand.roles` → `Option<Vec<String>>`
   - `manage_audit_logs::CreateAuditLogCommand.details` : laisser `Value` (intrinsèque) + ajouter doc.

### Phase C — P2, long terme

7. **Newtypes pour les 4 dérives `sqlx::Type/FromRow` du domain** (audit P2 #7). Pas urgent — décision conventionnelle.
8. **`application/manage_coude_combats_service` : extraire le check `surprise_min_hp_pct`** dans une fonction pure, puis transformer le `with_surprise_gate` en mécanisme moins implicite (le optional `Arc<dyn ManagePlayersUseCase>` est encore une dépendance use-case → use-case déguisée).
9. **`expire_combats_batch_service` → `bets_uc`** : si possible, remplacer par `Arc<dyn CoudeBetRepository>` (port outbound) au lieu du use case complet.

---

## 5. Métriques

| Métrique | Valeur | Cible |
|---|---|---|
| Lignes domain | ~12 000 | — |
| Lignes application | ~18 000 | — |
| Tests lib API | 2 524 ✓ | — |
| Tests bot | 658 ✓ | — |
| Warnings cargo (lib + tests) | **0** | 0 |
| `unimplemented!()` runtime path | 6 | 0 |
| Fuites driver dans ports | 4 (`serde_json::Value`) | 0 |
| Fuites driver dans application | 2 fichiers | 0 |
| Handlers HTTP avec SQL direct | 20 | 0 (sauf health) |
| Couplages use-case → use-case | 15 services | revoir 3-4 cas spécifiques |

---

## 6. Ce qui ne sera PAS dans cet audit

- Performance & scalabilité (pas le sujet ici, l'archi est compatible scale-out par design via les ports)
- Sécurité applicative (RBAC, auth, secrets) — couvert ailleurs
- Observabilité (tracing, métriques) — déjà bien outillé via `tracing`
- API publique (versioning, OpenAPI) — out of scope

---

## 7. Historique des migrations récentes

- **2026-04-25** Phase 1 (audit Coude) : 12 magic constants → `CoudeConfig`, `NotImplemented` variant, `CombatQueryRepository`, P4 #1-3 (clamp_steal, validations create_combat, season helpers), P0 #3 `load_balance_params`
- **2026-04-26** Phase 2 (RNG) : 4 endpoints (tout-ou-rien play, travaux play, insurance scam roll, steal roll). RNG persistant 100% côté API.
- **2026-04-27** Phase 3 (catalogues) : 110 templates `coude_flavor_templates` + endpoint `/flavor/{key}/random`. Suppression de toutes les arrays Rust dupliquées côté bot (steal/heist/prank/refuser/scoop/appel). Plus de fallback local — bot dépend strict de l'API.
- **2026-04-27** Bug fix Blackjack : `touch_activity_by_player` sur start/hit/stand/double pour éviter la fermeture mid-game par le cleanup-worker.
- **2026-04-27** Cleanup : 0 warning sur `cargo check --lib --tests` pour les 2 crates. 2 bugs pré-existants découverts et corrigés (`insurance_msg` overwrite, `regicide_msg` field push avant calcul).

---

## 8. TL;DR — actions recommandées

```
🔴 P0 (1 jour, low-risk)
  □ Remplacer 6 unimplemented!() de manage_wallet.rs par NotImplemented
  □ Sortir export_service de PgPool direct → port ExportQueryRepository
  □ Sortir manage_stats_service de redis::AsyncCommands → CachePort

🟠 P1 (2-3 jours, modéré)
  □ Migrer les 20 handlers HTTP qui font du SQL → ports outbound
  □ Refactorer ManageWatchedUsersService (5 use cases injectés)
  □ Typer les 4 fuites serde_json::Value dans les ports

🟡 P2 (long terme, décision conventionnelle)
  □ Newtype pour sqlx::Type/FromRow dans le domain (4 fichiers)
  □ Découpler manage_coude_combats_service.with_surprise_gate
  □ Évaluer expire_combats_batch_service → port CoudeBetRepository
```
