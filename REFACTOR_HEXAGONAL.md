# Refacto hexagonal — extraction de `sentinel-core`

**Statut : ✅ stages 1→5 terminés.** Build vert, 0 warning lib.

## Layout final

```
sentinel-core/                    # COEUR HEXAGONAL PUR
├── src/
│   ├── domain/                   # entities, services, errors (204 fichiers)
│   ├── ports/                    # 138 fichiers
│   │   ├── inbound/              # use case traits (ManageX, etc.)
│   │   ├── outbound/             # repository traits + DiscordApi, AI, EventBroadcaster, cache_helpers
│   │   └── uow.rs                # DbTx (Any+Send) + UnitOfWork
│   └── application/              # 116 services use-case
└── Cargo.toml                    # deps : tokio, serde, async-trait, ndarray, image (no infra)

sentinel-api/                     # ADAPTERS + 2 services bloqués
└── src/
    ├── adapters/
    │   ├── inbound/              # http, grpc, ws
    │   └── outbound/             # postgres (avec uow.rs : PgTx, PgUnitOfWork, as_pg)
    │                             # redis_cache, discord_api, inference_service, text_tokenizer
    └── application/
        ├── mod.rs                # re-exporte sentinel_core::application::* + 2 services locaux
        ├── audit/manage_stats_service.rs   # tient redis::Client (SMEMBERS bots:known)
        └── system/export_service.rs        # utilise sqlx::PgPool directement
```

## Décisions clés

### Stage 2 — UnitOfWork

Pour permettre aux ports d'accepter des transactions sans dépendre de sqlx :

```rust
// sentinel-core/src/ports/uow.rs
pub trait DbTx: Any + Send {
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

pub trait UnitOfWork: Send + Sync {
    async fn begin(&self) -> Result<Box<dyn DbTx>, DomainError>;
    async fn commit(&self, tx: Box<dyn DbTx>) -> Result<(), DomainError>;
    async fn rollback(&self, tx: Box<dyn DbTx>) -> Result<(), DomainError>;
}
```

L'adapter Postgres définit `PgTx(Transaction<'static, Postgres>)` qui impl `DbTx`, et le helper `as_pg(tx: &mut dyn DbTx) -> &mut Transaction` fait le downcast côté adapter.

Le lifetime `'static` sur la tx est OK : `pool.begin()` retourne déjà `Transaction<'static, Postgres>`.

### Stage 4 — résorption des fuites application → adapters

| Fuite | Solution | Commit |
|---|---|---|
| `cached_json` × 5 | Move vers `core/ports/outbound/system/cache_helpers.rs` (logique pure) | 4a |
| `DiscordApi` | Trait + DTOs déjà purs → move vers `core/ports/outbound/discord_api.rs` | 4a |
| `EventBroadcaster` | Extrait trait + DTO `WsEvent` dans core ; concret en adapter impl trait | 4b |
| `PgTx` slot/wheel | Inject `Arc<dyn UnitOfWork>` ; `pool.begin()` → `uow.begin()` | 4c |
| `InferenceService` × 2 | Extrait trait dans core (ndarray ajouté aux deps de core) | 4d |
| `TextTokenizer` | Idem | 4d |
| `as_pg` wallet | Extrait `WalletRepository::credit_in_tx` / `debit_in_tx` ; SQL inline → adapter | 4e |

**Décision deps `sentinel-core`** : `ndarray` et `image` ajoutés. Justification : libs purement CPU/numériques, no IO, no infra. Cohérent avec la philosophie "core pur" (même statut que `serde_json`).

### Stage 5 — move application/ vers core

116/118 services migrés. Les 2 restants tiennent une dep infra incompressible et bénéficieraient d'un port supplémentaire :

- `manage_stats_service` → besoin d'un port `ServiceRegistry` (SMEMBERS Redis pour découvrir bots actifs)
- `export_service` → besoin d'un port `ExportRepository` (queries SQL pour CSV/JSON exports)

Ces extractions sont possibles mais pas bloquantes pour l'archi : elles peuvent se faire au fil de l'eau.

## Patterns de code

### Repository Postgres : Row struct + From

```rust
#[derive(sqlx::FromRow)]
struct Row { guild_id: String, /* ... */ }

impl From<Row> for Entity {
    fn from(r: Row) -> Self { Self { guild_id: r.guild_id.into(), /* ... */ } }
}
```

Les newtypes `GuildId`/`UserId`/etc. impl `From<String>` côté domain pour la conversion.

### Service avec tx atomique

```rust
pub struct MyService {
    repo: Arc<dyn MyRepository>,
    uow: Arc<dyn UnitOfWork>,  // au lieu de pg_pool
}

async fn do_atomic(&self, ...) -> Result<(), DomainError> {
    let mut tx = self.uow.begin().await?;
    self.repo.something_in_tx(&mut *tx, ...).await?;  // &mut *tx déréférence Box
    self.uow.commit(tx).await
}
```

### Adapter d'un repo `_in_tx`

```rust
async fn something_in_tx(&self, tx: &mut dyn DbTx, ...) -> Result<(), DomainError> {
    let tx = as_pg(tx);  // récupère &mut Transaction<'static, Postgres>
    sqlx::query(...).execute(&mut **tx).await.map_err(pg_err)?;
    Ok(())
}
```

## Dette technique restante

1. **Tests cassés** (~140 erreurs E0308) : fixtures de tests stage 1 utilisent `g.clone()` au lieu de `g.clone().into()` pour les newtypes. Session dédiée pour corriger.
2. **2 services bloqués** dans sentinel-api : extraire les ports `ServiceRegistry` + `ExportRepository`.
3. **Tests sentinel-core** : `tracing_subscriber` manquant dans les deps de test du domain (test `community::conduct`).

## Historique des commits

```
713872c8  stage 1 — sentinel-core extraction (build vert)
e4fd26ea  stage 2 — UnitOfWork + DbTx
(stage 3) — move ports vers sentinel-core
stage 4a — cache_helpers + DiscordApi
32723515  stage 4b — EventBroadcaster
24f53909  stage 4c — UnitOfWork injection (slot/wheel)
stage 4d — InferenceService + TextTokenizer
4f670cdc  stage 4e — credit_in_tx/debit_in_tx
stage 5 — move application/ vers core
```
