# 📐 Règles d'architecture DiscordSentinel

Ce document fixe les règles **non négociables** pour construire un bot,
un worker ou une fonctionnalité dans DiscordSentinel. Elles ont été
définies au fil des refactos et doivent être respectées pour toute
nouvelle contribution.

> 🎯 **La règle d'or** : le bot et le worker sont **100 % thin**. Aucune
> logique métier, aucun SQL, aucune décision de gameplay/modération/
> persistance côté bot ou worker. Tout vit dans l'API.

---

## 🧱 Architecture hexagonale (API)

L'API Sentinel suit une architecture hexagonale stricte avec 4 couches
qui ne peuvent se référencer que dans un sens :

```
adapters  →  ports  →  application  →  domain
(inbound)    (in/out)   (services)     (entities, pure)
```

### Couche `domain/`

**Contenu** : entités pures, catalogues statiques, énums, fonctions
déterministes. **Zéro IO**, zéro `async`, zéro dépendance sur sqlx,
reqwest, serenity, etc.

```rust
// services/api/src/domain/entities/coude_steal_protection.rs
pub struct CoudeStealProtection { … }
pub const STEAL_PROTECTION_ITEMS: &[StealProtectionItemDef] = &[ … ];
pub fn find_protection_item(key: &str) -> Option<&'static …> { … }
```

**Règles** :
- Les constantes gameplay (prix, seuils, catalogues, multiplicateurs)
  vivent ici.
- Les fonctions helper sont pures : mêmes entrées → mêmes sorties.
- Les tests unitaires sont co-localisés (`#[cfg(test)] mod tests`).
- Aucune struct avec un `async` method ici.

### Couche `ports/`

**Contenu** : uniquement des *traits* (interfaces) qui définissent les
contrats que l'application et les adapters doivent respecter.

- **`ports/inbound/`** — interfaces des *use cases*. Nommés
  `Manage<Feature>UseCase`. Ce que l'extérieur peut demander à l'API.
- **`ports/outbound/`** — interfaces des *repositories* et services
  externes. Nommés `<Feature>Repository`. Ce dont l'application a
  besoin pour persister/appeler.

```rust
#[async_trait]
pub trait ManageCoudeCashboxUseCase: Send + Sync {
    async fn deposit(&self, guild_id: &str, amount: i64, source: CashboxSource)
        -> Result<(), DomainError>;
    // …
}

#[async_trait]
pub trait CoudeCashboxRepository: Send + Sync {
    async fn deposit(&self, guild_id: &str, amount: i64, source: CashboxSource)
        -> Result<(), DomainError>;
    // …
}
```

**Règles** :
- Les ports ne connaissent que le domain (pas sqlx, pas reqwest).
- Toute méthode async renvoie `Result<T, DomainError>`.
- Une implémentation de port (`Arc<dyn …Repository>`) est injectée au
  service application via constructor.

### Couche `application/`

**Contenu** : services qui *orchestrent* les ports pour réaliser un use
case. C'est ici que vit la logique métier : appliquer les règles du
domain, composer plusieurs repos, gérer les transactions logiques.

```rust
pub struct ManageCoudeCashboxService {
    repo: Arc<dyn CoudeCashboxRepository>,
    wallet_repo: Arc<dyn WalletRepository>,
}

#[async_trait]
impl ManageCoudeCashboxUseCase for ManageCoudeCashboxService {
    async fn redistribute_weekly(&self, guild_id: &str) -> Result<…> {
        // 1. Lister joueurs actifs
        // 2. Claim atomique contenu caisse
        // 3. Domain::distribute_random
        // 4. Créditer chaque gagnant
        // 5. Record audit
    }
}
```

**Règles** :
- Les services ne font **jamais** de SQL direct. Ils appellent les
  repos.
- Les décisions (seuils, formules, catalogues) viennent du `domain/`.
- Les `ThreadRng` et autres types `!Send` doivent être scopés avec
  `{ let mut rng = rand::thread_rng(); … }` pour être drop avant tout
  `await` qui suit.
- Pas de `warn!()` / `error!()` abusif : logger seulement aux points
  intéressants (erreurs non-fatales, transitions d'état).

### Couche `adapters/`

- **`adapters/outbound/postgres/`** — implémentations Postgres des
  repositories. **C'est le seul endroit où vit du SQL**.
- **`adapters/inbound/http/`** — handlers Axum qui traduisent une
  requête HTTP en appel use case.
- **`adapters/inbound/grpc/`** — handlers Tonic qui traduisent une
  requête gRPC en appel use case.

```rust
// handlers/coude/taunts.rs — ne fait QUE traduire DTO ↔ UC
pub async fn update_taunts_config(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
    Json(dto): Json<UpdateTauntsConfigDto>,
) -> Result<StatusCode, ApiError> {
    state.coude_taunts_uc.set_channel(&guild_id, dto.channel_id.as_deref()).await?;
    state.coude_taunts_uc.set_enabled(&guild_id, dto.enabled).await?;
    Ok(StatusCode::NO_CONTENT)
}
```

**Règles handler** :
- Aucune logique conditionnelle au-delà du mapping DTO.
- Les validations *métier* vont dans le use case, pas ici.
- L'authentification/autorisation (`require_role`) peut se faire ici
  ou dans un middleware, jamais dans le domain.

---

## 🤖 Règles pour les **bots**

Un bot DiscordSentinel est un *client* de l'API. Sa mission est de :

1. Recevoir les interactions Discord (slash commands, boutons,
   composants).
2. Faire **un ou plusieurs** appels gRPC à l'API pour récupérer les
   données ou déclencher les actions.
3. Traduire la réponse de l'API en embed/message Discord.

### Interdictions absolues

- ❌ **Pas de SQL**. Aucun accès direct à la DB.
- ❌ **Pas de `sqlx`** dans les dépendances du bot.
- ❌ **Pas de calcul gameplay**. Pas de « si ses HP sont bas alors… »
  côté bot.
- ❌ **Pas de catalogue statique** dupliqué. Le catalogue vit dans le
  domain API. Le bot le lit via un RPC (`GetCatalog`) au démarrage et
  le cache en mémoire (`CatalogCache` dans `bots/coude-bot/src/catalog.rs`).
- ❌ **Pas de seuils/constantes métier** hardcodés côté bot. Si un bot
  a besoin d'une valeur (cooldown, prix, taux), elle vient de
  l'API/catalog.

### Ce que le bot peut (et doit) faire

- ✅ Appels RPC au client API (`ApiClient`).
- ✅ Traduction `ResponseDto → CreateEmbed`.
- ✅ IO Discord pur : `channel.send_message`, `guild.edit_member`,
  `component.create_response`.
- ✅ Gestion du cache catalog en mémoire.
- ✅ Lecture de la config Discord-centrique (intents, token, salons
  configurés) via `bot_guild_config`.
- ✅ Routage des interactions slash → handler correspondant.
- ✅ **Orchestration minimale** : enchaîner `get_or_create_player` →
  `resolve_combat_now` → `post_embed` est OK. Ce n'est pas de la
  logique, c'est de la séquence d'IO.

### Pattern d'une commande slash

```rust
// bots/coude-bot/src/commands/cagnotte.rs
pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    // 1. Lire guild_id, config, user_id (IO Discord)
    let guild_id = command.guild_id.unwrap().to_string();
    let api = ctx.data.read().await.get::<GameApiKey>().unwrap();

    // 2. Un appel API qui retourne des données déjà cuisinées
    let cashbox = match api.get_cashbox(&guild_id).await { … };

    // 3. Mapping données → embed Discord
    let embed = CreateEmbed::new()
        .title("🎰 Cagnotte communautaire")
        .description(format!("… {}", cashbox.balance));

    // 4. Réponse Discord
    command.create_response(…).await.ok();
}
```

> Si un handler de commande dépasse ~150 lignes ou contient plus d'un
> `match` sur des états métier, c'est probablement qu'une partie
> devrait remonter dans l'API.

---

## ⚙️ Règles pour les **workers**

Les workers sont des processus qui tournent en background et appellent
l'API via gRPC à intervalles réguliers.

### Interdictions

- ❌ Pas de SQL (même règle que les bots).
- ❌ Pas de logique de décision (« est-ce qu'il faut résoudre ce
  combat ? »). L'API décide, le worker exécute.

### Ce qu'un worker doit faire

1. **Spawn périodique** via `spawn_periodic` de `sentinel-worker-common`.
2. **Appel gRPC** à l'API qui retourne une liste d'items à dispatcher.
3. **IO Discord** (reqwest brut ou serenity) pour poster les résultats.

### Pattern `coude-worker/jobs/redistribute_cashbox.rs`

```rust
pub async fn run(_pool: &PgPool, min_days: i64) -> Result<(), String> {
    let mut client = CoudeSocialServiceClient::with_interceptor(…);
    let resp = client
        .redistribute_due_cashboxes(Request::new(RedistributeDueRequest { min_days }))
        .await
        .map_err(|e| format!("RPC: {e}"))?;

    // L'API a déjà filtré les guilds dues, claim atomique, tirage aléatoire,
    // credit wallets, record audit. Le worker log et sort.
    for r in resp.into_inner().redistributed {
        info!(guild_id = %r.guild_id, total = r.total_amount, "redistribute");
    }
    Ok(())
}
```

### Le worker a un `pool: PgPool` — pourquoi ?

Certaines méthodes d'aide legacy utilisent encore le pool (health
check, heartbeat). Pour une nouvelle fonctionnalité, **ignore-le**
(`_pool`) et passe par gRPC.

---

## 🌐 Protocol buffers (`services/proto/proto/*.proto`)

### Règles d'évolution

- **Nouveau champ** : ajouter avec un nouveau tag > tous les existants,
  typé `optional` si ça peut manquer.
- **Nouvelle méthode RPC** : OK, elle est backward-compatible.
- **Supprimer un champ** : **interdit**. Marquer `reserved` ou `deprecated`.
- **Changer le type d'un champ** : **interdit**. Créer un nouveau
  champ.
- **Enum** : les variantes sont préfixées au nom de l'enum
  (`CASHBOX_SOURCE_SHOP_PURCHASE`) parce que prost strippe un préfixe
  commun et retourne des variants en PascalCase (`CashboxSourceShopPurchase`).
  Toujours ajouter une variante `_UNSPECIFIED = 0` en première position.

### Flow pour ajouter un nouveau RPC

1. **Proto** : ajoute `rpc NewMethod(Req) returns (Resp);` dans le
   bon service + les messages `Req`/`Resp`.
2. **API handler gRPC** : implémente la méthode sur le struct
   `…Grpc` qui implémente `XxxService`.
3. **Client bot/worker** : ajoute une méthode sur `ApiClient` qui
   wraps l'appel avec `self.grpc.guarded(…)` pour le retry + le
   mapping d'erreur.
4. **Use case** : le handler gRPC appelle toujours un use case, jamais
   un repo directement.

---

## 🗄️ Migrations SQL

### Règles

- Numérotation strictement croissante : `NNN_description.sql`
  (regarder `services/api/migrations/` pour le dernier N).
- Jamais de `DROP TABLE` sans plan de rollback.
- Jamais de `DROP COLUMN` sur une table activement utilisée sans étape
  intermédiaire.
- Les migrations destructrices doivent être précédées d'une note dans
  le commit message.
- `CREATE INDEX IF NOT EXISTS` pour les index (permet les re-runs).
- `ON CONFLICT DO NOTHING` ou `ON CONFLICT DO UPDATE` pour les seeds.

### Données de migration

Si ta migration doit transformer des données existantes (ex. Phase 9
Part B migrait les items anti-vol en 3 jours d'abonnement gratuit),
**écris-la dans la même migration SQL**. Pas de script Rust séparé
qui tourne une fois — c'est fragile.

---

## 🧪 Tests

### Domain

**Obligatoire** : chaque fichier de `domain/entities/` avec de la
logique pure doit avoir un `#[cfg(test)] mod tests` co-localisé qui
couvre les cas limites, les variants d'enum, les formules.

```rust
#[test]
fn duration_cost_multipliers_decrease_per_day() {
    for d in [Duration::OneDay, Duration::ThreeDays, …] {
        assert!(d.total_cost(base) > 0);
    }
}
```

### Application

Les services se testent avec des **mocks de repos** (impl manuelles du
trait pour le test). Pas besoin de mockall — un `struct MockRepo;
impl CoudeCashboxRepository for MockRepo { … }` suffit.

### Intégration

Pas de test d'intégration avec une vraie DB pour une nouvelle feature
sauf si c'est critique (paiements, transferts atomiques). Le domain +
application coverage suffit pour la plupart des features.

### Commande pour run

```bash
cargo test -p sentinel-api --lib <module>
```

---

## 🔀 Flow pour **ajouter une nouvelle feature**

Supposons que tu ajoutes « Daily streak bonus ». Voici l'ordre :

1. **Migration SQL** si nouvelle table ou colonne.
2. **Entité domain** dans `domain/entities/` avec les constantes +
   fonctions pures + tests.
3. **Port outbound** (`CoudeDailyStreakRepository` dans `ports/outbound/`).
4. **Port inbound** (`ManageCoudeDailyStreakUseCase` dans `ports/inbound/`).
5. **Service application** qui implémente le use case en utilisant le repo.
6. **Impl Postgres** du repo dans `adapters/outbound/postgres/`.
7. **Proto messages + RPC** dans `services/proto/proto/coude.proto`.
8. **Handler gRPC** dans `adapters/inbound/grpc/coude.rs`.
9. **Câblage** dans `main.rs` (création repo + use case) et `AppState`.
10. **Client bot** : méthode dans `api_client.rs`.
11. **Commande slash** : handler + register + mod.rs + handler.rs dispatch.
12. **Tests** du domain + application.
13. **`cargo check --workspace`** avant commit.
14. **Commit** avec un message descriptif (voir plus bas).

À **chaque étape**, `cargo check -p sentinel-api` doit passer avant de
continuer. Ne pas accumuler 10 fichiers avant de vérifier la
compilation.

---

## 🌳 Git & commits

### Règles non négociables

- ❌ **Jamais `--no-verify`** sur un commit.
- ❌ **Jamais `--amend`** après un hook raté — faire un nouveau commit.
- ❌ **Jamais de `git add .`** ou `git add -A` — stager les fichiers
  explicitement.
- ❌ **Jamais de `git push --force`** sur `main`.
- ✅ Une branche par feature si possible, mais commits directs sur
  `main` OK pour DiscordSentinel (mono-dev).

### Format des messages de commit

```
type(scope): résumé court en présent

Description longue expliquant le **pourquoi** plus que le **quoi**
(le diff montre déjà le quoi). Mentionner les impacts, les
limitations, les follow-ups.

Co-Authored-By: …
```

Types : `feat`, `fix`, `refactor`, `docs`, `chore`, `test`, `style`.
Scope : `coude`, `voice`, `security`, `web`, `worker`, etc.

Exemple :
```
feat(coude): phase 9 part A — caisse communautaire (cagnotte)

Collecte les coins "perdus" par l'économie (shop, assurance, classe,
reset, taxe /donner, pénalité lâcheté) dans une caisse par guild,
redistribuée aléatoirement chaque semaine aux joueurs actifs…
```

---

## ⚠️ Anti-patterns à fuir

1. **Bot qui appelle `pool.execute(…)`** → toute la logique remonte à
   l'API.
2. **Domain qui importe `sqlx`** → les types sqlx ne peuvent pas
   traverser le domain. Convertir en types purs dans l'adapter.
3. **Handler HTTP qui fait des `match status` compliqués** → c'est
   un indice que la logique devrait être dans le use case.
4. **Proto field renommé** → casse tous les clients qui ne sont pas
   re-compilés ensemble. Ne jamais renommer, ajouter un nouveau champ.
5. **`unwrap()` dans un service async** → toujours `ok_or_else` ou
   `map_err` avec un `DomainError` explicite.
6. **Logger secret/PII dans les warn!** → pas de tokens, pas de
   message Discord en clair dans les logs.
7. **Hardcoder une valeur « temporaire »** sans commentaire
   d'architecture. Si c'est intentionnel, l'écrire ; si ça doit
   bouger, créer une issue/TODO en commentaire.

---

## 🔁 Règle du pragmatisme

Toutes ces règles peuvent être violées **si** :
- Tu documentes pourquoi dans le commit message **et** dans un
  commentaire `// ARCHITECTURE: violation parce que X`.
- Tu crées un follow-up issue/TODO pour corriger.
- Tu en discutes (dans le commit ou PR).

Exemple de violation acceptable : le catalogue d'items est hardcodé
dans le domain au lieu d'être en DB. Raison : les valeurs ont été
validées et un override per-guild apporterait de la complexité sans
valeur. Annoté dans `coude_steal_protection.rs` avec une note
« Choix d'architecture ».

---

## 📚 Références dans le code

- `services/api/src/domain/entities/coude_cashbox.rs` — entité pure +
  catalogue.
- `services/api/src/application/manage_coude_cashbox_service.rs` —
  service qui orchestre.
- `services/api/src/adapters/outbound/postgres/coude_cashbox_repository.rs`
  — impl Postgres (transaction + SELECT FOR UPDATE).
- `services/api/src/adapters/inbound/grpc/coude.rs` — handler gRPC.
- `bots/coude-bot/src/commands/cagnotte.rs` — commande slash thin.
- `services/workers/coude-worker/src/jobs/redistribute_cashbox.rs` —
  worker periodic thin.

Ces 6 fichiers ensemble forment l'exemple canonique d'une feature
Phase 9 ajoutée en respectant toutes les règles ci-dessus.

---

*Ce document doit être gardé à jour. Si une règle change, mettre à
jour ici **avant** de committer le code qui la change.*
