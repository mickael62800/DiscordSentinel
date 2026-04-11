# blackjack-bot

**Rôle** : Fournit des tables de blackjack interactives avec gestion de sessions, jetons (wallet partagé) et interactions button-based Discord.

## Commandes / Events Discord principaux

- Slash `/blackjack setup` — configuration des salons de jeu
- Button interactions pour actions de jeu (hit, stand, double, split)
- Gestion automatique des timeouts et nettoyage des tables inactives

## API interne (Phase 7A)

**Statut : full gRPC pour le solo + wallet.** Tables multijoueur restent HTTP.

**gRPC** (`BlackjackService` sur `:50051`, défini dans `services/proto/proto/blackjack.proto`) :
- `StartGame` — démarrage d'une partie solo (lit min/max bet, starting coins, blackjack payout depuis `bot_guild_config` côté API)
- `Hit` — tirage carte
- `Stand` — fin du joueur, dealer joue
- `DoubleDown` — double mise + 1 carte
- `GetActive` — récupère la partie en cours d'un joueur
- `GetWallet` — solde du joueur

Tous wrappent `BlackjackService` (application) + `WalletRepository`. Les broadcasts WebSocket `blackjack_result` sont émis identiquement à l'ancien path HTTP — aucun changement côté dashboard.

**HTTP retenu** (`BaseApiClient`) — tables multijoueur :
- `POST /api/blackjack/tables` — création
- `POST /api/blackjack/tables/{id}/join` — rejoindre
- `GET /api/blackjack/tables/by-channel/{channel_id}` — lookup
- `GET /api/blackjack/tables/{id}/players` — liste participants
- `DELETE /api/blackjack/tables/{id}` — fermeture

**Pourquoi pas gRPC pour les tables ?** Pas encore de use case unifié côté API pour le domaine multijoueur (repos directs dans les handlers). Migration possible une fois `ManageBlackjackTablesUseCase` consolidé.

## Comportement si l'API tombe

Circuit breaker actif sur tous les appels gRPC :

- **`start_game`** : retourne `Err("API indisponible")`. La commande slash répond à l'utilisateur clairement, **aucune mise n'est débitée silencieusement** (la BDD reste cohérente).
- **`hit/stand/double_down`** : retournent l'erreur. Le joueur peut retenter son action — la partie reste dans son état précédent côté BDD.
- **`get_active`** : permet de détecter les parties orphelines au démarrage du bot. En panne, le bot ne charge rien — comportement gracieux.
- **`get_wallet`** : embed wallet affiche un message d'erreur.
- **Tables HTTP** : les commandes multijoueur tombent en erreur HTTP standard. Pas de circuit breaker côté HTTP, mais elles ne sont pas dans le hot path.

## Modules clés

- `src/commands/blackjack/` — game logic (embeds, messages, button handlers)
- `src/channel_manager.rs` — gestion des salons et sessions actives
- `src/handler/game.rs` — état des mains et actions
- `src/handler/table.rs` — état des tables
- `src/handler/afk_cleanup.rs` — nettoyage des joueurs inactifs
- `src/api_client.rs` — wrapper gRPC `BlackjackService` + HTTP fallback pour les tables

## Variables d'env

- `BLACKJACK_DISCORD_TOKEN`
- `API_BASE_URL`
- `GRPC_API_URL`
- `API_KEY`

## Cache Serenity (Phase 1)

**Tier : `minimal`** — peu d'interaction avec le cache Serenity.
