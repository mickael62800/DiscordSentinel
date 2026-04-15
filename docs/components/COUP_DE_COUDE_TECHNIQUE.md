# 🔧 Coup de Coude — Architecture technique

Document de référence pour **développeurs**. Explique la mécanique interne
du jeu Coup de Coude : comment le bot, les workers et l'API collaborent,
où vit la logique métier, quelles sont les invariants à respecter pour
ajouter / modifier des fonctionnalités sans rien casser.

> **Version** : post-refacto Phase 0 → 9 (15 avril 2026)
> **Architecture cible** : hexagonale stricte côté API, bot et worker 100 %
> thin (IO Discord + appels gRPC uniquement).
>
> 📐 **Lire aussi** : [`docs/ARCHITECTURE_RULES.md`](../ARCHITECTURE_RULES.md)
> pour les règles d'architecture non-négociables de DiscordSentinel
> (hexagonale, bot/worker thin, RPC, migrations, etc.).

## 📦 Résumé des phases

| Phase | Contenu | Docs |
|---|---|---|
| 0 → 8 | Architecture hexagonale, moteur combat multi-rounds, HP, saisons, catalog RPC | corps de ce document |
| **9A** | Caisse communautaire + `/cagnotte` + worker hebdo redistribution | [Phase 9 addendum](#phase-9-addendum) |
| **9B** | Protections vol en abonnements temps-base + `/protection` (ephemeral) | idem |
| **9C** | Boost voleur en abonnements + `/boost-voleur` | idem |
| **9D** | Railleries automatiques streak 3/5/10 + `/no-taunts` + rename Discord | idem |
| **9E** | Page web admin railleries + channel picker | idem |
| **10** | Braquage hebdomadaire + prison + 9 outils consommables | [Phase 10 addendum](#phase-10-addendum) |

---

## 1. Vue d'ensemble

```
┌─────────────────┐   gRPC   ┌──────────────────────────────────┐
│   coude-bot     │─────────▶│         services/api             │
│   (Discord IO)  │          │                                  │
│                 │          │  ┌──────────────────────────┐   │
│  - catalog.rs   │          │  │ Inbound (gRPC/HTTP)      │   │
│    (cache au    │          │  │  CoudeCombatsService     │   │
│     boot)       │          │  │  CoudePlayerService      │   │
│  - commands/    │          │  │  CoudeBetsService        │   │
│    (20 slash)   │          │  │  CoudeInventoryService   │   │
│  - assemble     │          │  │  CoudeSocialService      │   │
│    les embeds   │          │  │  CoudeEconomyService     │   │
│                 │          │  └──────────┬───────────────┘   │
└─────────────────┘          │             │                   │
                             │  ┌──────────▼───────────────┐   │
┌─────────────────┐          │  │ Application (use cases)  │   │
│  coude-worker   │          │  │  ManageCoudeCombats      │   │
│   (scheduler)   │          │  │  ResolveBettingBatch     │   │
│                 │          │  │  ResolveCombatNow        │   │
│  - resolve_     │  gRPC    │  │  ExpireCombatsBatch      │   │
│    betting      │─────────▶│  │  ManageCoudeCatalog      │   │
│  - hp_regen     │          │  │  ...                     │   │
│  - expire_      │          │  └──────────┬───────────────┘   │
│    combats      │          │             │                   │
└────────┬────────┘          │  ┌──────────▼───────────────┐   │
         │                   │  │ Domain (pur, zero IO)    │   │
         │ post              │  │  coude_combat_engine/    │   │
         │ Discord           │  │    combat / classes      │   │
         ▼                   │  │    chaos / shop          │   │
   Discord API               │  │    progression           │   │
                             │  │  entities/               │   │
                             │  │  value_objects/          │   │
                             │  └──────────┬───────────────┘   │
                             │             │                   │
                             │  ┌──────────▼───────────────┐   │
                             │  │ Ports outbound (traits)  │   │
                             │  │  CoudeCombatRepository   │   │
                             │  │  CoudePlayerRepository   │   │
                             │  │  WalletRepository        │   │
                             │  │  ...                     │   │
                             │  └──────────┬───────────────┘   │
                             │             │                   │
                             │  ┌──────────▼───────────────┐   │
                             │  │ Adapters outbound        │   │
                             │  │  postgres/*.rs (sqlx)    │   │
                             │  │  (SEUL endroit de SQL)   │   │
                             │  └──────────┬───────────────┘   │
                             └─────────────┼───────────────────┘
                                           │
                                           ▼
                                    ┌─────────────┐
                                    │ PostgreSQL  │
                                    └─────────────┘
```

### Principes

1. **Toute la logique métier vit dans l'API**, couche `domain/` et
   `application/` exclusivement.
2. **Le bot est IO Discord pur** : il reçoit les interactions, appelle
   l'API par gRPC, assemble un embed à partir des DTOs reçus, poste.
3. **Le worker est un scheduler thin** : il appelle périodiquement des
   RPCs batch (`ResolveBettingBatch`, `ExpireCombatsBatch`, `HpRegenTick`)
   et poste les résultats sur Discord.
4. **Hexagonal strict** : domain ne connaît pas les adapters, application
   n'importe pas sqlx, adapters postgres sont la seule couche avec SQL.

---

## 2. Arborescence des fichiers clés

### API (`services/api/src/`)

#### Domain (pur, zero IO)

```
domain/
├── entities/
│   ├── coude_player.rs        # CoudePlayer, XpProgress, CombatStat
│   ├── coude_combat.rs        # CoudeCombat, NewCoudeCombat, CombatResolution
│   ├── coude_bet.rs           # CoudeBet, BetResolutionPlan, FighterBetBonus
│   ├── coude_inventory.rs     # CoudeInventoryItem, CoudePrime, CoudeInsurance
│   └── coude_social.rs        # CoudeEvent, CoudeCurrentSeason, LeaderboardCategory
│
├── value_objects/
│   └── coude_class.rs         # CoudeClass enum (Bourrin/Agile/Fourbe/Tank)
│
└── services/
    └── coude_combat_engine/   # ⭐ MOTEUR PUR (zero IO, zero async)
        ├── mod.rs             # PlayerLite, ServerEventLite
        ├── combat.rs          # resolve_combat(...) — cœur du jeu
        ├── classes.rs         # Catalogue classes + stats
        ├── shop.rs            # Catalogue items
        ├── progression.rs     # Formules XP/level/handicap
        └── chaos.rs           # Enum ChaosEvent + roll_chaos()
```

**Règle** : `coude_combat_engine/` **ne doit jamais** importer :
- `sqlx`, `tokio`, `reqwest`, `tonic`
- `crate::adapters::*`
- `async`, `await` (sauf dans les tests)

C'est garanti par `grep -rn "sqlx\|async" coude_combat_engine/` au boot.

#### Application (use cases, orchestration)

```
application/
├── manage_coude_combats_service.rs        # CRUD combats
├── manage_coude_players_service.rs        # CRUD + XP + stats
├── manage_coude_bets_service.rs           # Paris pari-mutuel
├── manage_coude_economy_service.rs        # Transferts wallet coude-side
├── manage_coude_inventory_service.rs      # Items + primes + assurances
├── manage_coude_social_service.rs         # Leaderboard + events + seasons
├── manage_coude_catalog_service.rs        # ⭐ Catalogue statique (Phase 8)
├── resolve_betting_batch_service.rs       # Résolution batch par worker
├── resolve_combat_now_service.rs          # Résolution instantanée (surprise)
└── expire_combats_batch_service.rs        # Expiration batch (> 24h pending)
```

Chaque service implémente un trait du dossier `ports/inbound/`.

#### Ports (traits)

```
ports/
├── inbound/
│   ├── manage_coude_combats.rs      # trait ManageCoudeCombatsUseCase
│   ├── manage_coude_players.rs      # trait ManageCoudePlayersUseCase (add_xp, spend_stat_point, etc.)
│   ├── manage_coude_bets.rs
│   ├── manage_coude_inventory.rs
│   ├── manage_coude_social.rs
│   ├── manage_coude_economy.rs
│   ├── manage_coude_catalog.rs      # GetCatalog -> CoudeCatalog
│   ├── resolve_betting_batch.rs     # ResolveBettingBatch batch use case
│   ├── resolve_combat_now.rs        # ResolveCombatNow use case
│   └── expire_combats_batch.rs
│
└── outbound/
    ├── coude_combat_repository.rs   # trait CoudeCombatRepository
    ├── coude_player_repository.rs
    ├── coude_bet_repository.rs
    ├── coude_inventory_repository.rs
    ├── coude_social_repository.rs
    ├── coude_economy_repository.rs
    └── wallet_repository.rs         # partagé avec blackjack
```

#### Adapters

```
adapters/
├── inbound/
│   ├── grpc/
│   │   └── coude.rs                 # 6 services gRPC tonic
│   └── http/handlers/coude/         # Routes HTTP (panel web)
│
└── outbound/
    └── postgres/
        ├── coude_combat_repository.rs    # sqlx, FOR UPDATE SKIP LOCKED
        ├── coude_player_repository.rs    # sqlx, update stats/XP/HP
        ├── coude_bet_repository.rs       # sqlx, tx paris pari-mutuel
        ├── coude_inventory_repository.rs # sqlx, inventory + primes + insurances
        ├── coude_social_repository.rs    # sqlx, leaderboard + events
        └── coude_economy_repository.rs   # sqlx legacy (avant Phase 8 wallets)
```

### Bot (`bots/coude-bot/src/`)

```
main.rs            # Startup : fetch catalog via gRPC → cache TypeMap
api_client.rs      # Client gRPC pour tous les RPCs coude
catalog.rs         # ⭐ CatalogCache : struct immuable fetched au boot
handler.rs         # Dispatch des interactions Discord
channel_check.rs   # Gate des commandes par salon autorisé
config.rs
guild_config.rs    # Config per-guild (min_bet, max_bet, shop prices...)
commands/          # 20 commandes slash (voir docs/cmd_discord/COUP_DE_COUDE.md)
```

**Pas de `game/`** — les catalogues classes/shop/progression ne vivent plus
côté bot (Phase 8). Tout est fetché via le RPC `GetCatalog` au boot.

### Worker (`services/workers/coude-worker/src/`)

```
main.rs                  # Startup + load_worker_config + heartbeat
scheduler.rs             # spawn_periodic 3 jobs
config.rs                # Intervals (betting_check_secs, hp_regen_tick_secs, ...)
jobs/
├── resolve_betting.rs   # 156 lignes — 1 gRPC call + post Discord
├── expire_combats.rs    # 63 lignes — 1 gRPC call + log
└── hp_regen.rs          # 87 lignes — 1 gRPC call
```

Aucune logique métier, aucun `sqlx::query` dans les jobs. Seules références
à `PgPool` : signature imposée par `worker-common::spawn_periodic`.

---

## 3. Table PostgreSQL — schéma

### `coude_players`
```sql
CREATE TABLE coude_players (
    guild_id        TEXT NOT NULL,
    user_id         TEXT NOT NULL,
    username        TEXT NOT NULL,
    coins           BIGINT NOT NULL DEFAULT 0,  -- legacy, source = user_wallets
    total_wins      INT NOT NULL DEFAULT 0,
    total_losses    INT NOT NULL DEFAULT 0,
    total_draws     INT NOT NULL DEFAULT 0,
    total_earned    BIGINT NOT NULL DEFAULT 0,
    total_lost      BIGINT NOT NULL DEFAULT 0,
    total_stolen    BIGINT NOT NULL DEFAULT 0,
    cowardice_count INT NOT NULL DEFAULT 0,
    chaos_events    INT NOT NULL DEFAULT 0,
    casino_wins     INT NOT NULL DEFAULT 0,
    casino_losses   INT NOT NULL DEFAULT 0,
    level           INT NOT NULL DEFAULT 1,
    xp              BIGINT NOT NULL DEFAULT 0,
    stat_points     INT NOT NULL DEFAULT 0,
    atk             INT NOT NULL DEFAULT 0,
    def             INT NOT NULL DEFAULT 0,
    class           coude_class,                -- enum
    title           TEXT,
    class_changed_at TIMESTAMPTZ,
    hp_current      INT NOT NULL DEFAULT 100,
    hp_max          INT NOT NULL DEFAULT 100,
    hp_last_regen   TIMESTAMPTZ,
    repos_last_used TIMESTAMPTZ,
    season          INT NOT NULL DEFAULT 1,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (guild_id, user_id),
    CONSTRAINT coude_players_coins_non_negative CHECK (coins >= 0)
);
```

### `coude_combats`
```sql
CREATE TABLE coude_combats (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id        TEXT NOT NULL,
    channel_id      TEXT,
    attacker_id     TEXT NOT NULL,
    attacker_name   TEXT NOT NULL,
    defender_id     TEXT NOT NULL,
    defender_name   TEXT NOT NULL,
    mise            BIGINT NOT NULL,
    status          TEXT NOT NULL DEFAULT 'pending',  -- pending/betting/resolving/accepted/refused/expired
    winner_id       TEXT,
    attacker_roll   INT,
    defender_roll   INT,
    chaos_event     TEXT,
    special_attack  TEXT,
    defender_special TEXT,
    coins_transferred BIGINT,
    result_message  TEXT,
    message_id      TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    accepted_at     TIMESTAMPTZ,
    resolved_at     TIMESTAMPTZ
);
CREATE INDEX idx_coude_combats_guild_status ON coude_combats(guild_id, status);
CREATE INDEX idx_coude_combats_attacker_pending ON coude_combats(guild_id, attacker_id) WHERE status = 'pending';
CREATE INDEX idx_coude_combats_defender_pending ON coude_combats(guild_id, defender_id) WHERE status = 'pending';
```

### Tables associées
- `coude_bets` : paris par combat (parieur → cible, montant, payout, won)
- `coude_inventory` : items par joueur (guild_id, user_id, item_key, quantity)
- `coude_primes` : primes actives (target_id, placed_by_id, amount)
- `coude_insurances` : assurances actives (guild_id, user_id, is_scam, expires_at, active)
- `coude_events` : events serveur happy_hour/bloodbath (guild_id, event_type, active, expires_at)
- `coude_daily_chaos` : logs du chaos daily event
- `user_wallets` : wallet partagé (**source de vérité des coins**, pas `coude_players.coins`)

### `CHECK constraints` Phase audit
```sql
-- migration 123_add_coins_check_constraints.sql
ALTER TABLE user_wallets   ADD CONSTRAINT user_wallets_coins_non_negative   CHECK (coins >= 0);
ALTER TABLE coude_players  ADD CONSTRAINT coude_players_coins_non_negative  CHECK (coins >= 0);
```

---

## 4. Cycle de vie d'un combat

### État machine (`coude_combats.status`)

```
┌─────────┐  /coude     ┌─────────┐  /accept   ┌─────────┐  bet_delay   ┌────────────┐
│ (none)  │────────────▶│ pending │───────────▶│ betting │─────────────▶│ resolving  │
└─────────┘             └────┬────┘            └─────────┘              └─────┬──────┘
                             │                                                │
                             │ refus                                          │ worker
                             │ ou 24h                                         │ resolve
                             │                                                │
                             ▼                                                ▼
                        ┌─────────┐                                    ┌──────────┐
                        │ refused │                                    │ accepted │
                        │ expired │                                    │ (final)  │
                        └─────────┘                                    └──────────┘
```

### RPCs déclencheurs par transition

| De → Vers | Déclencheur | Use case |
|---|---|---|
| (none) → pending | Bot `/coude @user mise` → `Create` RPC | `ManageCoudeCombatsService::create` |
| pending → betting | Bouton "Accepter" → `SetBetting` RPC | `ManageCoudeCombatsService::set_betting` |
| pending → refused | Bouton "Refuser" → `Resolve(refused)` RPC | `ManageCoudeCombatsService::resolve` |
| pending → expired (24h) | Worker `expire_combats.rs` → `ExpireCombatsBatch` RPC | `ExpireCombatsBatchService::expire_batch` |
| pending → accepted (instant) | Surprise / Bloodbath / defend_item → `ResolveCombatNow` RPC | `ResolveCombatNowService::resolve_now` |
| betting → resolving | Worker → `ResolveBettingBatch` RPC (claim atomique) | `ResolveBettingBatchService::resolve_batch` |
| resolving → accepted | Idem (dans la même RPC call) | Idem |

### Claim atomique (garantit zéro double-résolution)

```rust
// PgCoudeCombatRepository::claim_due_betting_combats
UPDATE coude_combats SET status = 'resolving'
WHERE id IN (
    SELECT c.id FROM coude_combats c
    LEFT JOIN bot_guild_config cfg
      ON cfg.guild_id = c.guild_id
      AND cfg.bot_name = 'coude-worker'
      AND cfg.config_key = 'bet_delay_secs'
    WHERE c.status = 'betting'
      AND c.accepted_at < NOW() - (COALESCE(cfg.config_value::int, 300) * INTERVAL '1 second')
    FOR UPDATE OF c SKIP LOCKED  -- ⭐ empêche double-traitement
)
RETURNING <colonnes>
```

`FOR UPDATE SKIP LOCKED` → si deux workers tournent en parallèle, un seul
prend le combat. Même chose pour `claim_expired_pending_combats` et
`claim_stuck_resolving_combats` (retry après > 120s bloqués en `resolving`).

---

## 5. Le moteur de combat — `coude_combat_engine::resolve_combat`

Signature :
```rust
pub fn resolve_combat(
    attacker: &Player,
    defender: &Player,
    attacker_current_hp: i32,
    defender_current_hp: i32,
    mise: i64,
    special: Option<&str>,          // item offensif attaquant
    defender_special: Option<&str>, // item défensif défenseur
    active_events: &[ServerEvent],  // happy_hour, bloodbath, ...
) -> CombatResult
```

**Pur** : aucune IO, aucun async, aucun SQL. Juste `rand::thread_rng()` +
logique. Testable en unit tests (voir `combat.rs` tests module).

### Étapes internes

1. **Effective stats** : `ATK = base + (level-1) × growth + points`, idem DEF
2. **Matchmaking handicap** : applique le multiplicateur ATK au joueur plus fort
3. **Items offensifs** : Rage (+50/-30), Coup traître (DEF cible -50 %),
   Double Coup (2d20 garde meilleur), Mindgame (révélation)
4. **Items défensifs** : Bouclier (+20 DEF), Antidote (imm. poison),
   Explosion (early exit)
5. **Early exit Explosion** : si `defender_special == "explosion"`,
   retour immédiat avec `winner_id = None`, `coins_lost_by_loser = mise × 0.5`
6. **Max rounds** : 3 / 5 / 7 selon HP combinés
7. **Boucle de rounds** :
   - Rolls d20 simultanés
   - Passifs classe : Bourrin berserker (HP ≤ 30 %), Agile dodge (15 %),
     Tank blindage (-5, annulé en mirror Tank vs Tank), Fourbe vampirisme
   - Chaos roll (8 % total) : Critique × 2, Esquive Divine (dodge + contre),
     Accident Débile (self-dmg 10 % HP max chacun), Glissade (self-hit),
     Vol à la Tire (5 % mise bonus)
   - Poison (-5 HP/round si actif)
   - Application simultanée des dégâts
   - KO check (HP ≤ 0)
8. **Victory** :
   - KO → winner = l'autre
   - Timeout (max rounds) → winner = plus haut HP %
   - Match nul strict → winner_id = None
9. **Coins calculation** :
   - Base : 70 / 85 / 100 % selon marge HP
   - Fourbe bonus : `+ mise × 0.20` (si winner est Fourbe)
   - Cowardice penalty : `× 0.80` (si `cowardice_count ≥ 5`)
   - Happy hour : `× 2` (si event actif)
   - Tous les overflows protégés par `saturating_mul` / `saturating_add`
10. **Giant Killer** : détecté si `level_gap ≥ 3 && winner.level < loser.level`

### Return `CombatResult`

```rust
pub struct CombatResult {
    pub winner_id: Option<String>,         // None = draw/explosion
    pub loser_id: Option<String>,
    pub rounds: Vec<RoundResult>,          // rolls + chaos + passifs par round
    pub total_rounds: i32,
    pub attacker_hp_final: i32,
    pub defender_hp_final: i32,
    pub attacker_hp_max: i32,
    pub defender_hp_max: i32,
    pub chaos_events_count: i32,
    pub coins_won: i64,                    // ⚠️ sera capé sur solde perdant côté application
    pub coins_lost_by_loser: i64,
    pub stolen_bonus: i64,                 // Fourbe bonus (info)
    pub vol_coins: i64,                    // Vol chaos total (capé côté application)
    pub message: String,                   // Récit du combat formaté pour Discord
    pub is_giant_killer: bool,
    pub attacker_class_revealed: Option<String>,
    pub defender_class_revealed: Option<String>,
}
```

---

## 6. `ResolveCombatNowService` — orchestration instantanée

Appelé pour attaque surprise / bloodbath / defense via item. Flow :

```rust
async fn resolve_now(combat_id: Uuid) -> ResolveCombatNowOutput {
    // 1. Load combat
    let combat = combats_uc.get(combat_id).await?;

    // 2. Load players depuis coude_players
    let attacker = players_uc.get(&combat.guild_id, &combat.attacker_id).await?;
    let defender = players_uc.get(&combat.guild_id, &combat.defender_id).await?;

    // 3. Load events actifs
    let events = social_uc.list_active_events(&combat.guild_id).await?;

    // 4. Moteur pur (domain)
    let result = coude_combat_engine::combat::resolve_combat(...);

    // 5. Persister la résolution du combat
    combat_repo.resolve(combat.id, CombatResolution { ... }).await?;

    // 6. Update HP des 2 joueurs
    players_uc.update_hp(..., result.attacker_hp_final, result.attacker_hp_max).await;
    players_uc.update_hp(..., result.defender_hp_final, result.defender_hp_max).await;

    // 7. Winner path
    if let Some(winner_id) = &result.winner_id {
        // a. Get loser wallet → cap coins sur solde réel
        let loser_balance = wallet_repo.get(...)?.coins;
        let coins_transferred = result.coins_won.min(loser_balance);

        // b. Assurance (consommation + adjust loss)
        if let Some(insurance) = inventory_uc.get_active_insurance(...).await? {
            inventory_uc.expire_insurance(insurance.id).await?;
            if insurance.is_scam { loss *= 2 } else { loss /= 2 }
        }

        // c. Wallet transfers
        wallet_repo.credit(&guild, &winner_id, coins_transferred, "coude_combat_win", desc).await;
        wallet_repo.debit(&guild, loser_id, actual_loss, "coude_combat_loss", desc).await;

        // d. Stats (record_win / record_loss)
        players_uc.record_win(...).await;
        players_uc.record_loss(...).await;

        // e. Primes (claim sur le loser)
        let prime = inventory_uc.claim_primes(...).await?;

        // f. Chaos events count (increment_chaos)
        // g. XP (winner +15/30, loser +5) via add_xp qui retourne XpProgress
        // h. Bets resolve (bets_uc.resolve)
    } else {
        // Draw path : record_draw + refund all bets
    }

    // 8. Build ResolveCombatNowOutput { title, description, color, fields }
    //    → renvoyé au bot, prêt pour embed Discord
}
```

### Output DTO

```rust
pub struct ResolveCombatNowOutput {
    pub combat_id: String,
    pub title: String,       // "⚔️ Résultat du Coup de Coude !"
    pub description: String, // result.message (récit complet)
    pub color: u32,          // 0x57F287 (vert victoire) | 0x9B59B6 (chaos/draw)
    pub fields: Vec<ResolvedCombatEmbedField>,  // Combat / XP / Primes / Assurance / Paris
}
```

Le bot lit ça et fait :
```rust
let mut embed = CreateEmbed::new().title(&out.title).description(&out.description).color(out.color);
for f in out.fields { embed = embed.field(f.name, f.value, f.inline); }
```

---

## 7. `ResolveBettingBatchService` — orchestration batch (worker)

Même logique que `ResolveCombatNowService` mais :
- Opère sur **N combats à la fois** (claim atomique SKIP LOCKED)
- Loop interne : pour chaque combat resolved, construit un `ResolvedBettingCombatOutput`
- Le worker reçoit la liste et **poste chaque résultat sur Discord** individuellement
  (edit du message original via `message_id`, ou fallback post si edit fail)

Différence clé : **le worker n'est pas à l'origine du combat**, il le traite
de manière asynchrone 5 min après la création. Donc il y a une **phase de paris
ouverte** pendant laquelle les tiers peuvent miser via `/pari`.

---

## 8. `ManageCoudeCatalogService` — la source de vérité (Phase 8)

Avant Phase 8, les classes/shop/progression étaient **dupliqués** entre
`bots/coude-bot/src/game/` et `services/api/src/domain/services/coude_combat_engine/`.
Toute modification devait être portée aux 2 endroits → risque de désynchro.

Phase 8 a éliminé la duplication :

```rust
#[async_trait]
impl ManageCoudeCatalogUseCase for ManageCoudeCatalogService {
    async fn get_catalog(&self) -> Result<CoudeCatalog, DomainError> {
        Ok(CoudeCatalog {
            classes: [&CLASS_BOURRIN, &CLASS_AGILE, ...].map(ClassInfo::from_domain).collect(),
            shop_items: SHOP_ITEMS.iter().map(|i| ShopItemInfo {
                // ⭐ heal_amount calculé ici, pas côté bot
                heal_amount: match i.key {
                    "potion_soin" => 30,
                    "potion_majeure" => 80,
                    _ => 0,
                },
                ...
            }).collect(),
            level_table: (1..=MAX_LEVEL).map(|lvl| LevelEntry {
                level: lvl,
                title: title_for_level(lvl),
                xp_cumul: xp_for_level(lvl),
            }).collect(),
            matchmaking_buckets: vec![
                MatchmakingBucket { gap_min: 0, gap_max: 2, handicap: 1.0, blocked: false },
                MatchmakingBucket { gap_min: 3, gap_max: 5, handicap: 0.8, blocked: false },
                MatchmakingBucket { gap_min: 6, gap_max: 9, handicap: 0.6, blocked: false },
                MatchmakingBucket { gap_min: 10, gap_max: 999, handicap: 0.0, blocked: true },
            ],
            anti_theft_items: shop::ANTI_THEFT_ITEMS.iter().map(...).collect(),
            max_level: MAX_LEVEL,
            hp_base: 100,
            hp_per_def: 2,
        })
    }
}
```

### Bot : fetch au boot, cache en mémoire

```rust
// bots/coude-bot/src/main.rs
let catalog = api_client.get_catalog().await.expect("fetch catalog");
data.insert::<catalog::CatalogCacheKey>(Arc::new(catalog));

// bots/coude-bot/src/catalog.rs
impl CatalogCache {
    pub fn get_class(&self, name: &str) -> &ClassInfo { ... }
    pub fn get_item(&self, key: &str) -> Option<&ShopItemInfo> { ... }
    pub fn title_for_level(&self, level: i32) -> &str { ... }
    pub fn xp_for_level(&self, level: i32) -> i64 { ... }
    pub fn matchmaking_handicap(&self, atk: i32, def: i32) -> (f64, bool) { ... }
    pub fn display_hp(&self, def: i32) -> i32 { self.hp_base + def * self.hp_per_def }
    pub fn is_potion(&self, key: &str) -> bool { ... }
    pub fn potion_heal_amount(&self, key: &str) -> i32 { ... }
}
```

**Conséquence** : les 20 commandes bot font des `catalog.get_xxx()` qui sont
de simples lookups dans des Vec fetchés au boot. **Zéro formule en dur** dans
le bot (à l'exception de `display_hp` qui est `hp_base + def * hp_per_def`,
mais les 2 paramètres viennent de l'API donc la formule est tunable sans
toucher le bot tant que la forme reste linéaire).

### Invalidation du cache

**Le bot fetch le catalog uniquement au boot.** Pour propager une modification
du catalog (nouvel item, changement de formule XP), il faut **redémarrer
le bot**. C'est voulu : les modifications catalog sont rares et le restart
est immédiat.

---

## 9. Le worker : 3 jobs thin

### `jobs/resolve_betting.rs` (156 lignes)

```rust
pub async fn run(_pool: &PgPool, _api_url: &str, bot_token: &str) -> Result<(), String> {
    // 1. gRPC call
    let combats = call_resolve_batch().await?;

    // 2. Post results to Discord
    for combat in combats {
        post_result_to_discord(
            bot_token,
            &combat.channel_id,
            combat.message_id.as_deref(),
            &combat.result_message,
            combat.is_draw,
        ).await;
    }
    Ok(())
}
```

Le `_pool` n'est pas utilisé, c'est juste pour respecter la signature
`spawn_periodic` de `worker-common`.

### `jobs/expire_combats.rs` (63 lignes)

```rust
pub async fn run(_pool: &PgPool) -> Result<(), String> {
    // 1. gRPC call
    let response = client.expire_combats_batch(Empty {}).await?;

    // 2. Log (pas de post Discord — expiration silencieuse)
    for c in response.combats {
        warn!(combat_id = %c.combat_id, defender = %c.defender_name,
              penalty = c.penalty, "Combat expire");
    }
    Ok(())
}
```

### `jobs/hp_regen.rs` (87 lignes)

```rust
pub async fn run(_pool: &PgPool) -> Result<(), String> {
    // 1. Read rates from env (ou defaults 100/50/30/10)
    let rate_0_25 = env_rate("HP_REGEN_RATE_0_25", 100.0);
    // ... etc
    
    // 2. gRPC call
    client.hp_regen_tick(HpRegenTickRequest { rate_0_25, ... }).await?;
    Ok(())
}
```

Côté API, le RPC fait un seul `UPDATE coude_players` avec une CTE qui
recalcule le tier de régen par joueur et exclut ceux en combat actif :

```sql
WITH regen AS (
    SELECT guild_id, user_id,
        FLOOR((CASE
            WHEN hp_current * 4 < hp_max THEN $1::float8       -- [0, 25%)
            WHEN hp_current * 2 < hp_max THEN $2::float8       -- [25%, 50%)
            WHEN hp_current * 4 < hp_max * 3 THEN $3::float8   -- [50%, 75%)
            ELSE $4::float8                                     -- [75%, 100%]
        END) * EXTRACT(EPOCH FROM (NOW() - hp_last_regen)) / 3600.0)::int AS amount
    FROM coude_players p
    WHERE hp_current < hp_max
      AND hp_last_regen IS NOT NULL
      AND NOT EXISTS (
          SELECT 1 FROM coude_combats c
          WHERE c.guild_id = p.guild_id
            AND (c.attacker_id = p.user_id OR c.defender_id = p.user_id)
            AND c.status IN ('pending', 'betting', 'resolving')
      )
)
UPDATE coude_players p
SET hp_current = LEAST(p.hp_max, p.hp_current + r.amount),
    hp_last_regen = NOW(),
    updated_at = NOW()
FROM regen r
WHERE p.guild_id = r.guild_id AND p.user_id = r.user_id AND r.amount > 0
```

Le `NOT EXISTS` est crucial pour éviter que le regen écrase un `hp_current`
frais posé par une résolution de combat concurrente.

---

## 10. Points critiques & invariants

### Money safety (corrigé en audit Phase 7)

1. **Cap coins sur solde perdant** : `result.coins_won.min(loser_balance)`
   dans `ResolveCombatNowService` et `ResolveBettingBatchService`.
   Empêche la création de coins ex-nihilo.

2. **Cap vol_coins** : idem, calculé après le débit principal.

3. **CHECK constraints** `coins >= 0` sur `user_wallets` et `coude_players`.
   Filet de sécurité DB-level contre les régressions applicatives.

4. **`saturating_mul` / `saturating_add`** sur toutes les multiplications de
   `coins_won` dans `combat.rs` pour éviter les overflows i64.

5. **Atomic `buy_insurance`** : `INSERT ... WHERE NOT EXISTS` pour empêcher
   la double-exécution sur appel concurrent (auparavant : 2 debits possibles).

6. **`place_bet` race fermée** : `SELECT status FROM coude_combats FOR UPDATE`
   dans la transaction du repo, avant le debit du bettor. Si le worker a
   bougé le status en `resolving` entre le check service-layer et le debit,
   le bet est rejeté.

### Race conditions à surveiller

- **Stuck combats** : retry après 120s via `claim_stuck_resolving_combats`
- **HP regen pendant combat** : `NOT EXISTS` sur combats actifs
- **Worker double-resolution** : `FOR UPDATE SKIP LOCKED` sur claim

### Errors logging

Historiquement, `let _ = sqlx::query(...)` avalait les erreurs. Phase audit :
tous convertis en `if let Err(e) = ... { warn!(error = %e, ...) }`.
Les erreurs remontent maintenant dans les logs worker/API.

---

## 11. Ajouter une nouvelle fonctionnalité — checklist

### Ajouter un nouvel item au shop

1. Éditer **`services/api/src/domain/services/coude_combat_engine/shop.rs`** :
   ```rust
   ShopItem {
       key: "bouclier_magique",
       name: "Bouclier Magique",
       emoji: "🔮",
       price: 500,
       description: "Bloque le premier coup critique du combat",
       category: "defense",
   },
   ```

2. Si l'item a un effet combat, éditer **`combat.rs`** pour ajouter le handler :
   ```rust
   if defender_special == Some("bouclier_magique") {
       // logique d'effet
   }
   ```

3. Si c'est une potion avec heal_amount différent, éditer
   **`manage_coude_catalog_service.rs`** :
   ```rust
   heal_amount: match i.key {
       "potion_soin" => 30,
       "potion_majeure" => 80,
       "bouclier_magique" => 0,  // pas une potion
       _ => 0,
   },
   ```

4. Rebuild l'API + restart du bot (pour refresh le catalog cache).

**Aucun fichier côté bot à toucher** tant que l'item n'a pas besoin d'un
nouveau bouton Discord ou d'une nouvelle commande slash dédiée.

### Ajouter une nouvelle classe

1. Éditer **`services/api/src/domain/services/coude_combat_engine/classes.rs`** :
   ```rust
   pub const CLASS_MAGE: ClassStats = ClassStats {
       name: "mage",
       emoji: "🧙",
       base_atk: 20,
       base_def: 10,
       atk_growth: 3,
       def_growth: 2,
       dodge_chance: 0.0,
       steal_bonus: 0.0,
       description: "Sort de mana",
       passif_key: "arcane",
       passif_description: "Arcane : ...",
       passif_reveal: "...",
   };
   ```

2. Ajouter dans `manage_coude_catalog_service.rs` la liste :
   ```rust
   let classes_data: Vec<ClassInfo> = [
       &classes::CLASS_BOURRIN, &classes::CLASS_AGILE,
       &classes::CLASS_FOURBE, &classes::CLASS_TANK,
       &classes::CLASS_MAGE,   // ⭐ ajouté
   ].iter().map(|c| ClassInfo { ... }).collect();
   ```

3. Ajouter dans **`combat.rs`** le passif (section de boucle round) :
   ```rust
   if atk_class.name == "mage" && /* condition */ {
       // effet arcane
   }
   ```

4. Mettre à jour le enum `CoudeClass` dans
   **`domain/value_objects/coude_class.rs`** (parse + as_str).

5. Ajouter la value `'mage'` au type PostgreSQL `coude_class` via migration SQL :
   ```sql
   ALTER TYPE coude_class ADD VALUE 'mage';
   ```

6. Rebuild API + bot.

### Ajouter un chaos event

1. Éditer **`coude_combat_engine/chaos.rs`** :
   ```rust
   pub enum ChaosEvent {
       CritiqueSauvage, EsquiveDivine, AccidentDebile, Glissade, Vol,
       MonNouveauEvent,  // ⭐ ajouté
   }
   impl ChaosEvent {
       pub fn key(&self) -> &'static str { match self { ... Self::MonNouveauEvent => "mon_nouveau" } }
       pub fn emoji(&self) -> &'static str { ... }
       pub fn label(&self) -> &'static str { ... }
   }
   pub fn roll_chaos() -> Option<ChaosEvent> {
       let roll: u32 = rng.gen_range(1..=1000);
       match roll {
           1..=20 => Some(CritiqueSauvage),   // 2%
           21..=40 => Some(EsquiveDivine),
           41..=55 => Some(AccidentDebile),
           56..=65 => Some(Glissade),
           66..=80 => Some(Vol),
           81..=90 => Some(MonNouveauEvent),  // ⭐ 1%
           _ => None,
       }
   }
   ```

2. Éditer **`combat.rs`** pour ajouter le handler du nouveau event dans
   la match `ChaosEvent::` (section chaos du round loop).

3. `cargo test -p sentinel-api coude_combat_engine` pour valider.

4. Rebuild API.

---

## 12. Tests et validation

### Tests unitaires du moteur

```
services/api/src/domain/services/coude_combat_engine/combat.rs
├── resolve_produit_toujours_un_result_coherent        (smoke test)
├── explosion_retourne_draw_avec_loss_50_pct          (explosion flow)
├── tank_vs_tank_ne_bloque_pas_a_1_dmg                (mirror exception)
└── draw_path_pas_de_winner                           (invariants égalité)
```

`cargo test -p sentinel-api coude` → 28 tests verts couvrant les 4 couches.

### Tests d'intégration

Pas encore couvert à ce jour — à ajouter dans
`services/api/tests/integration_coude.rs` (patterns existants dans
`integration_blackjack.rs`).

### Test manuel E2E post-déploiement

1. `/profil` → crée un joueur
2. `/classe` → choisir une classe (gratuit)
3. `/coude @autre_joueur 50` → créer un combat
4. L'autre : `/accepter` ou bouton
5. Attendre 5 min (betting phase)
6. Résolution automatique par le worker
7. Vérifier dans la DB : `coude_combats.status = 'accepted'`,
   `user_wallets` mis à jour, `coude_players.total_wins` incrémenté.

---

## 13. Déploiement

```bash
# Rebuild + redémarrage
docker compose build api coude-bot coude-worker
docker compose up -d --force-recreate api coude-bot coude-worker

# Vérifier les logs
docker compose logs api coude-bot coude-worker --tail=50

# Vérifier que le catalog est fetché côté bot
docker compose logs coude-bot | grep "Catalogue Coude"
```

Les migrations PostgreSQL (`services/api/migrations/*.sql`) sont appliquées
automatiquement au boot de l'API via sqlx migrate.

---

## 14. Pour aller plus loin

- **`docs/cmd_discord/COUP_DE_COUDE.md`** : guide joueur (toutes les commandes)
- **`docs/ARCHITECTURE_RULES.md`** : règles d'architecture non-négociables
- **`docs/COUDE_REFACTOR_PLAN.md`** : historique du refacto Phase 0 → 8
- **`services/proto/proto/coude.proto`** : spec complète des RPCs gRPC
- **`services/api/src/adapters/inbound/grpc/coude.rs`** : implémentation des
  6 services gRPC tonic
- **`services/api/src/adapters/outbound/postgres/coude_*.rs`** : seul endroit
  du codebase où `sqlx::query` est appelée pour le jeu

---

## Phase 9 addendum

La Phase 9 ajoute 5 sous-features gameplay qui s'enchaînent pour
créer une boucle économique vivante et des interactions sociales
punitives. Toutes respectent l'architecture hexagonale et le
principe bot/worker thin.

### Vue d'ensemble

```
                    ┌──────────────────────┐
                    │ coude_cashbox        │  Part A
                    │ (balance par guild)  │
                    └──────────┬───────────┘
                               │ alimentée par
       ┌───────────────────────┼──────────────────────┐
       │                       │                      │
  /shop achats           /assurance             Pénalité lâcheté
  /classe change         /donner (taxe)         /reset-stats
  /protection (Part B)   /boost-voleur (C)
                               │
                               │ redistribuée hebdo
                               ▼
                    ┌──────────────────────┐
                    │ coude_worker hebdo   │
                    │ RedistributeDue…     │
                    └──────────┬───────────┘
                               │ crédite
                               ▼
                    joueurs actifs (7j)

                    ┌──────────────────────┐
                    │ coude_players        │  Part D
                    │ + current_win_streak │
                    │ + current_loss_streak│
                    │ + current_steal_vic_…│
                    └──────────┬───────────┘
                               │ lu par taunts_uc
                               │ au record_win/loss/stolen
                               ▼
                    ┌──────────────────────┐
                    │ TauntEvent           │
                    │ (channel, msg,       │
                    │  nickname_suffix)    │
                    └──────────┬───────────┘
                               │ dispatché par
                  ┌────────────┴──────────────┐
                  ▼                           ▼
           coude-bot                   coude-worker
      (resolve_combat_now,       (resolve_betting post-
       voler.rs)                  phase de paris)
                  │                           │
                  └──────────┬────────────────┘
                             ▼
                   Discord (post + rename)
```

### Part A — Caisse communautaire (Cashbox)

**Problème résolu** : avant Phase 9, tous les coins dépensés au shop
et en assurance disparaissaient de l'économie → contraction à long
terme.

**Solution** : table `coude_cashbox` par guild qui collecte tous les
flux sortants, redistribués chaque semaine aux joueurs actifs avec
un tirage aléatoire **exponentiel** (effet loterie disparate, un gros
gagnant + 19 petits).

**Fichiers clés** :
- `services/api/migrations/124_coude_cashbox.sql` — 3 tables
  (cashbox, redistributions, entries).
- `domain/entities/coude_cashbox.rs` — entité + enum `CashboxSource`.
- `application/manage_coude_cashbox_service.rs` — algorithme
  `distribute_random(total, n)` avec `-ln(r)` exponentielle.
- `adapters/outbound/postgres/coude_cashbox_repository.rs` —
  `claim_all_for_redistribution` atomique via `SELECT FOR UPDATE` +
  `UPDATE`.
- `services/workers/coude-worker/src/jobs/redistribute_cashbox.rs` —
  tick 1h, filtre `min_days_since_last = 7` côté API.
- `bots/coude-bot/src/commands/cagnotte.rs` — `/cagnotte` display-only.

**RPCs** (`CoudeSocialService`) : `GetCashbox`, `DepositCashbox`,
`RedistributeCashbox`, `RedistributeDueCashboxes`.

**Intégrations dépôt** (où le bot appelle `deposit_cashbox` après
un débit wallet) :
- `commands/shop_cmd.rs` → `ShopPurchase`
- `commands/assurance.rs` → `InsurancePurchase`
- `commands/classe.rs` (500c change) → `ClassChangeCost`
- `commands/reset_stats.rs` (300c) → `ResetStatsCost`
- `commands/donner.rs` (taxe 10 %) → `DonationTax`
- `application/expire_combats_batch_service.rs` (pénalité) →
  `CowardicePenalty` (côté API directement, pas via le bot)

### Part B — Protections vol en abonnements

**Problème résolu** : les anciens items anti-vol (`chien_garde`,
`camera`, `coffre_fort`) étaient consommés au blocage et visibles
dans l'inventaire public → cassait l'effet de surprise.

**Solution** : nouvelle table `coude_steal_protections` qui stocke
des abonnements temps-base (1/3/5/7 j) par joueur. **Aucun item
n'est consommé** ; ils rollent à chaque tentative jusqu'à expiration.
La commande `/protection` répond **ephemeral** pour que les voleurs
ne voient rien.

**Fichiers clés** :
- `migrations/125_coude_steal_protections.sql` — table + migration
  des items existants en 3 jours gratuits.
- `domain/entities/coude_steal_protection.rs` — 8 items
  (chien/alarme/piège/caméra/leurre/garde/coffre/forteresse), grille
  de prix 1-7 j avec remise 0/10/15/20 %.
- `application/manage_coude_steal_protections_service.rs` —
  `try_trigger` qui roll dans l'ordre décroissant de block chance,
  premier blocage stoppe.
- `bots/coude-bot/src/commands/protection.rs` — commande ephemeral.
- `commands/voler.rs::try_trigger_protection` — délégué à l'API.

**RPCs** (`CoudeInventoryService`) : `ListActiveStealProtections`,
`PriceStealProtection`, `BuyStealProtection`,
`TryTriggerStealProtection`.

### Part C — Boost voleur (symétrique)

**Solution** : table `coude_steal_boosts` identique en shape, mais
les items ajoutent un **bonus flat au roll d20** du voleur au lieu
de bloquer. **Cumulatif** : tous les items actifs s'additionnent
(Crochet +5 + Marteau +25 = +30 au roll).

**Fichiers clés** :
- `migrations/126_coude_steal_boosts.sql`
- `domain/entities/coude_steal_boost.rs` — 5 items
  (crochet/passe-partout/déguisement/fumigène/marteau).
- `application/manage_coude_steal_boosts_service.rs` —
  `total_bonus` = somme des `roll_bonus` des actifs.
- `commands/boost_voleur.rs` — commande ephemeral.
- `commands/voler.rs::resolve_steal_attempt` appelle
  `get_steal_boost_total` avant le roll.

**Affichage conditionnel** : si `boost_bonus == 0`, n'affiche que
`class: X`. Sinon affiche `class: X + boost: Y` — évite de leaker
l'absence de boost aux curieux.

### Part D — Railleries automatiques (streaks)

**Comportement** : 3 compteurs par joueur
(`current_win_streak`, `current_loss_streak`,
`current_steal_victim_streak`) incrémentés au `record_win/loss` ou
au track vol. Paliers **3/5/10** → `TauntEvent` cuisiné côté API
avec :
- `channel_id` (config par guild)
- `message` (tiré aléatoirement du catalogue par kind × palier)
- `nickname_suffix` (constant par kind × palier, max 24 chars)
- `streak_kind`, `streak_value` (pour couleur de l'embed + footer)

Le bot/worker **postent et renomment tel quel**, aucune décision.

**Fichiers clés** :
- `migrations/127_coude_taunts.sql` — 3 colonnes streak sur
  `coude_players` + tables `coude_taunts_config` et
  `coude_taunts_opt_outs`.
- `domain/entities/coude_taunt.rs` — `TAUNT_THRESHOLDS = [3, 5, 10]`,
  9 catalogues de messages (3 kinds × 3 paliers), 9 suffixes,
  `build_taunt_event` qui décide.
- `ports/outbound/coude_player_repository.rs` — extensions
  `touch_win_streak` / `touch_loss_streak` /
  `touch_steal_victim_streak` / `reset_*` qui retournent la nouvelle
  valeur.
- `application/manage_coude_taunts_service.rs` — orchestre streak
  update + config lookup + opt-out check + domain call.
- `application/resolve_betting_batch_service.rs` +
  `application/resolve_combat_now_service.rs` — appellent
  `on_player_won/lost/drew` après `record_*`, pushent les
  `TauntEvent` dans l'output.
- `bots/coude-bot/src/taunts_dispatch.rs` — dispatch IO pur
  (post embed + serenity `edit_member`).
- `services/workers/coude-worker/src/jobs/resolve_betting.rs` —
  `dispatch_taunt_event` via reqwest brut (post REST + PATCH member).
- `commands/no_taunts.rs` et `commands/taunts_channel.rs`.

**Reset** :
- Win streak reset sur défaite ou draw.
- Loss streak reset sur victoire ou draw.
- Steal victim streak reset quand le vol est bloqué (par protection)
  ou raté (rolls défavorables au voleur).

### Part E — Page web admin railleries

**Endpoints** (sous `/api/coude/{guild_id}/config/taunts`) :
- `GET` — retourne la config + liste des opt-outs
- `PUT` — update channel + enabled (Admin+ via RBAC)
- `DELETE /.../opt-outs/{user_id}` — retrait forcé (Admin+)
- `GET /api/guilds/{guild_id}/channels` — endpoint channel picker
  (utilise `DiscordApiService::list_text_channels`, cache Redis 10min)

**Page Vue** : `apps/web/src/components/pages/CoudeTauntsConfigPage.vue`
avec AppSelect (channel picker), AppToggle (enabled), liste des
opt-outs. Route `/coude/taunts` enregistrée dans `router/index.ts`.

### Décisions d'architecture Phase 9

Plusieurs valeurs ont été **intentionnellement hardcodées** avec
commentaires `**Choix d'architecture**` dans le domain :
- Prix des items protection/boost (grille validée à la conception)
- Seuils de railleries [3, 5, 10] (liés au catalogue de messages)
- `MAX_WINNERS = 20`, `ACTIVE_WINDOW_DAYS = 7` pour la cashbox

Les rendre per-guild configurables demanderait de refactorer les
services pour accepter des overrides runtime, sans valeur gameplay
réelle pour la quantité de complexité ajoutée. Si ça doit bouger,
modifier le code et redéployer.

### Limitations connues

- **Worker dispatch taunts** utilise reqwest brut (pas serenity) car
  le worker n'a pas de contexte serenity. Le code est plus verbeux
  mais isolé dans `resolve_betting.rs::dispatch_taunt_event`.
- **Nickname rename** est best-effort : si le bot n'a pas la
  permission `Manage Nicknames` ou si le user est admin du serveur,
  le PATCH échoue silencieusement (logged en warn).
- **Cagnotte "max winners"** : fixé à 20. Une caisse très grosse
  (ex. 100 k) donnera donc au max 20 gagnants, soit ~5 k moyenne, ce
  qui est volontaire (effet loterie).

---

## Phase 10 addendum

Système `/braquage` : une fois par semaine, un joueur peut tenter
de siphonner la caisse communautaire. Taux de base très faible (5 %),
boost par items consommables achetés via `/shop braquage`. En cas
d'échec, le joueur est envoyé en **prison** 24 h et ne peut plus
jouer à rien.

### Vue d'ensemble

```
   /shop braquage acheter:<tool>
               │
               ▼
    inventaire joueur (coude_inventory)
               │ liste au moment du braquage
               ▼
         /braquage
               │
    ┌──────────┼───────────┐
    │          │           │
    ▼          ▼           ▼
coude_heist  cashbox    coude_inventory
_attempts    withdraw   (use_item x N)
    │
    │ si echec
    ▼
 coude_prison  ←─── prison_check.rs (middleware bot)
    │                       ▲
    │                       │ bloque /coude /voler /pari /prime etc.
```

### Règles gameplay

| Paramètre | Valeur | Source |
|---|---|---|
| Cooldown | 7 jours | `HEIST_COOLDOWN_DAYS` |
| Chance de base | 5 % | `HEIST_BASE_SUCCESS_PERCENT` |
| Bonus par item | +5 % | `HEIST_ITEM_BONUS_PERCENT` |
| Cap maximum | 50 % | `HEIST_MAX_SUCCESS_PERCENT` |
| Gain sur succès | 30-75 % (aléatoire) | `HEIST_GAIN_MIN_PERCENT`/`_MAX_` |
| Prison sur échec | 24 h | `HEIST_PRISON_HOURS` |

Toutes les constantes vivent dans
`services/api/src/domain/entities/coude_heist.rs` avec note
« Choix d'architecture » — hardcodées à cause du catalogue de 9
items spécifiquement calibré pour atteindre 50 % avec tous les
items (5 + 9 × 5 = 50).

### Fichiers clés

- **`migrations/128_coude_heist.sql`** — 2 tables : `coude_heist_attempts`
  (log + cooldown) et `coude_prison` (état prison par user).
- **`domain/entities/coude_heist.rs`** — constantes + catalogue 9
  outils (`HEIST_TOOLS`) + `compute_success_chance` pur (testé) +
  types `HeistOutcome`, `CoudeHeistAttempt`, `CoudePrisonState`.
- **`ports/outbound/coude_heist_repository.rs`** — trait avec
  `last_attempt` (cooldown), `record_attempt`, `get_prison`,
  `send_to_prison`.
- **`adapters/outbound/postgres/coude_heist_repository.rs`** — impl
  Postgres (UPSERT sur coude_prison, INSERT simple sur attempts).
- **`ports/outbound/coude_cashbox_repository.rs`** étendu avec
  `withdraw(guild_id, amount)` : transaction SELECT FOR UPDATE +
  UPDATE clamp à 0. Utilisé par le braquage pour décrémenter la
  caisse sans passer par `claim_all_for_redistribution` (qui vide
  tout).
- **`ports/inbound/manage_coude_heist.rs`** — use case avec
  `get_cooldown_status`, `get_prison_status`, `attempt_heist`.
- **`application/manage_coude_heist_service.rs`** — orchestre :
  1. check prison (error si en prison)
  2. check cooldown 7j
  3. check caisse non-vide
  4. liste inventory, filtre sur `HEIST_TOOLS`, dedup
  5. `compute_success_chance` (domain pur)
  6. roll aléatoire + gain aléatoire 30-75 %
  7. consomme tous les outils (`use_item`) quel que soit le résultat
  8. si succès : `cashbox.withdraw` puis `wallet.credit`
  9. log via `record_attempt`
  10. si échec : `send_to_prison`
- **`adapters/inbound/grpc/coude.rs`** — 3 handlers : `AttemptHeist`,
  `GetHeistCooldown`, `GetPrisonStatus`.

### Bot side

- **`commands/braquage.rs`** — commande slash thin : defer public,
  `attempt_heist` API call, affichage embed (or sur succès, rouge
  sur échec avec date de libération prison).
- **`commands/shop_cmd.rs`** — 3e sous-commande `/shop braquage`
  ajoutée avec les 9 outils dans les string_choices.
- **`domain/services/coude_combat_engine/shop.rs`** — 9 items
  ajoutés à `SHOP_ITEMS` avec `category = "braquage"`.
- **`prison_check.rs`** — middleware bot appelé depuis `handler.rs`
  AVANT le dispatch des slash commands. Whitelist de commandes
  bloquées en prison (tout le gameplay : coude, voler, pari, prime,
  potion, shop, protection, boost-voleur, train, classe, donner,
  repos, reset-stats, braquage). Les commandes passives (profil,
  cagnotte, leaderboard, etc.) passent. Fail-open si l'API est
  down.

### Décisions d'architecture

- **Constantes hardcodées** : cf. note sur les autres phases. Le
  catalogue 9 items + le cap 50 % + la grille 5 % sont couplés — on
  ne peut pas changer un paramètre sans rebalancer tout. Redéployer
  pour tuner.
- **Prison check côté bot** (pas côté API) : on fait un pré-check
  via `GetPrisonStatus` RPC avant le dispatch, au lieu d'enforcer
  dans chaque use case. Justifié par :
  - La prison est transversale (15+ commandes concernées), y mettre
    un check dans chaque use case multiplierait le code.
  - Le RPC `GetPrisonStatus` est super simple (1 SELECT) donc pas
    de coût notable.
  - Fail-open : si l'API est down, on laisse jouer (UX > strictness).
  - L'API reste l'autorité finale : elle enforce quand même la
    prison pour `/braquage` directement dans `attempt_heist`.
- **Items consommés même en échec** : coût d'entrée du braquage.
  Sans ça, le joueur pourrait spammer sans risquer ses items.
- **Pas de logique "choisir quels items utiliser"** : le service
  consomme TOUS les items de braquage dans l'inventaire. Le joueur
  décide quand braquer, pas avec quoi. Simplifie énormément la
  commande (pas de UI multi-select).

### Limitations connues

- **Une seule tentative par semaine même sur succès** : le cooldown
  s'applique toujours. Tu ne peux pas "profiter" d'une caisse qui
  grossit vite.
- **Pas de feedback avant tentative** : le joueur ne voit pas sa
  chance calculée avant de lancer `/braquage`. On pourrait ajouter
  une sous-commande `/braquage check` qui affiche la chance sans
  consommer — follow-up possible.
- **Prison non appelable par admin** : pas de commande
  `/admin-prison` pour libérer manuellement un joueur. Il faut
  passer par un UPDATE SQL direct en cas de besoin. Follow-up si
  nécessaire.

---

*Dernière mise à jour : 15 avril 2026 — post Phase 8 (bot 100 % thin,
catalog API source unique de vérité).*
