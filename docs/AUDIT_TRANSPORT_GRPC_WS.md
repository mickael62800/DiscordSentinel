# Audit transport — candidats gRPC & WebSocket

> Où le projet gagnerait à passer du HTTP/polling vers **gRPC** (bot ↔ API interne) ou **WebSocket** (push temps réel vers le web).
> Constat de départ : gRPC et WS sont **déjà largement adoptés**. L'objectif n'est donc pas d'en ajouter partout, mais de combler des **incohérences** et de remplacer du **polling** par du push événementiel.

---

## État des lieux

**gRPC déjà en place** (`sentinel-proto/proto/`) : `automod`, `blackjack`, `community`, `coude` (player/combats/bets/economy/inventory/social), `export`, `images`, `members`, `moderation`, `progression`, `roles`, `security`, `stats`, `tickets`, `voice`, `welcome`.

**WebSocket déjà en place** : `sentinel-gateway` relaie la stream Redis `sentinel:events` vers les clients ; côté web, `stores/realtimeStore.ts` consomme ces événements.

---

## 🔌 Candidats gRPC (bot ↔ API)

### 1. Tamagotchi — incohérence majeure (haute)

C'est le **seul** domaine de jeu **sans service gRPC**. Le bot parle à l'API entièrement en **HTTP** :

| Fichier | Appels |
|---|---|
| `sentinel-bot/src/modules/tamagotchi/panel.rs` | `post_json`/`get_json` (`/api/tamagotchi/pets`, `/care`, `/train`, `/visit`, `/combat`, `/{guild}/{owner}`, `/{guild}/{owner}/card`) |
| `sentinel-bot/src/modules/tamagotchi/refresh.rs` | `get_json` (`/api/tamagotchi/cards`, paginé) |
| `sentinel-bot/src/modules/tamagotchi/lifecycle_events.rs` | `get_json` (`/api/tamagotchi/{guild}/{owner}`) |

Tous les modules comparables (voice, coude, blackjack, progression, stats…) sont en gRPC.

**Reco** : créer un `TamagotchiService` (proto) avec CRUD pets, `Care`/`Train`/`Combat`, `Tick`, `SetCardLocation`, `ListCards`. Le rafraîchissement horaire des cartes (aujourd'hui `GET /cards` paginé) gagnerait à utiliser un **server-streaming** gRPC (`rpc ListCards(...) returns (stream CardItem)`) plutôt qu'une pagination par curseur.

### 2. Presets / whitelist vocaux — dette introduite (moyenne)

Les endpoints ajoutés récemment passent par **HTTP** (`BaseApiClient`) alors que tout le reste du domaine voice est sur `VoiceChannelsService` (gRPC) :

| Fichier | Appels HTTP |
|---|---|
| `sentinel-bot/src/modules/voice/api_client.rs` | `get_preset`, `save_preset`, `get_whitelist` → `self.base.get_json`/`post_fire_and_forget` |
| `sentinel-bot/src/modules/voice/handlers/voice/channel_lifecycle.rs` | lit preset + whitelist à la création |

**Reco** : ajouter `SavePreset` / `GetPreset` / `GetWhitelist` au proto `voice.proto` existant (le service `VoiceChannelsService` a déjà `AddToWhitelist`) pour l'uniformité du domaine.

### 3. Vérifier les domaines à gRPC partiel (basse)

`welcome.proto` n'expose que `GetConfig` ; vérifier qu'aucun chemin bot↔API welcome ne reste en HTTP pour des écritures. Idem pour les modules récents (`bump`, `rotation`, `ai_dataset`) qui semblent 100% HTTP — à évaluer selon leur fréquence d'appel (faible fréquence → HTTP acceptable).

---

## 📡 Candidats WebSocket (push vs polling)

7 fichiers web utilisent `setInterval`. Il faut distinguer **données événementielles** (→ push) et **jauges échantillonnées** (→ polling légitime).

### Bons candidats au push (événements)

| Fichier:ligne | Aujourd'hui | Reco |
|---|---|---|
| `components/organisms/AutomodAnalysisHistory.vue` | polling périodique de l'historique | Une nouvelle analyse est un **événement** → publier `automod_analysis_new` sur `sentinel:events`, consommer via `realtimeStore`. |
| `components/pages/GamePortalPage.vue` + `components/molecules/GameServerStatsBar.vue` | `setInterval` sur `gamePortalService` | Démarrage / arrêt / join de serveur = événements → push de l'état (le worker `game_portal` peut émettre). |

### Polling légitime — NE PAS forcer en WS

| Fichier | Pourquoi le polling reste le bon choix |
|---|---|
| `components/pages/ServerHealthPage.vue` | jauges CPU/mémoire échantillonnées : un push nécessiterait quand même un échantillonnage serveur. |
| `components/pages/SystemOpsPage.vue` | métriques d'ops échantillonnées. |
| `components/organisms/DockerAdminSection.vue` | statut conteneurs (état sampled). |
| `components/atoms/ConnectionBanner.vue` | `/health` toutes les 90 s = **fallback** de connectivité sain. (Voir aussi audit atomic : le fetch devrait être extrait dans un composable.) |

> Règle : pousser ce qui **change par événement discret** (analyse, sanction, ticket, session de jeu, état de salon vocal) ; continuer à interroger ce qui est une **mesure continue** (CPU, mémoire, santé).

---

## Priorités

1. **Tamagotchi → gRPC** (`TamagotchiService`, avec streaming pour `ListCards`). Incohérence la plus nette + bénéfice sur le refresh.
2. **Presets vocaux → `voice.proto`** (résorber la dette HTTP introduite récemment).
3. **AutomodAnalysisHistory + GamePortal → WS push** (vrais événements, gateway + `realtimeStore` déjà en place).
4. (Veille) Vérifier les chemins welcome/bump/rotation/ai_dataset restés en HTTP ; migrer seulement si la fréquence le justifie.
