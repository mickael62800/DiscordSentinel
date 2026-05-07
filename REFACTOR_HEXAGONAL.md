# Refacto hexagonal — extraction de `sentinel-core`

**Statut : WIP, build cassé** (488 erreurs côté `sentinel-api` après corruption à corriger d'abord — voir « Tâches restantes »).

## Contexte

L'archi hexagonale était auparavant tout entière dans `sentinel-api` (domain + application + ports + adapters). Objectif : extraire le **cœur pur** (sans dépendance infra) dans une crate dédiée `sentinel-core` pour pouvoir le réutiliser ailleurs (workers, futurs binaires, tests isolés).

## Stratégie « pure stricte »

`sentinel-core` ne dépend **d'aucune** lib infra : pas de `axum`, `sqlx`, `reqwest`, `redis`, `tonic`, `serenity`. Seulement les utils standard (`chrono`, `uuid`, `serde`, `thiserror`, `tracing`, `tokio`, `rand`).

Conséquence : les types domaine (`GuildId/UserId/ChannelId/MessageId/RoleId`, et les enums `ModerationGravity/PlayerClass/VoiceChannelKind`) ne dérivent plus `sqlx::Type/FromRow/Encode/Decode`. Le mapping Postgres vit côté adapter, dans `sentinel-api/src/adapters/outbound/postgres/types.rs` (wrappers `Pg*`).

## Phasage

| Stage | Description | État |
|-------|-------------|------|
| **1** | Créer `sentinel-core` + bouger `domain/` + extraire les sqlx-derives | 🟡 quasi fini (~7 vraies erreurs + 5 fichiers corrompus à revert) |
| 2 | Introduire trait `UnitOfWork` dans `sentinel-core` + impl Postgres dans adapter | ⏳ |
| 3 | Migrer les 138 fichiers `ports/` vers `sentinel-core` (sans `sqlx::Transaction<Postgres>`) | ⏳ |
| 4 | Fixer les 8 fuites `application → adapters` (extraire 4 traits + `cached_json`) | ⏳ |
| 5 | Migrer les 118 fichiers `application/` vers `sentinel-core` | ⏳ |

## Ce qui a été fait (stage 1)

### Layout

```
sentinel-core/                 # NOUVEAU — coeur pur
└── src/
    ├── lib.rs                 # pub mod domain;
    └── domain/                # 204 fichiers (entities, enums, errors, services)

sentinel-api/
└── src/
    ├── adapters/outbound/postgres/types.rs   # NOUVEAU — wrappers Pg* pour sqlx
    ├── application/                          # reste ici (à bouger stage 5)
    ├── ports/                                # reste ici (à bouger stage 3)
    └── adapters/                             # reste ici
```

### Workspace `Cargo.toml`

- Ajout du membre `sentinel-core`
- Ajout de la dep workspace : `sentinel-core = { path = "sentinel-core" }`

### `sentinel-api/Cargo.toml`

- Ajout : `sentinel-core = { workspace = true }`

### Renommages globaux

- `crate::domain::*` → `sentinel_core::domain::*` dans 549 fichiers de `sentinel-api`
- Suppression de `pub mod domain;` dans `sentinel-api/src/lib.rs`

### Wrappers `Pg*` (adapter Postgres)

Fichier `sentinel-api/src/adapters/outbound/postgres/types.rs` :

- `PgModerationGravity` ↔ `ModerationGravity` (via `From` bidirectionnel)
- `PgPlayerClass` ↔ `PlayerClass`
- `PgVoiceChannelKind` ↔ `VoiceChannelKind`

Les 5 newtypes discord_ids (`GuildId/UserId/...`) n'ont **pas** de wrapper Pg* — on utilise `String` directement dans les `FromRow` rows et on convertit via `.into()` (le newtype impl `From<String>`).

## Patterns appliqués (~70 fichiers postgres adapter)

### 1. `FromRow` Row structs

```rust
// AVANT
#[derive(sqlx::FromRow)]
struct Row { guild_id: GuildId, user_id: UserId, channel_id: ChannelId, ... }

// APRÈS
#[derive(sqlx::FromRow)]
struct Row { guild_id: String, user_id: String, channel_id: String, ... }
```

### 2. `From<Row> for Entity`

```rust
// AVANT
impl From<Row> for Entity {
    fn from(r: Row) -> Self {
        Self { guild_id: r.guild_id, user_id: r.user_id, ... }
    }
}

// APRÈS
impl From<Row> for Entity {
    fn from(r: Row) -> Self {
        Self { guild_id: r.guild_id.into(), user_id: r.user_id.into(), ... }
    }
}
// Pour Option : .map(Into::into)
```

### 3. Bind sites

```rust
// AVANT — &GuildId n'impl plus Encode
.bind(&entity.guild_id)
.bind(&entity.optional_id)  // Option<GuildId>

// APRÈS — &str impl Encode
.bind(entity.guild_id.as_str())
.bind(entity.optional_id.as_deref())  // Option<&str>
```

### 4. Enums (3 wrappers Pg*)

```rust
// Row struct
struct Row { class: crate::adapters::outbound::postgres::types::PgPlayerClass }

// From<Row>
class: r.class.into()  // PgPlayerClass → PlayerClass

// Bind
.bind(crate::adapters::outbound::postgres::types::PgPlayerClass::from(value))
```

## Tâches restantes — stage 1

### A. PRIORITÉ — revert des 5 fichiers corrompus

Tentative de bulk-fix via PowerShell hashtable a injecté du texte parasite (`bbind`, `bawait`, `bmap_err`...) dans 5 fichiers. À revert via :

```bash
git checkout HEAD -- \
  sentinel-api/src/adapters/outbound/postgres/audit/audit_log_repository.rs \
  sentinel-api/src/adapters/outbound/postgres/audit/discord_action_message_repository.rs \
  sentinel-api/src/adapters/outbound/postgres/community/confession_repository.rs \
  sentinel-api/src/adapters/outbound/postgres/coude/combat_repository.rs \
  sentinel-api/src/adapters/outbound/postgres/system/ticket_repository.rs
```

**Attention** : ce revert ramène ces 5 fichiers à l'état du commit `4c724618` (avant agents), donc il faudra **réappliquer manuellement** les fixes simples ci-dessous (cf. B).

### B. 7 erreurs triviales à fixer manuellement (Edit, pas PowerShell)

Après revert, refixer ces 6 lignes (1 par fichier sauf confession qui en a 2) :

| Fichier | Ligne | Avant | Après |
|---|---|---|---|
| `audit/audit_log_repository.rs` | 68 | `.bind(log.channel_id.as_str())` | `.bind(log.channel_id.as_deref())` |
| `audit/discord_action_message_repository.rs` | 43 | `.bind(msg.message_id.as_deref())` | `.bind(msg.message_id.as_str())` |
| `community/confession_repository.rs` | 204, 507 | `.bind(c.channel_id.as_str())` | `.bind(c.channel_id.as_deref())` |
| `coude/combat_repository.rs` | 335 | `.bind(new.channel_id.as_str())` | `.bind(new.channel_id.as_deref())` |
| `system/ticket_repository.rs` | 183 | `.bind(ticket.channel_id.as_str())` | `.bind(ticket.channel_id.as_deref())` |

Logique : `channel_id` est `Option<String>` dans ces entités → `.as_deref()` donne `Option<&str>` ; `MessageId` est un newtype non-Option → `.as_str()`.

### C. Décisions archi appliquées

- **`ports/outbound/casino/blackjack_table_repository.rs`** : option **b** appliquée — DTOs `BlackjackTable` et `BlackjackTablePlayer` passés en `String` au lieu des newtypes. `sqlx::FromRow` gardé temporairement (à déplacer en stage 3).

- **`ports/outbound/moderation/pending_action_repository.rs`** : `PendingAction` conserve `#[derive(sqlx::FromRow)]` qui devient inutile après ajout d'un Row local côté adapter. À nettoyer en stage 3.

### D. Stages 2-5 (cf. tableau plus haut)

## Historique récent

- `45f72d68` : commit baseline (avant refacto)
- `4c724618` : wip stage 1 (extraction sentinel-core, 82 erreurs)
- `fcdfc305` : wip suite (4 agents fixes + 5 fichiers corrompus à revert) ← **HEAD**

## Comment continuer

```bash
# Voir les erreurs résiduelles
cargo check -p sentinel-api 2>&1 | grep "^error" | head -30

# Voir les fichiers concernés
cargo check -p sentinel-api 2>&1 | grep "^  --> sentinel-api" | sed 's/:.*//' | sort -u

# Pour reprendre : commencer par décider quoi faire pour blackjack_table_repository.rs
# (le port). Le déblocage de ce fichier devrait éliminer la majorité des cascades.
```

## Notes

- `sentinel-core` compile pur (`cargo check -p sentinel-core` ✅).
- Les autres crates (`sentinel-bot`, `sentinel-worker`, `sentinel-gateway`) ne dépendent pas de `sentinel-api`, donc non impactées par le WIP.
- Les conversions `String → GuildId` reposent sur le `From<String>` déjà présent sur les newtypes (`sentinel-core/src/domain/entities/system/discord_ids.rs`).
- La `Deref<Target=str>` sur les newtypes permet aussi d'écrire `&*guild_id` pour obtenir `&str` (utilisé en alternative à `.as_str()` dans certains binds).
