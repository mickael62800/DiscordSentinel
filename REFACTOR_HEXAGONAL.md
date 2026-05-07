# Refacto hexagonal — extraction de `sentinel-core`

**Statut : WIP, build cassé** (~82 erreurs restantes côté `sentinel-api`).

## Contexte

L'archi hexagonale était auparavant tout entière dans `sentinel-api` (domain + application + ports + adapters). Objectif : extraire le **cœur pur** (sans dépendance infra) dans une crate dédiée `sentinel-core` pour pouvoir le réutiliser ailleurs (workers, futurs binaires, tests isolés).

## Stratégie « pure stricte »

`sentinel-core` ne dépend **d'aucune** lib infra : pas de `axum`, `sqlx`, `reqwest`, `redis`, `tonic`, `serenity`. Seulement les utils standard (`chrono`, `uuid`, `serde`, `thiserror`, `tracing`, `tokio`, `rand`).

Conséquence : les types domaine (`GuildId/UserId/ChannelId/MessageId/RoleId`, et les enums `ModerationGravity/PlayerClass/VoiceChannelKind`) ne dérivent plus `sqlx::Type/FromRow/Encode/Decode`. Le mapping Postgres vit côté adapter, dans `sentinel-api/src/adapters/outbound/postgres/types.rs` (wrappers `Pg*`).

## Phasage

| Stage | Description | État |
|-------|-------------|------|
| **1** | Créer `sentinel-core` + bouger `domain/` + extraire les sqlx-derives | 🟡 quasi fini (~82 erreurs) |
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

### A. Erreurs de compilation (~82)

Cas signalés par les agents :

1. **`ports/outbound/casino/blackjack_table_repository.rs`** : ce port définit `BlackjackTable` et `BlackjackTablePlayer` avec `#[derive(sqlx::FromRow)]` ET des champs `GuildId/ChannelId/UserId`. Trois options :
   - Déplacer ces structs dans l'adapter, faire que le port retourne un DTO pur
   - Remplacer les newtypes par `String` dans ces structs
   - Ajouter des wrappers Pg* pour les ids dans `types.rs`
   - **→ Décision archi nécessaire**.

2. **`adapters/outbound/postgres/casino/blackjack_table_repository.rs`** : 2 erreurs miroirs du précédent.

3. **`adapters/outbound/postgres/audit/discord_action_message_repository.rs`** : 1 erreur résiduelle.

4. **Reste** : ~75 erreurs cascade probablement liées au point 1 (FromRow ne compile pas → propagation).

### B. Stages 2-5 (cf. tableau plus haut)

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
