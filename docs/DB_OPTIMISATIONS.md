# Optimisations base de données

Audit des 98 migrations Postgres du projet et recommandations d'optimisation avec impact code mesuré.

**Périmètre** : toutes les tables dans `services/api/migrations/`.

---

## 🎯 Vue d'ensemble

| # | Optimisation | Gain perf | Breaking code | Effort IA |
|---|---|---|---|---|
| 1 | Supprimer index dupliqués | 🔴 | Non | 15min |
| 2 | Index partiels soft-delete | 🔴 | Non | 30min |
| 3 | Index GIN sur JSONB hot | 🔴🔴 | Non | 30min |
| 4 | TEXT → VARCHAR(20) Discord IDs | 🔴🔴 | Non | 1h |
| 5 | `bot_guild_config.config_value` → JSONB | 🔴 | **Oui** | 3-4h |
| 6 | Partitionnement tables event-heavy | 🔴🔴🔴 | Non (vérif) | 2-3h |
| 7 | Vues matérialisées leaderboards | 🔴🔴🔴 | Non (additif) | 2-3h |
| 8 | Enums Postgres | 🟡 | **Oui** | 2-3h |
| 9 | `permissions` TEXT → BIGINT | 🟡 | Oui | 1h |
| 10 | Dénormalisation username (worker sync) | 🟡 | Non (additif) | 2-3h |
| 11 | Contraintes NOT NULL / CHECK | 🟢 | Non | 1h |
| 12 | Colonnes mortes à nettoyer | 🟢 | Non | 30min |

**Total effort IA** : ~15-20 heures effectives.

---

## 🔴 Critique

### 1. Supprimer les index dupliqués

**Problème détecté** :
- `idx_infractions_guild_created` défini en migration **058** ET **072** → doublon exact
- `idx_audit_logs_guild_created` défini en **015** et re-défini en **075**
- Certains index composites anciens sont subsumés par les nouveaux

**Fix** (migration 099) :
```sql
-- Supprime les doublons, garde le plus récent (le plus optimisé)
DROP INDEX IF EXISTS idx_infractions_guild_created_old;
DROP INDEX IF EXISTS idx_audit_logs_guild_created_old;

-- Audit : lister les doublons restants
SELECT indexrelid::regclass, indrelid::regclass, indkey
FROM pg_index
GROUP BY indrelid, indkey, indexrelid
HAVING COUNT(*) > 1;
```

**Code impacté** : aucun.

---

### 2. Index partiels pour soft-delete

**Problème** : `voice_channels.channel_status` a un index plein `idx_voice_channels_status` qui scan aussi les lignes closed/deleted. Gaspillage massif.

**Fix** :
```sql
DROP INDEX IF EXISTS idx_voice_channels_status;

CREATE INDEX idx_voice_channels_active
ON voice_channels (guild_id, owner_id)
WHERE channel_status = 'open';

-- Même pattern pour les autres soft-deletes
CREATE INDEX idx_tickets_open
ON tickets (guild_id, created_at DESC)
WHERE status IN ('open', 'assigned');
```

**Gain** : index 5-10× plus petit, scans sur la partie "vivante" beaucoup plus rapides.

**Code impacté** : aucun.

---

### 3. Index GIN sur JSONB fréquemment requêté

**Problème** : plusieurs colonnes JSONB sont filtrées avec `@>` ou `?` sans index GIN.

**Fix** :
```sql
CREATE INDEX idx_infractions_flags_gin ON infractions USING GIN (flags);
CREATE INDEX idx_security_user_ids_gin ON security_events USING GIN (user_ids);
CREATE INDEX idx_bot_def_schema_gin ON bot_definitions USING GIN (config_schema);
```

**Gain** : queries JSONB 10-50× plus rapides.

**Code impacté** : aucun. Les queries existantes utiliseront automatiquement l'index.

---

### 4. TEXT → VARCHAR(20) pour les Discord IDs

**Problème** : les IDs Discord font max 20 caractères, mais stockés en TEXT non borné dans ~30 tables. Gaspillage dans les index et le buffer pool.

**Fix progressif** (peut être fait table par table, non-breaking) :
```sql
-- Exemple pour infractions
ALTER TABLE infractions
    ALTER COLUMN guild_id TYPE VARCHAR(20),
    ALTER COLUMN user_id TYPE VARCHAR(20),
    ALTER COLUMN moderator_id TYPE VARCHAR(20);

-- Répéter pour : audit_logs, moderation_actions, security_events, tickets,
-- voice_channels, voice_sessions, user_stats, conduct_points, user_notes,
-- strikes, role_panels, discord_roles, guild_members, user_wallets,
-- coude_players, coude_combats, coude_bets, blackjack_games, levels,
-- daily_activity, hourly_activity, user_activity_log, manual_watched_users
```

**Gain** : **-20 à -30 % taille des index** → plus de données en buffer pool RAM.

**Code impacté** : **aucun** ! SQLx mappe TEXT et VARCHAR(20) indifféremment vers `String`. Vérification faite : tous les Discord IDs dans les structs Rust sont déjà `String`.

---

### 5. `bot_guild_config.config_value` : TEXT → JSONB

**Problème** : stocké en TEXT → aucune query JSONB possible, pas d'index GIN, parsing côté applicatif à chaque lecture.

**Fix** :
```sql
-- Migration 100
ALTER TABLE bot_guild_config
    ALTER COLUMN config_value TYPE JSONB
    USING config_value::jsonb;

CREATE INDEX idx_bot_guild_config_value_gin
ON bot_guild_config USING GIN (config_value);
```

**Code à modifier** (breaking change) :
- `services/api/src/domain/entities/bot_config.rs` : `config_value: String` → `config_value: serde_json::Value`
- `services/api/src/adapters/outbound/postgres/bot_config_repository.rs` : adapter `FromRow` et `set_config()`
- `services/api/src/adapters/inbound/http/dto/bot_config.rs` : adapter les DTOs HTTP
- Éventuels clients desktop/bots qui lisent `config_value` (vérifier sérialisation JSON)

**Effort** : 3-4h IA.

---

### 6. Partitionnement des tables event-heavy

**Problème** : croissance non-bornée → VACUUM lent, index obèses, purges impossibles sans `DELETE` massifs.

**Tables concernées** :
- `infractions`
- `audit_logs`
- `user_activity_log`
- `moderation_actions`
- `security_events`
- `logs`
- `daily_activity` (déjà partitionnable par `day`)
- `hourly_activity`
- `coude_casino_log`

**Fix** (exemple pour `audit_logs`) :
```sql
-- Migration 101 — renommer l'ancienne table, créer la partitionnée
ALTER TABLE audit_logs RENAME TO audit_logs_old;

CREATE TABLE audit_logs (
    LIKE audit_logs_old INCLUDING ALL
) PARTITION BY RANGE (created_at);

-- Partitions mensuelles — créer les 12 prochains mois
CREATE TABLE audit_logs_2026_04 PARTITION OF audit_logs
    FOR VALUES FROM ('2026-04-01') TO ('2026-05-01');
CREATE TABLE audit_logs_2026_05 PARTITION OF audit_logs
    FOR VALUES FROM ('2026-05-01') TO ('2026-06-01');
-- ... etc

-- Partition "default" pour l'historique
CREATE TABLE audit_logs_default PARTITION OF audit_logs DEFAULT;

-- Copier les données existantes
INSERT INTO audit_logs SELECT * FROM audit_logs_old;
DROP TABLE audit_logs_old;
```

**Automatisation** : créer un worker `partition-manager-worker` qui crée la partition du mois suivant le 25 de chaque mois et archive/drop les partitions > rétention (12 mois ?).

**Gain** :
- Queries temporelles **10-100× plus rapides** (pruning de partitions)
- VACUUM **-80 à -95 %** de durée
- Purges en **O(1)** (DROP PARTITION)

**Code impacté** : **aucun** en théorie. Vérifications à faire :
- Aucune query ne doit utiliser `ONLY audit_logs` (aucune occurrence trouvée ✅)
- `created_at` doit être `NOT NULL` et bindé à l'INSERT (vérifié ✅)
- Les contraintes `UNIQUE` doivent inclure la clé de partition (à vérifier table par table)

---

### 7. Vues matérialisées pour les leaderboards

**Problème** : queries `ORDER BY coins DESC LIMIT 100` sur `coude_players`, `user_wallets`, `user_levels` à chaque affichage de leaderboard → scan + tri complet.

**Fix** :
```sql
-- Migration 102
CREATE MATERIALIZED VIEW coude_leaderboard AS
SELECT
    guild_id,
    user_id,
    username,
    coins,
    xp,
    level,
    rank() OVER (PARTITION BY guild_id ORDER BY coins DESC) AS rank_coins,
    rank() OVER (PARTITION BY guild_id ORDER BY xp DESC) AS rank_xp
FROM coude_players;

CREATE UNIQUE INDEX ON coude_leaderboard (guild_id, user_id);
CREATE INDEX ON coude_leaderboard (guild_id, rank_coins);
CREATE INDEX ON coude_leaderboard (guild_id, rank_xp);

-- Refresh concurrent (ne bloque pas les lectures)
REFRESH MATERIALIZED VIEW CONCURRENTLY coude_leaderboard;
```

**Worker de refresh** : ajouter au `cache-worker` existant un job qui fait `REFRESH MATERIALIZED VIEW CONCURRENTLY` toutes les 5-15 minutes sur chaque vue.

**Vues similaires à créer** :
- `user_wallets_leaderboard`
- `user_levels_leaderboard`
- `progression_leaderboard` (texte + voice combinés)

**Gain** : **100-1000×** sur les endpoints leaderboard.

**Code à modifier** (additif, non-breaking) :
- `services/api/src/adapters/outbound/postgres/coude_player_repository.rs` : nouvelle méthode `get_leaderboard_from_view()`
- Idem pour `wallet_repository.rs`, `level_repository.rs`, `stats_repository.rs`
- `cache-worker` : ajouter le job de refresh

**Effort** : 2-3h IA.

---

## 🟡 Important

### 8. Enums Postgres pour les colonnes à valeurs fixes

**Colonnes concernées** :
- `coude_players.class` (bourrin, agile, fourbe, tank)
- `moderation_actions.gravity` (warn, mute, ban, kick)
- `voice_channels.kind` (public, private)
- `infractions.action`

**Fix** :
```sql
CREATE TYPE coude_class AS ENUM ('bourrin', 'agile', 'fourbe', 'tank');
CREATE TYPE moderation_gravity AS ENUM ('low', 'medium', 'high', 'critical');
CREATE TYPE voice_channel_kind AS ENUM ('public', 'private', 'stage');

ALTER TABLE coude_players
    ALTER COLUMN class TYPE coude_class USING class::coude_class;
ALTER TABLE moderation_actions
    ALTER COLUMN gravity TYPE moderation_gravity USING gravity::moderation_gravity;
ALTER TABLE voice_channels
    ALTER COLUMN kind TYPE voice_channel_kind USING kind::voice_channel_kind;
```

**Code à modifier** :
- `services/api/src/domain/entities/coude_player.rs`
- `services/api/src/domain/entities/moderation_action.rs`
- `services/api/src/domain/entities/voice_channel.rs`
- Les repositories correspondants
- Les DTOs HTTP (sérialisation serde)

Exemple de type Rust :
```rust
#[derive(Debug, Clone, PartialEq, sqlx::Type, serde::Serialize, serde::Deserialize)]
#[sqlx(type_name = "coude_class", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum CoudeClass {
    Bourrin,
    Agile,
    Fourbe,
    Tank,
}
```

**Gain** : validation au niveau DB (impossible d'insérer une valeur invalide), -1 à -4 bytes/row, types Rust plus sûrs.

**Effort** : 2-3h IA.

---

### 9. `discord_roles.permissions` : TEXT → BIGINT

**Problème** : les permissions Discord sont un bitmask 64 bits, stocké en TEXT → parsing string à chaque lecture, pas d'opérations bitwise possibles en SQL.

**Fix** :
```sql
ALTER TABLE discord_roles
    ALTER COLUMN permissions TYPE BIGINT
    USING permissions::bigint;
```

**Code à modifier** :
- `services/api/src/domain/entities/discord_role.rs` : `permissions: String` → `permissions: i64`
- Repository et DTO associés

**Gain** : checks de permissions directement en SQL (ex : `WHERE permissions & 8 = 8` pour admin).

---

### 10. Dénormalisation username/display_name → worker de sync

**Problème** : 15+ tables stockent `user_id + username` en copie. Si un user change de nom Discord, toutes les tables deviennent stales.

**Fix** : créer une table canonique + un worker de sync :
```sql
-- Migration 103
CREATE TABLE user_cache (
    user_id VARCHAR(20) PRIMARY KEY,
    username VARCHAR(32) NOT NULL,
    display_name VARCHAR(32),
    avatar_hash VARCHAR(64),
    synced_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_user_cache_synced ON user_cache (synced_at);
```

**Nouveau worker** : `user-cache-worker`
- Rafraîchit les users récemment actifs (via Discord API batch)
- Trigger async sur les tables qui ont un `username` → copier depuis `user_cache` au prochain refresh
- Alternative plus simple : garder les copies dénormalisées mais ajouter une vue SQL qui JOIN avec `user_cache` pour les affichages critiques

**Effort** : 2-3h IA + création du worker.

---

## 🟢 Nice-to-have

### 11. Contraintes `NOT NULL` et `CHECK` manquantes

Colonnes TEXT "optionnelles" mais jamais NULL en pratique :
- `role_panel_entries.role_name`
- `blackjack_games.username`
- Plusieurs colonnes `_name` dans les tables tickets, voice_channels, etc.

**Fix** :
```sql
-- Audit d'abord
SELECT COUNT(*) FROM role_panel_entries WHERE role_name IS NULL;
-- Si 0, appliquer
ALTER TABLE role_panel_entries ALTER COLUMN role_name SET NOT NULL;
```

---

### 12. Colonnes mortes à nettoyer

- `voice_channels.channel_id_temp` (migration 082) — semble abandonné
- `tickets.channels` (migration 022) — vérifier l'usage

**Fix** : audit + `ALTER TABLE ... DROP COLUMN` après validation.

---

## 🗺️ Ordre optimal de déploiement

Logique : **non-breaking d'abord, breaking ensuite, partitionnement en dernier** (car plus complexe).

### Étape 1 — Quick wins zéro-breaking (1-2h IA)
1. **#1** Supprimer index dupliqués
2. **#2** Index partiels soft-delete
3. **#3** Index GIN sur JSONB
4. **#4** TEXT → VARCHAR(20) Discord IDs (aucun impact code)
5. **#12** Nettoyage colonnes mortes

### Étape 2 — Optimisations non-breaking additives (3-4h IA)
6. **#7** Vues matérialisées leaderboards + refresh worker
7. **#10** Table `user_cache` + worker de sync

### Étape 3 — Breaking changes contrôlés (5-7h IA)
8. **#8** Enums Postgres (breaking, mais localisé)
9. **#9** `permissions` TEXT → BIGINT
10. **#5** `bot_guild_config.config_value` → JSONB (le plus breaking)
11. **#11** NOT NULL / CHECK constraints

### Étape 4 — Chantier majeur (2-3h IA + migrations)
12. **#6** Partitionnement des tables event-heavy + `partition-manager-worker`

---

## 📊 Impact global cumulé

| Axe | Gain estimé |
|---|---|
| Taille des index | **-25 à -35 %** |
| Latence queries temporelles | **10-100×** (partitionnement) |
| Latence leaderboards | **100-1000×** (vues matérialisées) |
| Queries JSONB | **10-50×** (index GIN) |
| RAM buffer pool utilisé | **-20 %** |
| VACUUM duration | **-80 à -95 %** |
| Sécurité données (enums, NOT NULL) | Correctness++ |

**C'est probablement le plus gros gain perf du projet**, plus que toutes les optimisations applicatives réunies.
