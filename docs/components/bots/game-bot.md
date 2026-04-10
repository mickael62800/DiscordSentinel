# game-bot

**Rôle** : Propose des mini-jeux textuels simples (devinettes, trivia, etc.) avec détection de participation.

## Commandes / Events Discord principaux

- Event `message` — détection des réponses des utilisateurs
- Interactions avec messages et boutons pour jeux interactifs

## Dépendances externes

- API interne (scores)
- Discord Gateway

## Modules clés

- `src/detector.rs` — logique de détection des réponses correctes
- `src/handler.rs` — dispatch des messages vers les jeux
- `src/api_client.rs` — communication avec l'API pour scores

## Variables d'env

- `GAME_DISCORD_TOKEN`
- `API_BASE_URL`
- `API_KEY`

## Cache Serenity (Phase 1)

**Tier : `minimal`** — traitement simple des messages.
