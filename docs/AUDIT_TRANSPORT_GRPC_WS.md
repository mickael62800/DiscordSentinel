# Audit transport — candidats gRPC & WebSocket

> Où le projet gagnerait à passer du HTTP/polling vers **gRPC** (bot ↔ API interne) ou **WebSocket** (push temps réel vers le web).
> Constat de départ : gRPC et WS sont **déjà largement adoptés**. L'objectif n'est donc pas d'en ajouter partout, mais de combler des **incohérences** et de remplacer du **polling** par du push événementiel.

---

## État des lieux

**gRPC déjà en place** (`sentinel-proto/proto/`) : `ai_dataset`, `automod`, `blackjack`, `community`, `coude` (player/combats/bets/economy/inventory/social), `export`, `images`, `members`, `moderation`, `progression`, `roles`, `security`, `stats`, `tamagotchi`, `tickets`, `voice`, `welcome`.

**WebSocket déjà en place** : `sentinel-gateway` relaie la stream Redis `sentinel:events` vers les clients ; côté web, `stores/realtimeStore.ts` consomme ces événements.

> **Mise à jour** — Toutes les priorités de cet audit sont traitées. Détail par section ci-dessous, synthèse dans [Ce qu'il reste à faire](#ce-quil-reste-à-faire).

---

## 🔌 Candidats gRPC (bot ↔ API)

### 1. Tamagotchi — incohérence majeure (haute) — ✅ fait

> **Migré.** `tamagotchi.proto` (`TamagotchiService`) est en place ; le bot (`tamagotchi/api_client.rs`) ne fait **plus aucun appel HTTP** (CRUD pets, `Care`/`Train`/`Combat`, `Tick`, `ListCards` tous en gRPC). `ListCards` est désormais en **server-streaming** (`rpc ListCards(ListCardsRequest) returns (stream Card)`) : l'API pagine la lecture DB en interne et streame les cartes une à une, le bot consomme le stream en une passe (plus de pagination par curseur côté client).

C'était le **seul** domaine de jeu **sans service gRPC**. Le bot parlait à l'API entièrement en **HTTP** :

| Fichier | Appels |
|---|---|
| `sentinel-bot/src/modules/tamagotchi/panel.rs` | `post_json`/`get_json` (`/api/tamagotchi/pets`, `/care`, `/train`, `/visit`, `/combat`, `/{guild}/{owner}`, `/{guild}/{owner}/card`) |
| `sentinel-bot/src/modules/tamagotchi/refresh.rs` | `get_json` (`/api/tamagotchi/cards`, paginé) |
| `sentinel-bot/src/modules/tamagotchi/lifecycle_events.rs` | `get_json` (`/api/tamagotchi/{guild}/{owner}`) |

Tous les modules comparables (voice, coude, blackjack, progression, stats…) sont en gRPC.

**Reco** : créer un `TamagotchiService` (proto) avec CRUD pets, `Care`/`Train`/`Combat`, `Tick`, `SetCardLocation`, `ListCards`. Le rafraîchissement horaire des cartes (aujourd'hui `GET /cards` paginé) gagnerait à utiliser un **server-streaming** gRPC (`rpc ListCards(...) returns (stream CardItem)`) plutôt qu'une pagination par curseur.

### 2. Presets / whitelist vocaux — dette introduite (moyenne) — ✅ fait

> **Migré.** `GetPreset` / `SavePreset` / `GetWhitelist` ont été ajoutés au domaine voice gRPC ; `voice/api_client.rs` les appelle via `guarded(...)` et `channel_lifecycle.rs` / `channel_management.rs` n'utilisent plus HTTP pour ces chemins. La dette est résorbée.

Les endpoints ajoutés récemment passaient par **HTTP** (`BaseApiClient`) alors que tout le reste du domaine voice est sur `VoiceChannelsService` (gRPC) :

| Fichier | Appels HTTP |
|---|---|
| `sentinel-bot/src/modules/voice/api_client.rs` | `get_preset`, `save_preset`, `get_whitelist` → `self.base.get_json`/`post_fire_and_forget` |
| `sentinel-bot/src/modules/voice/handlers/voice/channel_lifecycle.rs` | lit preset + whitelist à la création |

**Reco** : ajouter `SavePreset` / `GetPreset` / `GetWhitelist` au proto `voice.proto` existant (le service `VoiceChannelsService` a déjà `AddToWhitelist`) pour l'uniformité du domaine.

### 3. Vérifier les domaines à gRPC partiel (basse) — ✅ audité

Audit réalisé sur `welcome`, `bump`, `rotation`, `ai_dataset` :

- **welcome** : déjà migré. Le bot lit en gRPC (`WelcomeService.GetConfig` + `MembersService.GetMember` sur le hot path member-join) ; aucun chemin d'écriture welcome n'est appelé par le bot. Le seul HTTP restant est `send_log` (log générique fire-and-forget, non spécifique à welcome). **Rien à faire.**
- **ai_dataset** : ✅ **migré en gRPC** (`AiDatasetService.CollectMessage`). C'était le seul candidat défendable du lot car l'appel est fire-and-forget **sur chaque message** (chemin le plus chaud du bot). L'endpoint HTTP `POST /api/ai-dataset/collect` a été supprimé.
- **bump** : 100% HTTP. Fréquence faible (POST sur `/bump`, GET reminders toutes les 60 s, POST reminder-sent). **HTTP acceptable**, pas de migration justifiée.
- **rotation** : 100% HTTP (state machine : GET config/state/history + POST save/served). Fréquence faible (tick 10 min + commandes). **HTTP acceptable.**

---

## 📡 Candidats WebSocket (push vs polling)

7 fichiers web utilisent `setInterval`. Il faut distinguer **données événementielles** (→ push) et **jauges échantillonnées** (→ polling légitime).

### Bons candidats au push (événements) — ✅ fait

| Fichier:ligne | État | Détail |
|---|---|---|
| `components/organisms/AutomodAnalysisHistory.vue` | ✅ poussé | Plus de `setInterval` ; consomme désormais le `realtimeStore` (événement de nouvelle analyse). |
| `components/pages/GamePortalPage.vue` | ✅ poussé | S'abonne aux événements `game_server_created`/`started`/… via `realtimeStore`. Garde un `setInterval(fetchAll, 10s)` en **fallback** de resync — acceptable. |
| `components/molecules/GameServerStatsBar.vue` | polling légitime | `setInterval(fetchStats, 5s)` : jauges de serveur (joueurs/CPU) **échantillonnées** → relève de la catégorie « mesure continue » ci-dessous, pas du push. |

### Polling légitime — NE PAS forcer en WS

| Fichier | Pourquoi le polling reste le bon choix |
|---|---|
| `components/pages/ServerHealthPage.vue` | jauges CPU/mémoire échantillonnées : un push nécessiterait quand même un échantillonnage serveur. |
| `components/pages/SystemOpsPage.vue` | métriques d'ops échantillonnées. |
| `components/organisms/DockerAdminSection.vue` | statut conteneurs (état sampled). |
| `components/atoms/ConnectionBanner.vue` | `/health` toutes les 90 s = **fallback** de connectivité sain. (Voir aussi audit atomic : le fetch devrait être extrait dans un composable.) |

> Règle : pousser ce qui **change par événement discret** (analyse, sanction, ticket, session de jeu, état de salon vocal) ; continuer à interroger ce qui est une **mesure continue** (CPU, mémoire, santé).

---

## Priorités — toutes traitées

1. ✅ **Tamagotchi → gRPC** — `TamagotchiService` en place, bot 100% gRPC (`ListCards` en **server-streaming**).
2. ✅ **Presets vocaux → `voice.proto`** — `GetPreset`/`SavePreset`/`GetWhitelist` ajoutés, dette HTTP résorbée.
3. ✅ **AutomodAnalysisHistory + GamePortal → WS push** — les deux consomment `realtimeStore` (GamePortal garde un poll de resync en fallback).
4. ✅ **Veille welcome/bump/rotation/ai_dataset** — welcome déjà migré, **ai_dataset migré en gRPC**, bump/rotation laissés en HTTP (fréquence faible, acceptable).

---

## Ce qu'il reste à faire

**Aucune action obligatoire** : tous les candidats prioritaires de l'audit sont traités. Restent uniquement des points **optionnels / de veille**, à n'engager que si un besoin concret apparaît :

| Point | Nature | Quand le faire |
|---|---|---|
| `ai_dataset` → **client-streaming** (`CollectStream`) | Optimisation | Si le volume per-message devient un coût mesurable. L'unaire fire-and-forget actuel suffit. (Note : depuis le server-streaming de `ListCards`, le projet a désormais un précédent de streaming gRPC.) |
| **bump** / **rotation** → gRPC | Uniformité | Seulement par cohérence : fréquence faible (tick 10 min, poll 60 s), bénéfice marginal. Laisser en HTTP par défaut. |
| `welcome` `send_log` → uniformiser | Cosmétique | Log générique fire-and-forget, non spécifique à welcome ; à traiter avec une éventuelle migration globale du logging, pas isolément. |
| `GameServerStatsBar` / `ServerHealthPage` / `SystemOpsPage` / `DockerAdminSection` / `ConnectionBanner` | **NE PAS migrer** | Polling de **mesures continues échantillonnées** = bon choix (cf. règle ci-dessus). |
