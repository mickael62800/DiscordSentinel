# API Refactor — Roadmap

État de l'archi `services/api/` après les PR1 → PR2bis + extraction enums + suppression des re-exports.

Document vivant. Coche au fur et à mesure.

---

## ✅ Déjà fait

| PR | Commit | Effet |
|---|---|---|
| PR1 | `001b947` | `domain/entities/` → 7 bounded contexts (ai/audit/casino/community/coude/moderation/system) |
| PR1bis | `d35bbde` | Même reorga sur `services/`, `value_objects/`, `application/`, `ports/inbound/`, `ports/outbound/`, `handlers/` |
| PR2 | `0b1d17a` | `entities/coude/coude_*.rs` → `*.rs` (drop préfixe) |
| PR2bis | `9415952` | Idem pour `application/coude/`, `ports/{inbound,outbound}/coude/` |
| extract enums | `1806e8f` | 8 enums sortis de `value_objects/` vers `domain/enums/` |
| extract Role | `ad06319` | `Role` enum sorti du middleware HTTP vers `domain/enums/system/role.rs` |
| remove re-exports | `d57e6fd` | Suppression des 291 `pub use` dans les `mod.rs`, paths qualifiés partout |
| fix integration tests | `8d05408` | `cargo test --workspace --tests --no-run` : 0 erreur (avant 298 sur main 🎉) |

**Score actuel** : `cargo check --workspace` + `cargo test --workspace --tests --no-run` → **0 erreur**.

---

## 🟢 Court terme — symétrie & cohérence

Faible risque. Mêmes patterns que PR1bis, scriptable.

### [ ] PR3 — Reorg `adapters/outbound/postgres/`
**Pourquoi** : ~60 fichiers flat alors que `ports/outbound/` est en bounded contexts. Asymétrie qui sera de plus en plus douloureuse.

**Effet attendu** :
```
adapters/outbound/postgres/
├── audit/        — pg_audit_log_repository, pg_security_event_repository, ...
├── casino/       — pg_blackjack_repository, pg_slot_repository, ...
├── community/    — pg_conduct_repository, pg_voice_channel_repository, ...
├── coude/        — pg_bet_repository, pg_player_repository, ... (~20 fichiers)
├── moderation/   — pg_infraction_repository, pg_strike_repository, ...
└── system/       — pg_bot_config_repository, pg_ticket_repository, ...
```

**Risque** : moyen — les implémentations s'importent entre elles parfois (`pg_combat_query_repository` peut dépendre de helpers d'un autre repo). À traiter au cas par cas.

**Scriptable** : oui, même approche que PR1bis (`scripts/remove_reexports.py` peut être adapté).

### [ ] PR4 — Reorg `adapters/inbound/http/dto/`
**Pourquoi** : symétrie avec `handlers/` (qui est en bounded contexts).

**Effet attendu** :
```
adapters/inbound/http/dto/
├── audit/, casino/, community/, coude/, moderation/, system/
```

**Risque** : faible — les DTOs sont uniquement utilisés par les handlers correspondants.

### [ ] PR5 — Supprimer `domain/value_objects/`
**Pourquoi** : ne contient plus que `DetectionFlags` (struct moderation). Le dossier est presque vide, ses backward-compat re-exports sont morts.

**Action** :
- Déplacer `DetectionFlags` vers `entities/moderation/detection_flags.rs`
- Supprimer le dossier `value_objects/` complet
- Retirer `pub mod value_objects;` du `domain/mod.rs`
- Sed les imports : `value_objects::DetectionFlags` → `entities::moderation::detection_flags::DetectionFlags`

**Risque** : faible — un seul type touché.

---

## 🟡 Moyen terme — qualité du code

### [ ] PR6 — Splitter `domain/errors.rs` par bounded context
**Pourquoi** : si `DomainError` est un super-enum avec 50+ variantes, il devient illisible et chaque ajout impacte tous les consommateurs.

**Pré-requis** : auditer le contenu de `domain/errors.rs`. Si c'est un enum minimaliste générique (Validation/NotFound/Internal), garder. Si c'est un super-enum métier, splitter.

**Effet possible** :
```
domain/errors/
├── mod.rs              — DomainError de base (Validation, NotFound, Internal, Conflict)
├── coude.rs            — CoudeError (BetExpired, InsufficientCoins, NotInClass, ...)
├── moderation.rs       — ModerationError (StrikeAlreadyResolved, MuteTooShort, ...)
└── ...
```

**Risque** : moyen — beaucoup de `?` operators à adapter si on change la signature des Result.

### [ ] PR7 — Drop le préfixe `Coude` sur les types
**Pourquoi** : `entities::coude::bet::CoudeBet` est redondant. `entities::coude::bet::Bet` est plus propre. PR2 a fait ça pour les fichiers, pas pour les types.

**Scope** : ~50 types à renommer (`CoudeBet`, `CoudeCombat`, `CoudePlayer`, `ManageCoudeBetsService`, `ManageCoudeBetsUseCase`, `CoudeBetRepository`, etc.) + tous les usages.

**Risque** : élevé — beaucoup de touches mais faisable en sed cascadé. À tester avec le script Python en mode dry-run d'abord.

**Bénéfice** : ~30% du nom devient sémantique au lieu d'être un préfixe.

### [ ] PR8 — Newtypes pour les IDs Discord
**Pourquoi** : actuellement `String` partout. Un `&str` `user_id` peut être confondu avec un `guild_id` au compilo. Newtypes `GuildId(String)`, `UserId(String)`, `MessageId(String)` empêchent ça.

**Scope** : touche les signatures partout (`async fn lookup_role(state: &AppState, user_id: &str, guild_id: &str)` devient `&UserId, &GuildId`).

**Risque** : très élevé — impact sur des centaines de signatures. À faire incrémentalement, contexte par contexte. Bénéfice surtout sur les nouveaux dev qui se prennent les pieds dans les types.

**Approche** : créer `domain/ids.rs` avec les newtypes + Display + From + serde, puis adapter graduellement (commencer par les domain entities, étendre progressivement).

---

## 🔵 Long terme — architecture profonde

### [ ] PR9 — Promouvoir plus de deps en `workspace.dependencies`
**Scope** : `serenity`, `axum`, `redis`, `tonic`, `prost`, `metrics`, etc. (déjà fait : sqlx).

**Effet** : 1 endroit pour bumper une version au lieu de N crates.

**Risque** : nul.

### [ ] PR10 — Appliquer le pattern bounded-context à `bots/sentinel-bot/`
**Pourquoi** : 15 modules existent déjà (audit, automod, blackjack, ...). Pourrait être structuré symétriquement avec l'API.

**Risque** : moyen — le bot a sa propre logique (Serenity event handlers) et un grouping différent peut faire moins de sens. À évaluer.

### [ ] PR11 — Splitter `services/api/` en sous-crates
**Idée** : `api-coude`, `api-moderation`, `api-casino`, ..., et un `api-server` qui les agrège. Permet :
- Builds incrémentaux par bounded context (gain énorme sur cold builds)
- Encapsulation forcée (chaque crate ne voit que les autres via leurs ports publics)
- Tests parallélisés

**Risque** : très élevé — gros chantier de refactor des Cargo.toml + ports croisés.

**ROI** : énorme à long terme si l'API continue de grossir, sinon premature optimization.

### [ ] PR12 — CQRS : séparer query repos vs command repos
**Pourquoi** : les contextes read-heavy (analytics, dashboard, stats) bénéficieraient d'optims spécifiques (vues matérialisées, cache plus agressif).

**Risque** : élevé. Probablement overkill pour 10 guilds.

---

## 📊 Suivi des métriques

À chaque PR, vérifier :

```bash
# Lib + tests doivent rester verts
cargo check --workspace
cargo test --workspace --tests --no-run

# Surveiller la dette
echo "fichiers .rs    : $(find services/api/src -name '*.rs' | wc -l)"
echo "lignes Cargo.lock: $(wc -l < Cargo.lock)"
echo "TODO/FIXME      : $(grep -rn 'TODO\|FIXME' services/api/src/ | wc -l)"
```

---

## 🚦 Ordre recommandé

```
PR3  postgres reorg          ← symétrie
PR4  dto reorg               ← symétrie
PR5  drop value_objects      ← cleanup
─── ↑ rapide, scriptable ───────────────
PR6  split errors            ← qualité
PR7  drop Coude prefix       ← cosmétique mais cohérent
─── ↑ moyen risque ──────────────────────
PR8  newtypes IDs            ← qualité long terme
PR9  workspace deps          ← cosmétique
PR10 bots reorg              ← extension
─── ↑ optionnel ─────────────────────────
PR11 split en sous-crates    ← uniquement si l'API explose
PR12 CQRS                    ← uniquement si charge le justifie
```

---

## 🧠 Principes de design à conserver

- **Bounded context cohérence** : chaque ajout dans `domain/`, `application/`, `ports/`, `adapters/inbound/http/handlers/` doit aller dans un des 7 contextes (ai, audit, casino, community, coude, moderation, system). Si ça n'en colle aucun, créer un nouveau contexte.
- **Pas de re-exports** : les paths sont qualifiés. `use crate::domain::entities::coude::bet::CoudeBet`, jamais `use crate::domain::entities::CoudeBet`.
- **Enums et structs séparés** : enums dans `domain/enums/`, structs dans `entities/`.
- **Ports = traits, Adapters = implémentations** : pas de logique métier dans un adapter, pas de détail technique dans un port.
- **Tests à côté du code** : `module.rs` + `#[path="tests/module.rs"] mod tests;` + `tests/module.rs`. Pattern uniforme.
