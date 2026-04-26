# 🏛️ Audit architectural — Module Coup de Coude

> **Date** : 2026-04-26
> **Périmètre** : `services/api/src/` (côté API) + `bots/sentinel-bot/src/modules/coude/` (côté bot)
> **Méthode** : 2 agents d'analyse en parallèle (hexagonale API + logique métier bot) sur ~45 000 lignes
> **Objectif** : identifier les écarts à l'architecture cible et préparer un plan de refacto

---

## 1. Architecture cible (rappel)

### Côté API (hexagonale)

```
domain/
├── entities/        # structs + fonctions PURES (no I/O, no async)
├── value_objects/   # types primitifs validés
└── services/        # algorithmes purs (combat engine, calculs)

ports/
├── inbound/         # traits use case (entrée du domaine)
└── outbound/        # traits repository (sortie du domaine)

application/         # services qui implémentent les use cases,
                     # orchestrent ports + domain (pas de logique métier nue)

adapters/
├── inbound/http/    # handlers HTTP minces : DTO ↔ use case
├── inbound/grpc/    # handlers gRPC minces
└── outbound/postgres/  # implémentations des ports outbound
```

**Règle d'or** : `domain` ne dépend de **rien**. Les flèches d'import vont de l'extérieur vers l'intérieur.

### Côté bot

Le bot doit être un **client thin Discord ↔ API** :

| Doit faire | Ne doit PAS faire |
|---|---|
| Parser les inputs Discord | Calculer des montants/multiplicateurs/probabilités |
| Construire embeds & boutons | Décider des règles de gameplay (qui gagne, combien) |
| Appeler `api_client::*` | Faire de la validation métier (au-delà du parsing) |
| Afficher les réponses | Connaître seuils/niveaux/durées de gameplay |
|  | Implémenter du RNG sur du state persistant |
|  | Lister en dur les catalogues (items, classes, events) |

---

## 2. Vue d'ensemble — verdict général

| Côté | Note | Verdict |
|---|---|---|
| **API (hexagonale)** | 🟢 **B+** | Globalement sain. Domain pur, adapters fins, ports clairs. Quelques P0/P1 corrigibles rapidement. |
| **Bot (thin client)** | 🟠 **C–** | **40-50 % de logique métier** dans le bot. RNG décisionnels, seuils hardcodés, catalogues textuels. Refacto significatif nécessaire. |

> **Interprétation** : on a livré le gameplay vite (MVP, hors-spec hexagonal), surtout côté bot. L'API tient bien la ligne, mais le bot est devenu un mini-client lourd.

---

## 3. 🟦 Côté API — Audit hexagonale

### 🔴 P0 — Violations majeures

#### 1. `unimplemented!()` dans des default impls de port

- **Fichier** : `services/api/src/ports/outbound/coude_combat_repository.rs:126,139`
- **Problème** : `purge_guild_subsystem()` et `count_defeats_today()` ont `unimplemented!()` comme default. Si un mock ou un adapter omet de les implémenter, le binaire **panique en runtime**.
- **Fix** : remplacer par `Err(DomainError::NotImplemented(...))` ou rendre la méthode non-default (forcer chaque impl à la fournir).

#### 2. Couplage `use case → use case`

- **Fichier** : `services/api/src/application/manage_coude_bets_service.rs:15,20`
- **Problème** : `ManageCoudeBetsService` injecte `combats_uc: Arc<dyn ManageCoudeCombatsUseCase>` directement. Or les use cases doivent dépendre des **ports outbound**, pas d'autres use cases (sinon couplage transitif et tests cassés).
- **Fix** : créer un port outbound `CombatQueryRepository` (lecture seule du statut combat) et injecter ça à la place.

#### 3. Mélange config-loading + service métier

- **Fichier** : `services/api/src/application/manage_coude_combats_service.rs:42-56`
- **Problème** : `load_balance()` est async, dépend d'un `bot_config_repo` optionnel, et fallback silencieux sur `default()` si réseau KO. Mélange orchestration et chargement de config.
- **Fix** : extraire un port dédié `BalanceParamsLoader` (chargement obligatoire au boot) ou un cache injecté.

### 🟠 P1 — Violations notables

#### 4. Logique métier fragmentée entre `domain` et `application`

- **Fichier** : `services/api/src/application/manage_coude_economy_service.rs:91-100,174-246`
- **Problème** : la formule du bonus saisonnier (`stolen * (mult - 1.0)`) est en application, mais `theme_for_season()` vit en domain. La règle « Saison du Vol » est éclatée.
- **Fix** : centraliser tout le calcul dans un domain service `season_bonus.rs`.

#### 5. Validation orchestrée + branchements optionnels

- **Fichier** : `services/api/src/application/manage_coude_combats_service.rs:110-168`
- **Problème** : la création de combat coordonne 4 validations (mise > 0, attacker ≠ defender, HP min, surprise gate) dont 2 dépendent d'un `players_uc` optionnel branché via `with_surprise_gate()`. Couplage tacite difficile à auditer.
- **Fix** : extraire les validations stateless en pure functions du domain ; garder en application uniquement la coordination des dépendances optionnelles.

### 🟡 P2 — Violations mineures

#### 6. Clamp métier dans l'application

- **Fichier** : `services/api/src/application/manage_coude_economy_service.rs:194`
- **Problème** : `stolen = amount.min(victim_coins)` est en application — règle métier qui devrait être pure.
- **Fix** : `domain/entities/coude_economy.rs::clamp_steal_amount(requested, available)`.

#### 7. `sqlx::Type` derive sur un value object

- **Fichier** : `services/api/src/domain/value_objects/coude_class.rs:8`
- **Problème** : `#[derive(sqlx::Type)]` lie le VO au driver SQL. Couplage compile-time discutable.
- **Fix** : envisager un newtype wrapper côté adapter, ou documenter que c'est intentionnel (enum Postgres-native).

### ✅ Bonnes pratiques observées (côté API)

1. **Handlers HTTP/gRPC minces** : `adapters/inbound/coude/*.rs` font de la traduction DTO ↔ use case sans logique métier.
2. **Domain services purs** : `domain/services/coude_combat_engine/{combat,chaos,progression,classes}.rs` — aucun `async`, `sqlx` ou `reqwest`. Le moteur de combat est testable en isolation.
3. **Ports outbound bien définis** : `CoudeCombatRepository`, `CoudeEconomyRepository`, `CoudeBetRepository` n'exposent que des entités domain (pas de `PgRow` ni `serde_json::Value`).
4. **Tests isolés** : tests domain dans `domain/entities/tests/`, services testés avec mocks cohérents.

---

## 4. 🟧 Côté bot — Logique métier détectée

### 🔴 RNG décisionnel persisté côté bot (6 cas)

> Ces RNG décident du résultat d'une action **persistée** (coins, niveaux, états). Doivent être côté API.

| Fichier | Ligne | Effet |
|---|---|---|
| `commands/voler.rs` | 451-452 | `rng.gen_range(1..=20)` — d20 du vol → coins persistants |
| `commands/voler.rs` | 541-546 | `rng.gen_range(10.0..=15.0)` — % volé (AFK/actif) |
| `commands/tout_ou_rien.rs` | 133-134 | `rng.gen_range(0.0..1.0)` — win/lose all-in |
| `commands/travaux.rs` | 124-128 | `rng.gen_bool(0.5)` — succès/échec tâche prison + crédit coins |
| `commands/assurance.rs` | 165-166 | `rng.gen_range(1..=100)` — détermine si scam (persiste sur la souscription) |
| `commands/prank.rs` | 178-179 | `rng.gen_range(5..=50) * 1000` — montant faux gain (mineur, affichage) |

**Risque** : un joueur qui inspecte le client bot pourrait théoriquement biaiser ces rolls (faible vu Discord/Rust mais le principe est cassé). Plus critique : impossible d'auditer/rejouer côté API.

### 🟠 Validations métier côté bot (7 cas)

| Fichier | Ligne | Règle hardcodée |
|---|---|---|
| `commands/coude/mod.rs` | 177 | `hp_pct_now < 10` (seuil de blocage combat) |
| `commands/coude/mod.rs` | 327-330 | Seuils HP 25/50 % (warnings UI — acceptable) |
| `commands/voler.rs` | 260 | `target_player.coins < 10` (min volable) |
| `commands/assurance.rs` | 134 | `if level >= 5 { 2 } else { 1 }` slots **(déjà migrable, cf. `assurance_extra_slot_level`)** |
| `commands/prank.rs` | 147 | `player.coins < cost` (devrait être API qui rejette) |
| `commands/prestige.rs` | 50 | `level < PRESTIGE_UNLOCK_LEVEL (25)` |
| `commands/ultimate.rs` | 128 | `level >= ULTIMATE_UNLOCK_LEVEL` |

**Problème** : si l'API change le seuil (via la migration 170 par ex.), le bot continue d'utiliser sa propre constante hardcodée → comportement incohérent.

### 🟡 Constantes magiques métier dans le bot (12+)

| Fichier | Constantes | Devraient venir de |
|---|---|---|
| `commands/assurance.rs:27-46` | TIERS (durations 86_400/604_800/2_592_000, multipliers 1/6/22) | API catalog |
| `commands/prestige.rs:16` | `PRESTIGE_UNLOCK_LEVEL = 25` | `bot_guild_config` (déjà dispo via 170) |
| `commands/ultimate.rs:17` | `ULTIMATE_UNLOCK_LEVEL` | API catalog |
| `commands/prank.rs:22-24` | `PRANK_BRAQUAGE_COST=100`, `PRANK_APPEL_COST=50` | API |
| `commands/travaux.rs:18-23` | Cooldown, success%, coins min/max | API |
| `commands/tout_ou_rien.rs:24-25` | `ANIMATION_DURATION_SECS=10`, `MIN_BALANCE_FOR_PLAY=100` | API |
| `commands/coalition.rs:14` | `COST_PER_MEMBER=500` | API |
| `commands/contribuer_prime.rs:14` | `MIN_CONTRIBUTION=50` | API |
| `commands/refuser.rs:66` | `86400` (durée hardcodée) | API |
| `commands/accepter.rs:285` | `ANIMATED_COMBAT_MISE_THRESHOLD=500` | Config bot OK |
| `commands/memorial.rs:15` | `MEMORIAL_LIMIT=10` | Config bot OK |

### 🔵 Catalogues textuels hardcodés (~110 templates)

| Fichier | Lignes | Contenu |
|---|---|---|
| `commands/voler.rs:45-107` | 62 lignes | 45 templates `STEAL_SUCCESS_AFK`, `STEAL_SUCCESS_FIGHT`, `STEAL_FAIL` |
| `commands/braquage.rs:19-63` | 44 lignes | 40 templates `HEIST_SUCCESS`, `HEIST_FAIL` |
| `commands/prank.rs:26-45` | 19 lignes | 15 templates `SCOOP_TEMPLATES`, `FAUX_APPEL_MESSAGES` |
| `commands/travaux.rs:25-55` | 30 lignes | 3 tâches + 8 flavors `SUCCESS_FLAVORS`, `FAIL_FLAVORS` |

**Problème** : pour ajouter une variante de raillerie, il faut redéployer le bot. Un module taunts existe déjà côté API (`manage_coude_taunts_service`) — extension naturelle.

---

## 5. 🎯 Plan d'action recommandé

### Phase 1 — Quick wins (1-2 jours)

1. **API P0 #1** : remplacer `unimplemented!()` par `DomainError::NotImplemented` (1 commit, ~10 lignes).
2. **API P0 #2** : extraire un port `CombatQueryRepository` pour découpler `ManageCoudeBetsService` (~50 lignes).
3. **Bot 🟠** : pour chaque validation hardcodée (`level >= 5`, `level >= 25`, etc.), retirer le check côté bot et laisser l'API rejeter. Le bot affiche le message d'erreur API.
4. **Bot 🟡** : exposer les 12 constantes magiques au `config_schema` (suite naturelle de la migration 170).

### Phase 2 — Refacto métier (1 semaine)

5. **Migrer `/voler` côté API** : créer un use case `ResolveStealUseCase` qui exécute le RNG, applique les règles, retourne le verdict. Bot devient un appel API + affichage.
6. **Migrer `/tout-ou-rien` côté API** : idem, RNG + persistance dans un seul appel.
7. **Migrer `/travaux` côté API** : sélection tâche + résultat dans un endpoint dédié.
8. **Migrer `/assurance` scam roll côté API** : déplacer le `gen_range(1..=100)` dans `buy_insurance` côté API (le serveur décide si c'est un scam, le bot reçoit le verdict).

### Phase 3 — Catalogues (1 semaine)

9. **Migrer les ~110 templates** dans une table `coude_flavor_templates` (key, locale, weight, content). Endpoint `GET /api/coude/flavor/{key}/random` qui retourne un template au hasard. Les commandes bot consomment cet endpoint.
10. **Centraliser `assurance.rs` TIERS** : table `coude_insurance_tiers` (key, label, duration_secs, multiplier).

### Phase 4 — Domain refacto API (long terme)

11. **API P1 #4** : centraliser `season_bonus` calcul en domain.
12. **API P1 #5** : extraire les validations stateless de `manage_coude_combats_service.create()` en pure functions.
13. **API P2 #6** : `clamp_steal_amount` en domain.
14. **API P2 #7** : décider si on garde `sqlx::Type` sur les value objects ou si on encapsule (décision conventionnelle, pas urgent).

---

## 6. 🧪 Risques de la non-action

| Si on ne fixe rien | Conséquence |
|---|---|
| `unimplemented!()` en prod | Crash si appelé depuis un mock oublié |
| Constantes hardcodées bot | Désynchro silencieuse avec la config admin (migration 170) → bug "ne respecte pas le palier" |
| RNG bot persisté | Difficile à auditer / rejouer / tester en intégration |
| Catalogues hardcodés | Redéploiement bot pour chaque variante de raillerie |
| Couplage `use case → use case` | Mocks cassés, tests fragiles |

---

## 7. 📊 Métriques actuelles

| Métrique | Valeur |
|---|---|
| Fichiers Rust côté API (coude) | ~170 |
| Lignes côté API (coude) | ~30 200 |
| Fichiers Rust côté bot (coude) | 82 |
| Lignes côté bot (coude) | ~14 900 |
| Tests sentinel-api | 2 474 ✅ |
| Tests sentinel-bot | 658 ✅ |
| Migrations Postgres coude | ~50 |
| Commandes Discord | 41 |

---

## 8. 🧭 Ce qui ne sera PAS dans cet audit

- Couverture de tests (déjà auditée précédemment)
- Sécurité / permissions Discord (audité, RAS)
- Bugs métier (audité, fixé dans le commit `282333e`)
- Performance / scalabilité (hors périmètre)

---

*Rédigé le 2026-04-26 par 2 agents d'analyse parallèles + relecture humaine. À discuter en revue de code.*
