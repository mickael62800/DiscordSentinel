# Économie des organisations « Influence » — Trésorerie / Cagnotte d'organisation

> Document de conception. Se lit à la suite de `ARCHITECTURE.md`, `04.md §4 & §9`, `05.md §5 & §9`.
> **Conventions** calquées sur l'existant : module `coude` (cashbox) + `casino` (wallet unifié).
> Aucune dépendance infra dans `sentinel-core`. `guild_id TEXT` partout. Migrations idempotentes.

---

## 0. TL;DR

- L'argent des **joueurs** existe déjà : une **monnaie unique et partagée** entre tous les jeux, la table `user_wallets` (migration `080`), pilotée par le use case unifié `ManageWalletUseCase` (`sentinel-core/src/ports/inbound/casino/manage_wallet.rs`).
- **Recommandation : Influence NE crée PAS de monnaie dédiée.** Le capital « Argent » d'un citoyen (`04.md §4`) = son solde `user_wallets.coins`. On réutilise `ManageWalletUseCase`. La colonne `money` de `influence_citizens` prévue dans `ARCHITECTURE.md §3` devient **redondante** et doit être **abandonnée** au profit du wallet partagé (voir §2).
- La **trésorerie d'organisation** est une **cagnotte** modelée sur `coude_cashbox` (migration `124`) : une table de solde par org + une table de mouvements immuables (append-only) pour l'historique. Les membres l'alimentent en reversant leurs coins ; les dirigeants la dépensent (salaires, campagnes, projets).
- Les mouvements trésorerie déplacent des coins **depuis/vers `user_wallets`** de façon atomique — la trésorerie n'imprime pas d'argent, elle est un « coffre » commun libellé dans la même monnaie.

---

## 1. Analyse de l'économie existante (factuel)

### 1.1 La monnaie canonique du joueur : `user_wallets` (monnaie unifiée)

- **Table** : `user_wallets` — `sentinel-api/migrations/080_create_user_wallets.sql:5`.
  Colonnes : `id UUID`, `guild_id TEXT`, `user_id TEXT`, `username`, `coins BIGINT`, `total_earned BIGINT`, `total_spent BIGINT`, `created_at/updated_at`, `UNIQUE(guild_id,user_id)`.
  L'en-tête du fichier le dit explicitement : *« Wallet partagé — système de coins unifié entre tous les jeux »* (`080:2`).
- **Historique** : `wallet_transactions` (`080:22`) — `amount` (positif=crédit, négatif=débit), `balance_after`, `source` (`'blackjack'|'coude'|'casino'|'admin'|'daily'...`), `description`, `created_at`. **Log append-only** de toutes les mutations.
- **Point d'entrée unique (use case)** : `ManageWalletUseCase` — `sentinel-core/src/ports/inbound/casino/manage_wallet.rs:52`. Méthodes clés :
  - `credit(guild_id, user_id, amount, source, description)` → `WalletMutation` (`:56`)
  - `debit(...)` → erreur de validation si solde insuffisant, détecte la faillite (`:68`)
  - `transfer(guild_id, from_user, to_user, amount, source, description)` → `Vec<TauntEvent>`, atomique (`:80`)
  - `get_balance(...)` (`:91`), plus variantes `credit_tx/debit_tx` opérant **dans une transaction en cours** sans commit (`:108`, `:120`) — indispensables pour composer une mutation trésorerie atomique.
- **Entité domaine** : `Wallet`, `WalletTransaction` — `sentinel-core/src/domain/entities/casino/wallet.rs`.
- **Migration d'unification** : `080` copie les coins historiques depuis `coude_players` vers `user_wallets` (`080:36`). Depuis, les repos `coude` (economy/player/bet) et `casino` mutent `user_wallets` — plusieurs call sites migrent progressivement derrière `ManageWalletUseCase` (cf. note « Statut de migration » `manage_wallet.rs:11-15`).

**Comment un joueur gagne / dépense / transfère (aujourd'hui) :**
- Gagne : combats coude, heists, casino win, daily, redistribution cashbox → `credit(...)`.
- Dépense : shop, assurances, mises casino/paris → `debit(...)`.
- Transfère : don entre joueurs (`ManageCoudeEconomyUseCase::gift_coins`, `manage_economy.rs:58`, avec taxe reversée en cashbox), vol (`steal`, `:75`), payout combat → `transfer(...)`.

> **Il n'y a donc pas plusieurs monnaies.** Il y a **une** monnaie (`user_wallets.coins`) et **des sous-systèmes** qui la manipulent (coude, casino, tamagotchi partagent le même solde). Le seul « autre pot » est la cagnotte communautaire ci-dessous.

### 1.2 Le modèle de référence à répliquer : la cagnotte `coude_cashbox`

C'est la référence directe pour une trésorerie d'org.

- **Migration** : `sentinel-api/migrations/124_coude_cashbox.sql`.
- **Tables** :
  - `coude_cashbox` (`124:26`) : `guild_id TEXT PRIMARY KEY`, `balance BIGINT`, `total_collected BIGINT`, `total_redistributed BIGINT`, `last_redistribution_at`, `created_at/updated_at`, `CHECK (balance >= 0)` (`124:34`). **Une caisse par guild.**
  - `coude_cashbox_redistributions` (`124:38`) + `coude_cashbox_redistribution_entries` (`124:50`) : historique des redistributions et des gains individuels.
- **Entité domaine** : `Cashbox`, `CashboxRedistribution`, `CashboxRedistributionEntry`, enum `CashboxSource` (label d'audit non persisté) — `sentinel-core/src/domain/entities/coude/cashbox.rs`.
- **Port outbound** : `CashboxRepository` — `sentinel-core/src/ports/outbound/coude/cashbox_repository.rs:13`. Méthodes :
  - `get_or_create(guild_id)` (`:15`)
  - `deposit(guild_id, amount, source: CashboxSource)` — atomique, upsert + incrémente `total_collected` (`:19`)
  - `withdraw(guild_id, amount)` — **clampé au solde, jamais négatif**, retourne le montant réel retiré (`:35`)
  - `claim_all_for_redistribution`, `record_redistribution`, `list_redistributions`, `list_entries` (`:28-56`).
- **Use case / service** : `manage_cashbox` (`ports/inbound/coude/manage_cashbox.rs`) implémenté par `application/coude/manage_cashbox_service.rs`.

**Fonctionnement (dépôt / redistribution)** : chaque coin « perdu » du circuit coude (shop, assurance, taxe de don 10 %, pénalité lâcheté, commission paris — cf. `CashboxSource` + `124:14-22`) est **déposé** dans la caisse au lieu d'être détruit. Un worker hebdomadaire vide la caisse (`claim_all_for_redistribution`) et **redistribue** aléatoirement aux joueurs actifs des 7 derniers jours, en loggant une redistribution + une entry par gagnant.

> La trésorerie d'org reprend **exactement** ce squelette (table solde + `CHECK (balance >= 0)` + tables d'historique + repo `deposit/withdraw`), en le passant de « 1 par guild » à « 1 par organisation », et en **couplant chaque mouvement au wallet** du membre concerné.

### 1.3 Helper de validation

`validate_positive(amount, label)` — `sentinel-core/src/application/validation.rs:23` : renvoie `DomainError::ValidationError` si `amount <= 0`. À réutiliser sur tous les montants de dépôt/retrait/paie. (Voisins utiles : `validate_guild_id`, `validate_non_empty`, `validate_range`.)

### 1.4 État des migrations

Plus haut numéro actuel : **`327_game_portal_sessions.sql`**. `ARCHITECTURE.md §3` réserve **`328`** pour la migration de départ Influence (citoyens, orgs, membres, votes — Phase 1). La trésorerie relève de **Phase 2** (voir §6), donc :

> **Prochain numéro proposé pour la trésorerie d'org : `329_influence_org_treasury.sql`** (juste après la base Influence `328`). Si l'ordre des phases décale la numérotation, prendre le prochain entier libre après la base Influence.

### 1.5 Recommandation monnaie — **partagée, pas dédiée**

| Critère | Monnaie partagée (`user_wallets`) | Monnaie dédiée Influence |
|---|---|---|
| Cohérence GDD (`04.md §2` : Argent → publicité → réputation) | ✅ un joueur riche au casino/coude peut investir en politique | ❌ silo, il faut tout re-gagner |
| Réutilisation code | ✅ `ManageWalletUseCase`, `wallet_transactions`, faillite/jackpot | ❌ tout réécrire |
| Simplicité pour le joueur | ✅ un seul solde `/wallet` | ❌ deux portefeuilles à gérer |
| Risque d'inflation croisée | ⚠️ à surveiller (mais coûts de création d'org + campagnes = puits) | — |

**Décision : réutiliser `user_wallets` comme monnaie du capital « Argent » du citoyen.** La colonne `influence_citizens.money` (prévue dans `ARCHITECTURE.md §3`) fait **doublon** avec le wallet partagé : on la **retire** de la table Influence et `ViewProfileUseCase` lit l'Argent via `ManageWalletUseCase::get_balance`. Les 4 autres capitaux (influence, reputation, information, network) restent propres à Influence. La **trésorerie d'org** est un pot séparé, mais **libellé dans la même monnaie** : y déposer = débiter le wallet du membre ; en retirer/payer = créditer le wallet du bénéficiaire.

---

## 2. Modèle de données — trésorerie d'organisation

Migration proposée : **`329_influence_org_treasury.sql`** (idempotente, motif `124` + `327`).

### 2.1 Solde de trésorerie

`ARCHITECTURE.md §3` prévoit déjà `influence_organizations.treasury BIGINT`. On garde ce solde **dénormalisé sur la ligne org** (lecture rapide) et on ajoute la table de mouvements pour l'historique et l'audit, exactement comme `coude_cashbox` porte `balance` + tables de redistribution.

```sql
-- 329_influence_org_treasury.sql

-- Le solde vit sur influence_organizations.treasury (déjà prévu, migration 328).
-- Garde-fou : jamais négatif.
ALTER TABLE influence_organizations
    ADD COLUMN IF NOT EXISTS treasury BIGINT NOT NULL DEFAULT 0;
DO $$ BEGIN
  ALTER TABLE influence_organizations
    ADD CONSTRAINT influence_org_treasury_non_negative CHECK (treasury >= 0);
EXCEPTION WHEN duplicate_object THEN NULL; END $$;
```

### 2.2 Mouvements de trésorerie (historique immuable, append-only)

```sql
CREATE TABLE IF NOT EXISTS influence_org_treasury_movements (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id        TEXT NOT NULL,
    org_id          UUID NOT NULL REFERENCES influence_organizations(id) ON DELETE CASCADE,
    -- Sens & nature du mouvement
    kind            TEXT NOT NULL,      -- 'deposit' | 'withdrawal' | 'salary' | 'campaign' | 'creation_endowment' | 'adjustment'
    amount          BIGINT NOT NULL,    -- toujours > 0 (le signe est porté par `kind`)
    treasury_after  BIGINT NOT NULL,    -- solde de la trésorerie après le mouvement (audit)
    -- Acteurs
    actor_user_id   TEXT NOT NULL,      -- qui a déclenché l'action (dirigeant ou membre)
    counterparty_user_id TEXT,          -- membre crédité/débité côté wallet (don: = actor ; paie: le payé)
    memo            TEXT NOT NULL DEFAULT '',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
    -- Pas d'UPDATE/DELETE applicatif : table append-only (cf. 05.md §10 archives).
);

CREATE INDEX IF NOT EXISTS idx_influence_treasury_mov_org
    ON influence_org_treasury_movements(org_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_influence_treasury_mov_actor
    ON influence_org_treasury_movements(guild_id, actor_user_id, created_at DESC);
```

> **Immuabilité** : comme `wallet_transactions` et `influence_archives` (`ARCHITECTURE.md §3`, `05.md §10/§14`), la table n'est jamais mise à jour ni purgée. Optionnellement, un trigger `BEFORE UPDATE OR DELETE ... RAISE EXCEPTION` peut la verrouiller au niveau SQL.

### 2.3 Rôles hiérarchiques (permissions)

La table `influence_org_members.role` (`ARCHITECTURE.md §3`, `05.md §5`) porte le rôle : **`Fondateur | Dirigeant | Responsable | Membre | Recrue`**. On dérive les droits trésorerie d'un rang numérique (fonction pure domaine, voir §3.2). Pas de nouvelle table.

### 2.4 Config web (fin de migration, motif `327:41`)

```sql
UPDATE bot_definitions SET config_schema = config_schema || '[
  {"key":"influence_treasury_min_deposit","label":"Don minimum a la tresorerie","type":"number","default":"1"},
  {"key":"influence_treasury_withdraw_min_role","label":"Role min. pour retirer/payer (0=Recrue..4=Fondateur)","type":"number","default":"3"},
  {"key":"influence_org_creation_cost","label":"Cout de creation d une organisation","type":"number","default":"1000"}
]'::jsonb
WHERE bot_name = 'influence-bot'
  AND NOT (config_schema @> '[{"key":"influence_treasury_min_deposit"}]'::jsonb);
```

---

## 3. Couche hexagonale (calquée sur cashbox + wallet)

### 3.1 Domaine — entités (`sentinel-core/src/domain/entities/influence/`)

```rust
// treasury.rs  (cf. entities/coude/cashbox.rs)
pub struct OrgTreasury {
    pub org_id: Uuid,
    pub guild_id: GuildId,
    pub balance: i64,          // = influence_organizations.treasury
    pub updated_at: DateTime<Utc>,
}

pub enum TreasuryMovementKind {
    Deposit,            // un membre reverse ses coins -> caisse
    Withdrawal,         // un dirigeant sort des coins vers son wallet
    Salary,             // un dirigeant paie un membre
    Campaign,           // dépense projet/campagne (06.md)
    CreationEndowment,  // dotation initiale à la création de l'org
    Adjustment,         // correction admin
}
impl TreasuryMovementKind { pub fn as_str(&self) -> &'static str { /* ... */ } }

pub struct TreasuryMovement {
    pub id: Uuid,
    pub org_id: Uuid,
    pub kind: TreasuryMovementKind,
    pub amount: i64,               // > 0
    pub treasury_after: i64,
    pub actor_user_id: UserId,
    pub counterparty_user_id: Option<UserId>,
    pub memo: String,
    pub created_at: DateTime<Utc>,
}
```

### 3.2 Domaine — permissions (fonction pure, cf. `tier.rs` / `tout_ou_rien.rs`)

```rust
// org_membership.rs — rang hiérarchique + droits, FONCTION PURE testable
pub enum OrgRole { Recrue, Membre, Responsable, Dirigeant, Fondateur }
impl OrgRole {
    pub fn rank(&self) -> u8 { /* Recrue=0 .. Fondateur=4 */ }
}

/// Tout membre (même Recrue) peut déposer.
pub fn can_deposit(_role: OrgRole) -> bool { true }

/// Retirer / payer : rang >= seuil configuré (défaut Dirigeant=3).
pub fn can_spend(role: OrgRole, min_role_rank: u8) -> bool {
    role.rank() >= min_role_rank
}
```

### 3.3 Ports outbound — repository (`ports/outbound/influence/treasury_repository.rs`)

Copie conforme de `CashboxRepository`, mais **par org** et **couplée au wallet** via des variantes `_tx` (atomicité caisse+wallet).

```rust
#[async_trait]
pub trait OrgTreasuryRepository: Send + Sync {
    async fn get(&self, guild_id: &str, org_id: Uuid) -> Result<OrgTreasury, DomainError>;

    /// Crédite la caisse de `amount` (upsert treasury) DANS la tx fournie
    /// et insère le mouvement. Ne commit pas. À composer avec un debit wallet.
    async fn deposit_tx(
        &self, tx: &mut dyn DbTx, guild_id: &str, org_id: Uuid,
        amount: i64, kind: TreasuryMovementKind,
        actor_user_id: &str, counterparty_user_id: Option<&str>, memo: &str,
    ) -> Result<i64 /* treasury_after */, DomainError>;

    /// Débite la caisse de `amount` DANS la tx fournie (garde CHECK>=0) et
    /// insère le mouvement. Ne commit pas. À composer avec un credit wallet.
    async fn withdraw_tx(
        &self, tx: &mut dyn DbTx, guild_id: &str, org_id: Uuid,
        amount: i64, kind: TreasuryMovementKind,
        actor_user_id: &str, counterparty_user_id: Option<&str>, memo: &str,
    ) -> Result<i64, DomainError>;

    async fn list_movements(
        &self, guild_id: &str, org_id: Uuid, limit: i64,
    ) -> Result<Vec<TreasuryMovement>, DomainError>;
}
```

> **Atomicité** : chaque use case ouvre une `DbTx` (`ports/uow`), appelle `ManageWalletUseCase::debit_tx/credit_tx` (`manage_wallet.rs:108/120`) **et** `OrgTreasuryRepository::deposit_tx/withdraw_tx` sur la **même** tx, puis `commit`. Le solde ne peut jamais diverger entre wallet et caisse.

### 3.4 Ports inbound — use cases (`ports/inbound/influence/manage_treasury.rs`)

```rust
#[async_trait]
pub trait ManageOrgTreasuryUseCase: Send + Sync {
    /// Un membre reverse `amount` de SON wallet vers la caisse de son org.
    async fn deposit_to_org(&self, guild_id: &str, org_id: Uuid,
        member_user_id: &str, amount: i64, memo: &str) -> Result<OrgTreasury, DomainError>;

    /// Un dirigeant sort `amount` de la caisse vers SON wallet.
    async fn withdraw_from_org(&self, guild_id: &str, org_id: Uuid,
        actor_user_id: &str, amount: i64, memo: &str) -> Result<OrgTreasury, DomainError>;

    /// Un dirigeant paie `amount` de la caisse vers le wallet d'un membre.
    async fn pay_member(&self, guild_id: &str, org_id: Uuid,
        actor_user_id: &str, target_user_id: &str, amount: i64, memo: &str)
        -> Result<OrgTreasury, DomainError>;

    /// Lecture : solde + N derniers mouvements (respecte 05.md §9).
    async fn view_treasury(&self, guild_id: &str, org_id: Uuid, limit: i64)
        -> Result<(OrgTreasury, Vec<TreasuryMovement>), DomainError>;
}
```

### 3.5 Service application (`application/influence/treasury_service.rs`)

Structure identique à `play_tout_ou_rien_service.rs:28-59` : `struct OrgTreasuryService { treasury_repo, membership_repo, wallet_uc: Arc<dyn ManageWalletUseCase>, uow, cfg_repo: Option<Arc<dyn BotConfigRepository>> }` + `new(...)` + `with_bot_config_repo(...)`. Tests co-localisés `#[cfg(test)] #[path="tests/treasury.rs"] mod tests;`.

Squelette de `deposit_to_org` :
```rust
validate_guild_id(guild_id)?;
validate_positive(amount, "montant")?;                 // validation.rs:23
let role = membership_repo.role_of(guild_id, org_id, member_user_id).await?; // Forbidden si non-membre
// can_deposit(role) == true pour tout membre
let mut tx = uow.begin().await?;
wallet_uc.debit_tx(&mut tx, guild_id, member_user_id, amount, "influence_org", "don tresorerie").await?; // solde insuffisant -> ValidationError
let after = treasury_repo.deposit_tx(&mut tx, guild_id, org_id, amount,
                 Deposit, member_user_id, Some(member_user_id), memo).await?;
tx.commit().await?;
// archive best-effort (ARCHITECTURE.md §6.2, motif play_tout_ou_rien_service.rs:183)
```
`withdraw_from_org` / `pay_member` : vérifier `can_spend(role, cfg.withdraw_min_role_rank)` → sinon `DomainError::Forbidden` ; puis `withdraw_tx` (caisse, `CHECK>=0` protège) + `credit_tx` (wallet du bénéficiaire) dans la même tx.

---

## 4. Commandes bot (`sentinel-bot/src/modules/influence/commands/`)

Ajout au groupe `/org` (Phase 2). Dispatch à câbler dans `handler.rs:46/531` et `command_registry.rs:41/68` (cf. `ARCHITECTURE.md §7`).

| Commande | Qui | Effet | Use case |
|---|---|---|---|
| `/org tresor` | tout membre | affiche solde + derniers mouvements | `view_treasury` |
| `/org don <montant>` | tout membre | reverse ses coins → caisse | `deposit_to_org` |
| `/org retrait <montant>` | dirigeant (rang ≥ seuil) | caisse → son wallet | `withdraw_from_org` |
| `/org paye <membre> <montant>` | dirigeant | caisse → wallet du membre (salaire) | `pay_member` |

Le bot appelle `sentinel-api` via `modules/influence/api_client/` (cf. `modules/coude/api_client/`), jamais la base directement.

---

## 5. Règles clés & garde-fous

1. **Qui dépose** : tout membre de l'org (`can_deposit` = true), y compris Recrue. Non-membre → `Forbidden`.
2. **Qui retire / paie** : rang ≥ `influence_treasury_withdraw_min_role` (défaut **Dirigeant=3**), réglable en web. Sinon `Forbidden`.
3. **Le patrimoine appartient à l'org, pas au dirigeant** (`05.md §9`) : un retrait est tracé (`actor_user_id`, `kind=Withdrawal`) et reste auditable ; à la dissolution, la trésorerie est redistribuée/archivée (Phase 5), jamais empochée en douce.
4. **Montants positifs** : `validate_positive` (`validation.rs:23`) sur don/retrait/paie.
5. **Solde suffisant** : côté wallet, `debit_tx` échoue si insuffisant ; côté caisse, `CHECK (treasury >= 0)` + `withdraw_tx` refuse le découvert.
6. **Atomicité** : wallet ↔ caisse mutés dans **une même transaction** (`credit_tx/debit_tx` + `deposit_tx/withdraw_tx`) — pas de coins créés ni perdus.
7. **Historique immuable** : `influence_org_treasury_movements` est append-only (comme `wallet_transactions` / `influence_archives`). Chaque mouvement écrit aussi une entrée `influence_archives` (best-effort, ne fait pas échouer la commande — motif `play_tout_ou_rien_service.rs:183-196`).
8. **Monnaie** : identique aux coins joueurs (`user_wallets`) — pas de monnaie dédiée. La trésorerie est un coffre commun, pas une seconde devise.
9. **Multi-serveur** : `guild_id` sur la table de mouvements et dans toutes les signatures.

---

## 6. Intégration dans le découpage en phases (`ARCHITECTURE.md §4`)

- **Phase 1 (MVP)** — la table `influence_organizations` (avec `treasury BIGINT DEFAULT 0`) est créée (migration `328`). La création d'org **débite** déjà le fondateur de `influence_org_creation_cost` via `ManageWalletUseCase::debit` (réutilise l'existant, aucune trésorerie active nécessaire). Optionnel : verser une **dotation initiale** en caisse (`kind=CreationEndowment`).
- **Phase 2 (Réputation & Capitaux + conversions)** — **c'est ici qu'on branche la trésorerie complète.** Cohérent avec `ARCHITECTURE.md §4 Phase 2` (« conversions de capitaux, transactions ») et le fait que l'Argent est justement l'un des capitaux convertibles (`04.md §10`). Livrables Phase 2 :
  - migration **`329_influence_org_treasury.sql`** (mouvements + CHECK + config web) ;
  - domaine `treasury.rs` + permissions `can_deposit/can_spend` ;
  - port `OrgTreasuryRepository` + impl `PgOrgTreasuryRepository` ;
  - use case `ManageOrgTreasuryUseCase` + `OrgTreasuryService` (câblés `app_state.rs` / `http/state.rs`, cf. `ARCHITECTURE.md §2.6`) ;
  - commandes `/org tresor|don|retrait|paye`.
- **Phase 3+** — la trésorerie devient la **source de financement** : salaires récurrents, financement de campagnes électorales (`06.md`), sponsoring (`05.md §3 Entreprises`). Ajouter les `TreasuryMovementKind::Salary/Campaign` aux dépenses correspondantes.
- **Phase 5 (Monde vivant & Archives)** — à la **dissolution** d'une org (`05.md §14`), vider la trésorerie (redistribution aux membres ou versement à une caisse serveur) et archiver le solde final ; les mouvements restent consultables.

---

## 7. Fichiers de référence (récap)

| Sujet | Fichier:ligne |
|---|---|
| Monnaie unifiée (table + tx log) | `sentinel-api/migrations/080_create_user_wallets.sql:5,22` |
| Use case wallet unifié + `_tx` | `sentinel-core/src/ports/inbound/casino/manage_wallet.rs:52,80,108,120` |
| Entité Wallet | `sentinel-core/src/domain/entities/casino/wallet.rs` |
| Cagnotte de référence (table) | `sentinel-api/migrations/124_coude_cashbox.sql:26` |
| Cagnotte (entité) | `sentinel-core/src/domain/entities/coude/cashbox.rs` |
| Cagnotte (port deposit/withdraw) | `sentinel-core/src/ports/outbound/coude/cashbox_repository.rs:13,19,35` |
| Don joueur taxé + steal | `sentinel-core/src/ports/inbound/coude/manage_economy.rs:58,75` |
| Validation montant positif | `sentinel-core/src/application/validation.rs:23` |
| Pattern service + archive best-effort | `sentinel-core/src/application/coude/play_tout_ou_rien_service.rs:28-59,183-196` |
| Plan Influence / phases / migration 328 | `docs/Nouveau jeux/ARCHITECTURE.md §3,§4` |
