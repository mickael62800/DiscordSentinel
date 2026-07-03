# Architecture & Plan d'implémentation — Jeu « Influence »

> Document actionnable. Suivre les phases dans l'ordre. Toutes les conventions sont calquées sur le module `coude` existant, cité en référence à chaque couche.

## 1. Vue d'ensemble

**Influence** est un jeu de stratégie sociale/politique persistant pour Discord (cf. `docs/Nouveau jeux/01.md` à `07.md`). Les joueurs sont des **citoyens** qui accumulent 5 capitaux — **Influence, Argent, Réputation, Information, Réseau** (04.md) —, fondent des **organisations** (05.md), font de la **politique** (parlement/gouvernement/justice/élections/lois, 06.md) et manipulent l'**information** (enquêtes/fuites/rumeurs/scandales, 07.md). Le bot est un **arbitre** (01.md §8) ; le monde évolue même hors ligne (worker).

### Principe technique clé : « stocké chiffré / exposé narratif »

Les capitaux sont stockés en **entiers** (privés, connus du seul joueur) mais **affichés en paliers narratifs** aux tiers (ex. `Influence: 1247` → `« Très élevée »`). Cela crée incertitude et mystère (cf. profil 03.md §4 : `★★★★☆`, réputation `Positive`).

**Où cela vit dans les couches :** la conversion valeur→palier est une **fonction pure du domaine**, sans I/O, testable unitairement — exactement comme `resolve_outcome` / `coin_delta` dans `sentinel-core/src/domain/entities/coude/tout_ou_rien.rs:38,53`. On la place dans `sentinel-core/src/domain/entities/influence/tier.rs`. Le stockage entier vit dans les repos Postgres ; l'exposition narrative est calculée soit dans un service application, soit à l'affichage (handler/DTO), **jamais** stockée en base.

```rust
// sentinel-core/src/domain/entities/influence/tier.rs  (fonction pure, cf. tout_ou_rien.rs)
pub enum NarrativeTier { Negligeable, Faible, Moyenne, Elevee, TresElevee, Legendaire }

/// Seuils = donnée de config passée en paramètre (domaine pur, aucune I/O).
pub fn to_tier(value: i64, thresholds: &TierThresholds) -> NarrativeTier { /* ... */ }
```

Règle : le **propriétaire** voit le chiffre exact ; les **autres** voient `to_tier(...)`. Décision prise dans le service application selon `viewer_id == owner_id`.

## 2. Cartographie hexagonale

Arborescence à créer, strictement calquée sur `coude` (un dossier `influence/` par couche). Aucune dépendance infra dans `sentinel-core`.

### 2.1 sentinel-core — Domaine (entités & logique pure)

```
sentinel-core/src/domain/entities/influence/
  mod.rs                 # déclare les sous-modules (cf. entities/coude/mod.rs)
  citizen.rs             # Citizen { capitals, roles, joined_at }
  capital.rs             # Capitals { influence, money, reputation, information, network: i64 }
  tier.rs                # to_tier() + TierThresholds  (FONCTION PURE, cf. tout_ou_rien.rs:38)
  reputation.rs          # dimensions (fiabilité, popularité...) 03.md §10
  organization.rs        # Organization { kind, treasury, reputation, influence }
  org_membership.rs      # rôle hiérarchique (Fondateur..Recrue) 05.md §5
  law.rs                 # Law { title, body, status, effects, duration }
  vote.rs                # Vote, VoteKind (public/secret), Majority (simple/absolue/qualifiée) 06.md §9
  mandate.rs             # Mandate { office, holder, started_at, expires_at } 06.md §11
  information.rs         # Info { visibility: Public|Prive|Secret, credibility, source } 07.md
  investigation.rs       # Investigation { target, progress, outcome } 07.md §6
  scandal.rs             # Scandal (info révélée -> conséquences) 07.md §12
  archive_entry.rs       # trace historique immuable 03.md §12 / 07.md §13
sentinel-core/src/domain/enums/influence/
  mod.rs
  organization_kind.rs   # Entreprise|Parti|Media|Syndicat|Secrete (cf. enums/coude/coude_class.rs)
  political_office.rs     # Depute|Ministre|President|Juge...
sentinel-core/src/domain/services/influence/
  mod.rs
  influence_engine/      # calculs d'influence collective org (cf. services/coude/coude_combat_engine/)
  election_engine/       # dépouillement, majorités
```

### 2.2 sentinel-core — Ports inbound (use cases)

```
sentinel-core/src/ports/inbound/influence/mod.rs   # cf. ports/inbound/coude/mod.rs
  register_citizen.rs        RegisterCitizenUseCase
  view_profile.rs            ViewProfileUseCase        # renvoie chiffres OU paliers selon viewer
  create_organization.rs     CreateOrganizationUseCase
  manage_membership.rs       ManageMembershipUseCase   # recruter/promouvoir/exclure
  transfer_capital.rs        TransferCapitalUseCase    # conversions 04.md §10
  propose_law.rs             ProposeLawUseCase
  cast_vote.rs               CastVoteUseCase
  tally_election.rs          TallyElectionUseCase
  open_investigation.rs      OpenInvestigationUseCase
  leak_information.rs        LeakInformationUseCase
  publish_article.rs         PublishArticleUseCase
```

Chaque fichier = un `#[async_trait] pub trait XxxUseCase: Send + Sync` retournant `Result<_, DomainError>`, exactement comme `resolve_combat_now.rs:55`.

### 2.3 sentinel-core — Ports outbound (repositories)

```
sentinel-core/src/ports/outbound/influence/mod.rs   # cf. ports/outbound/coude/mod.rs
  citizen_repository.rs          CitizenRepository (get_or_create, cf. player_repository)
  organization_repository.rs     OrganizationRepository
  membership_repository.rs       MembershipRepository
  law_repository.rs              LawRepository
  vote_repository.rs             VoteRepository
  mandate_repository.rs          MandateRepository
  information_repository.rs      InformationRepository
  investigation_repository.rs    InvestigationRepository
  archive_repository.rs          ArchiveRepository (append-only)
```

Traits identiques en forme à `tout_ou_rien_repository.rs:11` (`&str guild_id`, `DomainError`).

### 2.4 sentinel-core — Services application

```
sentinel-core/src/application/influence/mod.rs   # cf. application/coude/mod.rs
  register_citizen_service.rs
  view_profile_service.rs        # applique to_tier() si viewer != owner  <-- cœur du "chiffré/narratif"
  create_organization_service.rs
  manage_membership_service.rs
  law_service.rs                 # dépôt -> débat -> vote -> application (06.md §7)
  election_service.rs
  investigation_service.rs
  information_service.rs
  guild_settings.rs              # chargement config par serveur (cf. application/coude/guild_settings.rs)
```

Structure d'un service = `struct XService { repos: Arc<dyn ...>, cfg_repo: Option<Arc<dyn BotConfigRepository>> }` + `new(...)` + `with_bot_config_repo(...)`, exactement `play_tout_ou_rien_service.rs:28-59`. Tests via `#[cfg(test)] #[path="tests/x.rs"] mod tests;` (`play_tout_ou_rien_service.rs:208`).

### 2.5 sentinel-api — Adapters sortants (Postgres)

```
sentinel-api/src/adapters/outbound/postgres/influence/mod.rs
  citizen_repository.rs  organization_repository.rs  membership_repository.rs
  law_repository.rs  vote_repository.rs  mandate_repository.rs
  information_repository.rs  investigation_repository.rs  archive_repository.rs
```

Chaque `PgXRepository { pool: PgPool }`, `#[derive(sqlx::FromRow)] struct Row`, `TryFrom<Row>`, `pg_err_ctx(TBL, e)` — copie conforme de `tout_ou_rien_repository.rs`.

### 2.6 sentinel-api — Adapters entrants (HTTP + gRPC) + bootstrap

```
sentinel-api/src/adapters/inbound/http/handlers/influence/{mod.rs,dto.rs,citizens.rs,orgs.rs,laws.rs,votes.rs,information.rs}
sentinel-api/src/adapters/inbound/http/routes/influence.rs   # cf. routes/coude.rs, monté dans router.rs
sentinel-api/src/adapters/inbound/grpc/influence/            # si le bot passe par gRPC (cf. grpc/coude/)
```

Wiring **obligatoire** (cf. `app_state.rs:610-617` et `http/state.rs:97-98`) :
```rust
// app_state.rs
let citizen_repo: Arc<dyn CitizenRepository> = Arc::new(PgCitizenRepository::new(pg_pool.clone()));
let register_citizen_uc: Arc<dyn RegisterCitizenUseCase> =
    Arc::new(RegisterCitizenService::new(citizen_repo.clone(), ...));
// exposer chaque repo/uc comme champ de AppState (http/state.rs)
```

### 2.7 sentinel-bot — Adapter Discord

```
sentinel-bot/src/modules/influence/
  mod.rs            # register_commands(), handle_command(), handles_command(), handles_component()
  api_client/       # appels HTTP/gRPC vers sentinel-api (cf. modules/coude/api_client/)
  commands/         # un fichier par commande slash
```

### 2.8 sentinel-worker — Jobs monde vivant

```
sentinel-worker/src/domains/influence/mod.rs   # cf. domains/coude/mod.rs
  expire_mandates.rs      # fin de mandat -> élection (06.md §11)
  run_elections.rs        # ouverture/dépouillement scrutins
  resolve_laws.rs         # passage débat->vote->application
  progress_investigations.rs
  world_events.rs         # événements mondiaux occasionnels (01.md §8)
```

## 3. Modèle de données (migration de départ **328**)

Toutes idempotentes (`CREATE TABLE IF NOT EXISTS`, `ADD COLUMN IF NOT EXISTS`, config via `UPDATE bot_definitions ... WHERE NOT (config_schema @> ...)`), motif de `327_game_portal_sessions.sql`. Toutes les tables portent `guild_id TEXT NOT NULL` (multi-serveur). Préfixe `influence_`.

| Table | Colonnes clés | Relations |
|---|---|---|
| `influence_citizens` | `id UUID PK`, `guild_id`, `user_id`, `username`, `influence/money/reputation/information/network BIGINT NOT NULL DEFAULT 0`, `joined_at`, `UNIQUE(guild_id,user_id)` | racine |
| `influence_reputation_dims` | `citizen_id FK`, `fiabilite/popularite/notoriete/transparence INT` | → citizens |
| `influence_organizations` | `id UUID PK`, `guild_id`, `kind TEXT`, `name`, `motto`, `treasury BIGINT`, `reputation/influence BIGINT`, `founder_id`, `created_at`, `dissolved_at NULL` | founder → citizens |
| `influence_org_members` | `org_id FK`, `citizen_id FK`, `role TEXT`(Fondateur..Recrue), `joined_at`, `UNIQUE(org_id,citizen_id)` | M:N citizens/orgs |
| `influence_org_relations` | `org_a FK`, `org_b FK`, `relation`(alliance/rivalité/boycott) | orgs↔orgs |
| `influence_laws` | `id PK`, `guild_id`, `title`, `body`, `status`(depot/debat/vote/adoptee/rejetee), `effects JSONB`, `expires_at NULL`, `author_id` | author→citizens |
| `influence_votes` | `id PK`, `subject_type`(law/election/motion), `subject_id`, `voter_id`, `choice`, `secret BOOL`, `created_at`, `UNIQUE(subject_id,voter_id)` | → laws/elections |
| `influence_elections` | `id PK`, `office TEXT`, `opens_at`, `closes_at`, `status`, `winner_id NULL` | |
| `influence_candidacies` | `election_id FK`, `citizen_id FK`, `program TEXT` | → elections |
| `influence_mandates` | `id PK`, `office`, `holder_id FK`, `started_at`, `expires_at`, `active BOOL` | → citizens |
| `influence_information` | `id PK`, `visibility`(public/prive/secret), `content`, `credibility INT`, `source_type`, `owner_id`, `veracity`(vrai/faux/rumeur) | |
| `influence_investigations` | `id PK`, `initiator_id`, `target_type`, `target_id`, `progress INT`, `outcome NULL`, `created_at` | |
| `influence_archives` | `id PK`, `guild_id`, `event_type`, `payload JSONB`, `occurred_at` — **append-only, jamais supprimé** (03.md §12, 07.md §13) | mémoire du serveur |

Index de scan worker (cf. `327` `idx_game_servers_ip_reveal`) : `idx_influence_mandates_expiry ON influence_mandates(expires_at) WHERE active`, `idx_influence_elections_close ON influence_elections(closes_at) WHERE status='ouverte'`.

Config web (fin de migration, motif `327` lignes 41-49) :
```sql
UPDATE bot_definitions SET config_schema = config_schema || '[
  {"key":"influence_mandate_days","label":"Duree d un mandat (jours)","type":"number","default":"14"},
  {"key":"influence_org_creation_cost","label":"Cout de creation d une organisation","type":"number","default":"1000"},
  {"key":"influence_law_debate_hours","label":"Duree du debat d une loi (h)","type":"number","default":"48"}
]'::jsonb
WHERE bot_name = 'influence-bot' AND NOT (config_schema @> '[{"key":"influence_mandate_days"}]'::jsonb);
```

## 4. Découpage en PHASES

### Phase 1 — MVP : Identité + Organisations + Vote simple  *(jouable seul)*
- **Périmètre :** enregistrement citoyen auto, profil (chiffres pour soi / paliers pour les autres), création d'organisation + adhésion, un vote binaire simple au sein d'une org.
- **Entités :** `citizen`, `capital`, `tier`, `organization`, `org_membership`, `vote` (basique).
- **Tables :** `influence_citizens`, `influence_organizations`, `influence_org_members`, `influence_votes` (migration 328).
- **Jouable seul :** on peut déjà fonder une org, recruter, et trancher une décision par vote — boucle Observer→Agir→Conséquence minimale.

### Phase 2 — Réputation & Capitaux + conversions
- Dimensions de réputation, gains/pertes d'influence, conversions de capitaux (04.md §10). Tables : `influence_reputation_dims`, transactions. Consolide le système de paliers narratifs.

### Phase 3 — Politique : lois, mandats, élections
- Cycle de loi (dépôt→débat→vote→application), élections avec candidatures/mandats, worker `expire_mandates` + `run_elections`. Tables laws/elections/candidacies/mandates. Rend le serveur « gouvernable ».

### Phase 4 — Information & Médias
- Infos (public/privé/secret), enquêtes, fuites, rumeurs, scandales, crédibilité médias. Tables information/investigations. Ajoute la couche manipulation.

### Phase 5 — Monde vivant & Archives
- Worker `world_events`, archives consultables, relations inter-orgs (alliances/boycotts), motions/défiance (06.md §12). Ferme la boucle FOMO (02.md §13).

*(Phase 6 optionnelle : organisations secrètes, désinformation avancée, justice/procès.)*

## 5. Commandes bot par phase

- **P1 :** `/profil [joueur]`, `/org create`, `/org info`, `/org join`, `/org membres`, `/vote`.
- **P2 :** `/reputation`, `/transfert`, `/capital` (voir ses chiffres réels).
- **P3 :** `/loi propose`, `/loi debat`, `/loi vote`, `/candidature`, `/election`, `/mandat`.
- **P4 :** `/enquete`, `/fuite`, `/article publier`, `/rumeur`, `/info vendre`.
- **P5 :** `/archives`, `/actu` (fil d'actualité), `/org alliance`, `/motion defiance`.

## 6. Points d'attention

1. **Paliers narratifs** : `to_tier()` = fonction pure domaine testée exhaustivement (seuils passés en param, comme `CoudeEconomyConfig` dans `tout_ou_rien.rs`). Choix chiffre/palier fait dans `view_profile_service` selon `viewer_id`.
2. **Historique/archives** : `influence_archives` append-only, jamais purgé ; toute action majeure y écrit une entrée (best-effort, ne fait pas échouer la commande — motif log de `play_tout_ou_rien_service.rs:183-196`).
3. **Config web** : chaque paramètre réglable via `config_schema` sur `bot_definitions` (durées de mandat, coûts), chargé par `guild_settings.rs` (cf. `application/coude/guild_settings.rs`).
4. **Rôles/permissions Discord par org** : la hiérarchie (Fondateur..Recrue) mappe vers des rôles Discord gérés par le bot (01.md §8 « gérer les permissions »). Vérifier le rôle avant chaque action org côté service.
5. **Monde évolue via worker** : mandats expirent, élections se déclenchent, lois passent en vote — jobs `sentinel-worker/src/domains/influence/` scannés par index partiels (cf. `domains/coude/mod.rs`).

## 7. Conventions à respecter (tirées de l'existant)

- **Migrations idempotentes**, numérotées, `guild_id` partout, config via `UPDATE bot_definitions ... WHERE NOT (@>)` (`327_game_portal_sessions.sql`).
- **`DomainError`** uniquement dans core : `NotFound/ValidationError/Conflict/Forbidden/RateLimited/Timeout/Internal/NotImplemented` (`sentinel-core/src/domain/errors.rs`).
- **Aucune infra dans `sentinel-core`** : pas de `sqlx`, pas de `serenity` ; les traits ports isolent l'infra.
- **Pattern repo/port** : trait outbound dans core, impl `Pg...` dans `sentinel-api`, câblé dans `bootstrap/app_state.rs` + exposé dans `http/state.rs`.
- **Dispatch bot** : toute nouvelle commande DOIT être routée à **deux** endroits —
  - `sentinel-bot/src/handler.rs` : ajouter le nom au mapping module (`handler.rs:46`) et au `match` de dispatch (`handler.rs:531`), via `modules::influence::handles_command` / `handle_command` ;
  - `sentinel-bot/src/command_registry.rs` : ajouter `"influence-bot" => modules::influence::register_commands()` (`:41`) et l'entrée dans `BOT_NAMES_WITH_COMMANDS` (`:68`).
- **Module bot** expose `register_commands / handle_command / handles_command / handles_component` (`modules/coude/mod.rs:105,109,140,170`).
- **Tests domaine** co-localisés `#[cfg(test)] #[path="tests/..."] mod tests;`.
