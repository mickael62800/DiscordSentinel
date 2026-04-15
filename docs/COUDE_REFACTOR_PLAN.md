# Plan de refacto Coup de Coude : centraliser la logique dans l'API

## Constat (audit du 15/04/2026)

Actuellement la logique métier et l'accès SQL du jeu Coup de Coude sont éparpillés dans 3 endroits :

| Endroit | Lignes | État | Contenu |
|---|---|---|---|
| `services/api/src/application/manage_coude_combats_service.rs` | 154 | ✓ actif | Use case combats (light) |
| `services/workers/coude-worker/src/combat_engine/combat.rs` | 802 | ✗ actif mais hors API | Résolution combat complète |
| `services/workers/coude-worker/src/jobs/resolve_betting.rs` | 683 | ✗ actif mais hors API | Orchestration + 55 requêtes SQL directes |
| `services/workers/coude-worker/src/jobs/wallet_log.rs` | 101 | ✗ actif mais hors API | credit_and_log / debit_and_log |
| `bots/coude-bot/src/db.rs` | 1327 | ⚠️ dead code | Ancienne couche DB du bot, plus appelée |
| `bots/coude-bot/src/game/` (combat, classes, chaos, shop, progression) | ~1260 | ⚠️ dead code | Ancienne logique jeu du bot |

**Total à nettoyer/migrer : ~4200 lignes.**

## Principe cible

```
┌──────────┐           ┌─────────┐           ┌─────────────┐
│ coude-bot│──gRPC/HTTP│   API   │──sqlx────│  Postgres   │
│  (thin)  │           │  (DDD)  │           └─────────────┘
└──────────┘           │         │
                       │ combat  │
┌──────────┐           │ engine  │
│  worker  │──gRPC/HTTP│ lives   │
│ (thin)   │           │  here   │
└──────────┘           └─────────┘
```

- **Bot** : IO Discord uniquement. Aucun SQL, aucune logique de combat. Zéro accès `PgPool`.
- **Worker** : déclencheur + retry/queue. Appelle des endpoints API atomiques type `/api/coude/combats/resolve-betting-batch` qui font tout le boulot côté API et retournent les résultats à broadcaster.
- **API** : combat_engine + toutes les transitions d'état + toutes les écritures DB (repos DDD) + transactions atomiques.

## Phases de migration

Chaque phase est déployable indépendamment. Après chaque phase : rebuild + test en prod. Rollback = revert commit.

---

### Phase 0 — Préparation (0.5 j)

**Objectif** : baseline propre, nettoyer le dead code pour réduire la surface d'audit.

1. **Supprimer `bots/coude-bot/src/db.rs`** — 1327 lignes jamais appelées. Vérifier avec `grep -r "use crate::db::" bots/coude-bot/src` avant de dropper.
2. **Supprimer `bots/coude-bot/src/game/combat.rs`** — 805 lignes dupliquées avec le worker. Vérifier qu'aucune commande bot ne l'importe (seule la résolution surprise via `resolve_combat_internal` pourrait l'utiliser — à tracer).
3. **Supprimer `bots/coude-bot/src/game/{classes,chaos,shop,progression}.rs`** si dead. Sinon noter les usages restants.
4. Retirer `src/db.rs` et `src/game` du `mod.rs` du bot.
5. `cargo check -p coude-bot` doit passer.

**Blocage possible** : si `resolve_combat_internal` (pour attaque surprise / bloodbath) utilise encore `game::combat`. Dans ce cas, migrer ces deux flows à faire un appel gRPC `ResolveCombatNow` à l'API au lieu de résoudre localement.

**Tests** : `/coude @user` classique + attaque surprise + bloodbath.

---

### Phase 1 — Déplacer `combat_engine` du worker vers l'API (1 j)

**Objectif** : le moteur de combat (damage calc, classes, chaos, rounds) vit dans l'API.

1. Créer `services/api/src/domain/services/coude_combat_engine/` avec :
   - `mod.rs`, `combat.rs`, `classes.rs`, `chaos.rs` (copiés depuis `services/workers/coude-worker/src/combat_engine/`)
2. Adapter les imports (remplacer les structs `PlayerLite` worker par les entités domaine `CoudePlayer`).
3. Exposer une fn pure `resolve_combat(attacker, defender, mise, special_atk, special_def, events) -> CombatResult` (pas de SQL, pas d'IO).
4. Le worker garde sa copie pour l'instant — on bascule à la Phase 2. Le but ici est juste d'avoir la même logique côté API.
5. Tests unitaires du combat_engine côté API (quelques cas : fourbe vs tank, explosion, draw, giant killer).

**Rollback** : la nouvelle logique n'est pas encore appelée, donc rollback = delete le dossier.

---

### Phase 2 — Nouvel endpoint `/api/coude/combats/resolve-betting-batch` (1 j)

**Objectif** : le worker appelle un SEUL endpoint qui fait TOUT.

1. Créer `POST /api/coude/combats/resolve-betting-batch` côté API :
   - Charge tous les combats `status='betting'` dont le délai est écoulé (via `FOR UPDATE SKIP LOCKED`).
   - Pour chacun, appelle `combat_engine::resolve_combat`.
   - Dans une **transaction unique** par combat :
     - Update `coude_combats` (status, winner, coins_transferred, etc.)
     - Debit loser (capped on balance)
     - Credit winner
     - Update stats (`coude_players.total_wins`, `total_losses`, `total_earned`, `total_lost`, `xp`, `chaos_events`)
     - Update HP
     - Resolve bets (via `coude_bets_uc`)
     - Gestion vol_coins / explosion / assurance
   - Retourne `Vec<ResolvedCombatDto>` avec tous les champs nécessaires au worker pour broadcaster sur Discord (`channel_id`, `message_id`, `result_message`, `color`).
2. Créer un use case `ResolveBettingBatchUseCase` dans `application/` qui orchestre. C'est ici que vivra l'équivalent de `resolve_single` actuel.
3. Use cases satellites déjà existants réutilisés : `ManageCoudeBetsUseCase.resolve`, `ManageCoudeInventoryUseCase.get_active_insurance/expire_insurance`, `ManageCoudePlayersUseCase.add_xp`.

**Tests** : appeler l'endpoint via curl avec un combat de test + vérifier que toutes les tables sont mises à jour.

---

### Phase 3 — Worker devient thin (0.5 j)

**Objectif** : `resolve_betting.rs` ne contient plus qu'un appel HTTP et un loop Discord.

1. Remplacer tout le body de `run()` par :
   ```rust
   let resolved = api_client.resolve_betting_batch().await?;
   for combat in resolved {
       post_result_to_discord(bot_token, &combat.channel_id, combat.message_id.as_deref(), &combat.result_message).await;
   }
   ```
2. **Supprimer** `services/workers/coude-worker/src/combat_engine/` (le moteur vit dans l'API maintenant).
3. **Supprimer** `services/workers/coude-worker/src/jobs/wallet_log.rs` (les debit/credit passent par l'API via la transaction du use case).
4. Garder `post_result_to_discord` + `refund_all_bets` ? → Non, `refund_all_bets` est appelé dans le batch API maintenant. Seul `post_result_to_discord` reste (IO Discord pur).
5. `cargo check -p sentinel-coude-worker` doit passer avec ~50 lignes de code utile.

**Tests** : lancer un vrai combat en prod, vérifier que le worker résout et post le message.

---

### Phase 4 — HP regen via API (0.5 j)

**Objectif** : `hp_regen.rs` devient un simple appel HTTP.

1. Créer `POST /api/coude/hp-regen/tick` côté API qui exécute le même UPDATE CTE avec l'exclusion `NOT EXISTS` des combats actifs.
2. Worker `hp_regen.rs` → `api_client.hp_regen_tick().await`.

---

### Phase 5 — Bot : vérifier qu'il ne reste aucun SQL (0.25 j)

1. `grep -r "sqlx::" bots/coude-bot/src` → doit retourner vide.
2. `grep -r "PgPool" bots/coude-bot/src` → vide.
3. `grep -r "DATABASE_URL" bots/coude-bot/src` → vide.
4. Retirer `sqlx` et `sqlx-postgres` du `Cargo.toml` de `coude-bot` si plus utilisés.
5. `cargo check -p coude-bot`.

---

### Phase 6 — Worker : vérifier qu'il ne reste presque aucun SQL (0.25 j)

1. `grep -r "sqlx::" services/workers/coude-worker/src` → idéalement 0. Le seul SQL acceptable serait un check de santé ou une lecture de config batch.
2. Retirer `sqlx` du `Cargo.toml` si plus utilisé.
3. `cargo check -p sentinel-coude-worker`.

---

## Checklist de tests end-to-end (à exécuter après chaque phase 2-4)

- [ ] `/coude @user` crée un combat, défenseur accepte, résolution normale
- [ ] Victoire normale → coins transférés correctement, stats mises à jour
- [ ] Victoire Fourbe → bonus appliqué et capé sur solde perdant
- [ ] Défaite avec assurance normale → perte réduite 50%
- [ ] Défaite avec assurance arnaque → perte doublée
- [ ] Explosion → les 2 joueurs perdent, paris remboursés
- [ ] Draw → personne ne perd, paris remboursés
- [ ] Vol chaos → capé sur solde
- [ ] Giant killer → +XP bonus
- [ ] Tank vs Tank → pas de spirale 1 dmg
- [ ] Bourrin à 30% HP → berserker actif
- [ ] HP regen après combat (vérifier que le tick suivant régénère bien)
- [ ] Combat expiré après 24h
- [ ] Refus du défi
- [ ] Attaque surprise
- [ ] Bloodbath event
- [ ] `/pari` pendant la phase betting, résolution correcte
- [ ] `/pari` refusé si combat déjà en resolving (race)
- [ ] `/assurance` refusée si déjà active (race)
- [ ] Duel bloqué si défenseur à 0 coin

## Risques et mitigations

| Risque | Mitigation |
|---|---|
| Comportement combat change subtilement lors du port | Tests unitaires du combat_engine avant Phase 2, comparaison worker vs API sur dataset de combats existants |
| Perf : 1 transaction par combat peut ralentir le batch | Pool sqlx déjà tuned côté API, les batchs actuels sont ~10 combats max |
| Rollback partiel si Phase 2 pète en prod | Feature flag env var `COUDE_RESOLVE_VIA_API=true` dans le worker pour basculer entre l'ancienne et la nouvelle résolution en cas de pb |
| Perte de combats pendant la migration | Déployer hors pic d'activité + stop du worker → déploiement API → start worker |

## Effort total estimé

- Phase 0 : 0.5 j
- Phase 1 : 1 j
- Phase 2 : 1 j (la plus risquée)
- Phase 3 : 0.5 j
- Phase 4 : 0.5 j
- Phases 5-6 : 0.5 j

**Total : ~4 jours** de travail concentré. À étaler sur 1-2 semaines avec validation après chaque phase.

## Ordre de priorité si on ne fait pas tout

1. **Phase 0** (nettoyage dead code) — gain énorme de clarté pour zéro risque.
2. **Phase 5** (vérification bot SQL-free) — confirme que le bot est déjà propre.
3. **Phase 2 + 3** (combat engine + resolve_betting dans l'API) — le gros morceau, principal bénéfice architectural.
4. **Phase 4** (hp_regen) — petit, peu d'impact, à faire en dernier.
