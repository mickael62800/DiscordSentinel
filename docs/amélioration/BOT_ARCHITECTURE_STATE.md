# Sentinel Bot — État des lieux & plan d'amélioration

**Date** : 2026-04-27
**Périmètre** : `bots/sentinel-bot` (264 fichiers `.rs`, ~46 000 lignes, 15 modules métier)
**Status** : 🟢 **A−** après les 4 phases de migration récentes (RNG → API, templates → DB, fallbacks supprimés)

---

## 1. Architecture cible

Le bot **n'est PAS hexagonal** au sens strict — c'est un **client Discord** consommateur de l'API. Sa couche "domain" est l'API ; lui-même n'est qu'une façade UX.

```
┌────────────────────────────────────────────────────────┐
│  serenity (Discord gateway events + slash commands)    │
│         ↓                                              │
│  modules/{feature}/                                    │
│     ├── mod.rs           ← TypeMapKey + register       │
│     ├── api_client.rs    ← appels HTTP/gRPC            │
│     ├── commands/        ← slash commands              │
│     ├── handlers/        ← gateway event handlers      │
│     └── embeds.rs        ← présentation Discord        │
│         ↓                                              │
│  sentinel_shared::api_client (HTTP)                    │
│  sentinel_shared::grpc_client (tonic)                  │
│  sentinel_shared::event_bus (Redis streams consumer)   │
│         ↓                                              │
│        API Sentinel (services/api)                     │
└────────────────────────────────────────────────────────┘
```

**Règles d'or pour le bot** :
1. **Aucune logique métier persistée** — toute décision qui touche l'état d'un joueur passe par l'API.
2. **Aucun accès direct DB/Redis pour le métier** — uniquement via l'API ou via les events/streams shared.
3. **Présentation seulement** — embeds, animations, formatting, slash command UX.
4. **api_client par module** — point d'entrée unique pour les appels API.
5. **Fallbacks bannis** — si l'API échoue, afficher un message d'erreur clair, pas de logique locale.

---

## 2. Verdict global

| Critère | Note | Commentaire |
|---|---|---|
| **Métier persistant côté bot** | 🟢 A | 0 RNG persistant. Toutes les décisions de jeu sont API. |
| **Structure modulaire** | 🟢 A | 15/15 modules ont leur `api_client`, `mod.rs` + `TypeMapKey`. Très consistant. |
| **Couplage shared** | 🟢 A | `sentinel_shared::{api_client, grpc_client, event_bus, branding}` partagé proprement. |
| **Présentation/embeds** | 🟢 A | Bien isolée dans `embeds.rs` ou inline. Pas de mix avec la logique. |
| **Accès DB/Redis direct** | 🟡 B | **2 fichiers en violation** (automod backend, audit watched_users). |
| **Templates flavor** | 🟡 B+ | Coude templates (steal/heist/prank/refuser) **migrés en DB**. **Blackjack messages** restent locaux. |
| **Robustesse erreurs** | 🟡 B | 131 `.unwrap()`/`.expect()` à éplucher (certains légitimes, beaucoup risqués). |
| **Warnings** | 🟢 A | **0 warning** sur `cargo check --bins --tests`. |
| **Tests** | 🟡 B | 658 tests passent. Couverture inégale entre modules — slot/wheel/voice ont des `tests/`, les autres très peu. |

---

## 3. Conformité par règle d'or

### 3.1 Pas de métier persistant côté bot

✅ **Excellent**. Après les Phases 1/2/3 :
- 0 RNG décisionnel persisté côté bot (tous migrés API : tout-ou-rien, travaux, voler, assurance scam, prank fake_amount)
- 0 catalogue de templates dupliqué (110+ templates en DB `coude_flavor_templates`)
- Toutes les décisions économiques (gain/perte coins, level-up, débit wallet) passent par l'API
- Toutes les validations métier (cooldowns, level minimum, solde) sont rejetées par l'API

⚠️ **2 RNG résiduels — analyse** :

| Fichier | Usage | Verdict |
|---|---|---|
| `security/detectors/captcha.rs:86-94` | Génération de questions math (`a + b` + 1 mauvaise réponse) | 🟢 OK — UX éphémère, pas persisté |
| `blackjack/messages.rs:41` | `pick_random` sur 4 arrays de templates flavor (BJ_NATURAL/WIN/BUST/LOSE) | 🟡 À migrer dans `coude_flavor_templates` (mêmes clés `bj_natural`, `bj_win`, etc.) |

### 3.2 Pas d'accès DB/Redis direct

❌ **2 violations** :

#### A. `modules/automod/backend.rs:364`

```rust
let http_client = reqwest::Client::new();
```

Le bot fait un **appel HTTP direct** vers un service externe (probablement l'inference IA ou un webhook). À encapsuler dans un `api_client` ou un service shared dédié.

#### B. `modules/audit/watched_users.rs:10`

```rust
use redis::AsyncCommands;
```

Le bot lit/écrit Redis directement pour la surveillance des watched users. **Devrait passer par l'API** (qui a déjà un `WatchedUserRepository`) ou par le bus d'événements (déjà utilisé pour cleanup blackjack et cooldowns).

### 3.3 Présentation seulement

✅ Excellent. Les commandes `/coude/*` sont devenues des fines couches d'orchestration UI :
- `tout_ou_rien.rs` (134 lignes) : 1 appel API + animation 10s + embed
- `travaux.rs` (~80 lignes) : 1 appel API + embed
- `voler.rs` reste 600+ lignes mais c'est de la **présentation** (rolls détaillés, boutons défense, embed riche). Pas de logique métier.

### 3.4 `api_client` par module

✅ **15/15 modules** ont un `api_client.rs` (ou `api_client/` répertoire pour `coude` qui en a 18 sous-fichiers).

Pattern uniforme : chaque module expose des **DTO de réponse** (`Player`, `Combat`, `HeistResult`, etc.) en interne, et des méthodes `async fn` qui appellent `BaseApiClient::get_json/post_json` ou `SentinelGrpcClient::*`.

### 3.5 Fallbacks bannis

✅ **Toutes les arrays Rust de templates dupliquées sont supprimées** (Phase 3 finalisation) :
- `STEAL_SUCCESS_AFK/FIGHT/FAIL` (55 templates) → DB
- `HEIST_SUCCESS/FAIL` (40 templates) → DB
- `SCOOP_TEMPLATES`, `FAUX_APPEL_MESSAGES` (15 templates) → DB
- `SHAME_MESSAGES` (10 templates, refuser) → DB
- ~120 lignes de Rust supprimées

Comportement : si l'API down → message `"API indispo, veuillez reessayer plus tard"`. **Plus de fallback local**.

### 3.6 Bonnes pratiques observées

✅ `event_bus::listen_stream_group` utilisé proprement dans 7 modules (audit watched_users refresh, blackjack afk_cleanup, tournament events, daily chaos, ticket events, moderation events).
✅ `TypeMapKey` pour partager les clients (`GameApiKey`, `GrpcClientKey`, `ChannelManagerKey`) — pattern serenity standard.
✅ `CoudeConfig::load(api, guild_id)` centralise la lecture de la config guild — 1 seul appel par commande.
✅ Module `cleanup` dédié pour les jobs de maintenance (purge channels stale).
✅ Branding/copy partagé via `sentinel_shared::branding`.
✅ Aucun `unimplemented!()` runtime, 1 `panic!()` (dans `games/emoji.rs` — à vérifier).

---

## 4. Pain points & code smell

### 4.1 `131 .unwrap()/.expect()` à éplucher

Réparti sur 63 fichiers. Catégories :

| Catégorie | Nombre approx | Risque |
|---|---|---|
| `Mutex::lock().unwrap()` | ~30 | 🟢 Faible (poisoned mutex = bug fatal de toute façon) |
| `ctx.data.read().get::<Key>().unwrap()` | ~20 | 🟢 Faible (TypeMapKey enregistré au boot) |
| `parse::<u64>().unwrap()` sur Discord ID connu | ~15 | 🟢 Faible |
| `.expect("Erreur creation client Discord")` au boot | ~5 | 🟢 OK (fail-fast au démarrage) |
| **Slash command parsing** (option absente) | ~30 | 🟠 Modéré (rejet UX correct mais panique possible) |
| **HTTP/JSON unwraps** | ~20 | 🔴 Risqué (network failure → panic) |
| **Fichiers I/O** (export/transcript) | ~10 | 🟠 Modéré |

**Fix** : audit ciblé fichier par fichier pour les ~40 cas non-triviaux (HTTP/IO). Beaucoup peuvent être convertis en `?` avec un retour `Result` ou en log + early-return.

### 4.2 Tests inégaux par module

| Module | Lignes | Tests dédiés | Couverture estimée |
|---|---|---|---|
| coude | 13 539 | nombreux dans `tests/` (achievements, milestones, ultimates) | 🟢 Bonne |
| moderation | 5 482 | quelques | 🟡 Moyenne |
| voice | 4 415 | `tests/` dédié | 🟢 Bonne |
| automod | 3 016 | quelques détecteurs | 🟡 Moyenne |
| security | 2 833 | rien apparent | 🔴 Faible |
| tickets | 2 840 | quelques helpers | 🟡 Moyenne |
| progression | 2 731 | rien | 🔴 Faible |
| audit | 2 579 | rien | 🔴 Faible |
| blackjack | 2 204 | channel_manager seulement | 🟡 Faible |
| community | 2 086 | rien | 🔴 Faible |
| slot | 1 299 | `tests/` | 🟢 Bonne |
| games | 1 020 | rien | 🔴 Faible |
| cleanup | 743 | rien | 🔴 Faible |
| welcome | 565 | rien | 🔴 Faible |
| wheel | 503 | `tests/` | 🟢 Bonne |

**Fix** : focus tests sur les flows critiques de `security` (anti-raid), `audit` (event handlers), `progression` (XP calc), `blackjack` (logique multi-table), `community` (sponsorship).

### 4.3 Module `coude` ≈ 30% du bot

13 539 lignes pour 86 fichiers. Domaine vaste mais très bien organisé : 18 fichiers `api_client/`, 35 commandes, 7 helpers (catalog, channel_check, daily_chaos_events, etc.).

Pas un problème en soi — Coup de Coude est le plus gros feature, son volume est légitime. À surveiller seulement si la croissance future devient hors de contrôle.

### 4.4 Module `moderation` — duplications potentielles

5 482 lignes pour 26 fichiers, 1 commande par type d'action (ban, mute, warn, expirations, evidence, etc.). Beaucoup de duplication probable dans la construction d'embeds et la lecture d'audit. Candidat à un refactor "command base trait".

### 4.5 `automod/backend.rs` — vrai god-file

3 016 lignes. Inférence + appel HTTP externe + dispatching des verdicts + logging. **À découper** en sous-modules : `inference_client`, `verdict_handler`, `audit_logger`.

---

## 5. Plan de correction priorisé

### Phase A — P0, ~0.5 jour

1. **Migrer les 4 arrays `BJ_*` de `blackjack/messages.rs`** dans `coude_flavor_templates` (clés `bj_natural`, `bj_win`, `bj_bust`, `bj_lose`). Bot consomme via `api.random_flavor`. Plus aucun RNG flavor côté bot. Migration SQL + ~30 lignes Rust supprimées.

2. **Sortir `automod/backend.rs:364` du `reqwest::Client::new()`** : créer un `automod::inference_client.rs` ou utiliser le `SentinelGrpcClient` si l'inference passe par l'API.

3. **Sortir `audit/watched_users.rs:10` du `redis::AsyncCommands`** : passer soit par l'API (`WatchedUserRepository` existe déjà côté API), soit par un consumer event_bus dédié.

### Phase B — P1, ~2-3 jours

4. **Audit des 40 `.unwrap()/.expect()` à risque** (HTTP/IO/parsing dynamique). Convertir en `?` avec retour `Result<()>` ou en log + early return.

5. **Ajouter une couche de tests pour `security`, `audit`, `progression`, `community`**. Focus sur les detectors anti-raid, event handlers, et les calculs déterministes.

6. **Découper `automod/backend.rs`** (3 016 lignes) en sous-modules cohérents.

### Phase C — P2, long terme

7. **Tests d'intégration bot ↔ API** : aujourd'hui les tests bot sont unitaires (logique présentation isolée). Un harness e2e qui boot un mock API + lance des slash commands serait précieux pour les flows critiques.

8. **Métrique de duplication dans `moderation/commands/*`** : extraire un `BaseModeration trait` si pertinent.

9. **Décision sur le panic dans `games/emoji.rs`** : convertir en error path ou justifier.

---

## 6. Métriques

| Métrique | Valeur | Cible |
|---|---|---|
| Fichiers `.rs` | 264 | — |
| Lignes (approx) | ~46 000 | — |
| Modules | 15 | — |
| Tests bot | **658 ✓** | — |
| Warnings cargo | **0** | 0 |
| Métier persistant côté bot | **0** | 0 ✅ |
| Catalogues flavor dupliqués | 4 (blackjack BJ_*) | 0 |
| Accès DB/Redis hors API | 2 (`automod/backend`, `audit/watched_users`) | 0 |
| `.unwrap()`/`.expect()` à risque | ~40 (sur 131 totaux) | <5 |
| Modules sans tests | 8/15 | 0-2 |

---

## 7. Comparaison avec l'API

| Aspect | API | Bot |
|---|---|---|
| Architecture formelle | Hexagonale (ports/adapters) | Modulaire par feature |
| Dépendance domain pure | ✅ Oui | N/A (pas de domain au sens strict) |
| Contrats explicites | Ports inbound/outbound | api_client par module |
| Tests | 2524 lib (riche) | 658 (inégal) |
| RNG métier | Tout côté API ✅ | 0 RNG persisté ✅ |
| Couplage avec drivers | 6 résidus à nettoyer | 2 violations |
| Verdict global | 🟢 B+ → A− | 🟢 A− |

Les deux crates sont aujourd'hui **alignées sur les principes** : l'API porte le métier, le bot porte la présentation. La séparation est propre et défendable.

---

## 8. Ce qui ne sera PAS dans cet audit

- Performance gateway Discord (cache settings, sharding) — out of scope
- Choix UX des embeds (couleurs, emojis, formulations) — pas de l'archi
- Stratégie de sharding/replicas du bot — opérationnel, pas archi
- Sécurité des slash command (rate-limit, abus) — couvert par `sentinel_shared::ratelimit`

---

## 9. Historique récent (côté bot)

- **2026-04-25** Phase 1 leftovers : 12 magic constants → `CoudeConfig` getters dans 8 commandes (`prestige`, `ultimate`, `prank`, `coalition`, `contribuer_prime`, `voler`, `tout_ou_rien`, `assurance`).
- **2026-04-26** Phase 2 (RNG) : refactor de `tout_ou_rien.rs`, `travaux.rs`, `assurance.rs`, `voler.rs` pour appeler les nouveaux endpoints API. Animations + embeds inchangés.
- **2026-04-27** Phase 3 finalisation : suppression de **120+ lignes de templates** dupliqués + 4 fonctions `pick_random` + `SHAME_MESSAGES` migré en DB. Plus de fallback local — bot strict-dépendant API.
- **2026-04-27** Bug fix Blackjack : la table ne ferme plus mid-game (touch_activity côté API).
- **2026-04-27** Cleanup : `friendly_duel::FriendlyDuelResp`, `BuyInsuranceResolved`, `RollStealResp`, `FlavorTemplateResp` re-exports morts supprimés. `ULTIMATE_UNLOCK_LEVEL` const supprimée. `VoiceConfigResponse.vote_kick_timeout_secs` field non lu supprimé.

---

## 10. TL;DR — actions recommandées

```
🔴 P0 (0.5 jour, low-risk)
  □ Migrer BJ_NATURAL/WIN/BUST/LOSE → coude_flavor_templates
  □ Sortir automod/backend.rs du reqwest::Client direct
  □ Sortir audit/watched_users.rs du redis::AsyncCommands

🟠 P1 (2-3 jours, modéré)
  □ Auditer + remplacer ~40 .unwrap()/.expect() à risque
  □ Ajouter tests unitaires pour security/audit/progression/community
  □ Découper automod/backend.rs (3k lignes)

🟡 P2 (long terme)
  □ Harness e2e bot↔API pour les flows critiques
  □ Refactor moderation/commands/* (BaseModeration trait ?)
  □ Vérifier le panic! dans games/emoji.rs
```

**Verdict final** : le bot est **dans un état sain**. Pas de métier qui ne devrait pas y être, pas de duplication massive, pas de god-files (sauf automod/backend), structure modulaire consistante. Les améliorations sont du polish, pas du refactor architectural majeur.
