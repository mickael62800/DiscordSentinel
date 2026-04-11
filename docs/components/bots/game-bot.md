# game-bot

**Rôle** : Propose des mini-jeux textuels simples (devinettes, trivia, etc.) avec détection de participation.

## Commandes / Events Discord principaux

- Event `message` — détection des réponses des utilisateurs
- Interactions avec messages et boutons pour jeux interactifs

## API interne

**Statut Phase 7A : non migré gRPC.** Décision **architecturale** : game-bot reste sur HTTP par design.

**HTTP `BaseApiClient`** :
- `GET /api/games/{guild_id}` — liste des jeux d'un serveur
- `POST /api/games` — création d'un jeu
- `DELETE /api/games/{guild_id}/{game_id}` — suppression
- `POST /api/games/{guild_id}/{game_id}/subscribe` — inscription joueur
- `DELETE /api/games/{guild_id}/{game_id}/subscribe/{user_id}` — désinscription
- `GET /api/games/{guild_id}/{game_id}/subscribers` — liste des inscrits
- `GET /api/games/{guild_id}/by-name/{game_name}` — lookup par nom
- `GET /api/games/{guild_id}/user/{user_id}` — jeux d'un utilisateur

**Pourquoi pas gRPC ?** Trafic faible (mini-jeux occasionnels, pas appelés à chaque message Discord), pas de use case unifié côté API (repos directs). Même logique que cleanup-bot : pas de retour sur investissement pour la migration. Reste sur HTTP `BaseApiClient`.

## Comportement si l'API tombe

- Les commandes slash répondent une erreur HTTP standard.
- Les events `message` continuent de tourner mais ne peuvent pas vérifier l'inscription d'un joueur — best-effort, l'utilisateur retentera.
- Aucun état perdu côté Discord — la BDD reste source de vérité.

## Modules clés

- `src/detector.rs` — logique de détection des réponses correctes
- `src/handler.rs` — dispatch des messages vers les jeux
- `src/api_client.rs` — communication HTTP avec l'API pour scores et inscriptions

## Variables d'env

- `GAME_DISCORD_TOKEN`
- `API_BASE_URL`
- `API_KEY`

## Cache Serenity (Phase 1)

**Tier : `minimal`** — traitement simple des messages.
