# blackjack-bot

**Rôle** : Fournit des tables de blackjack interactives avec gestion de sessions, jetons (wallet partagé) et interactions button-based Discord.

## Commandes / Events Discord principaux

- Slash `/blackjack setup` — configuration des salons de jeu
- Button interactions pour actions de jeu (hit, stand, double, split)
- Gestion automatique des timeouts et nettoyage des tables inactives

## Dépendances externes

- API interne (wallet partagé, tables multi-joueurs)
- Discord Gateway

## Modules clés

- `src/commands/blackjack/` — game logic (embeds, messages, button handlers)
- `src/channel_manager.rs` — gestion des salons et sessions actives
- `src/handler/game.rs` — état des mains et actions
- `src/handler/table.rs` — état des tables
- `src/handler/afk_cleanup.rs` — nettoyage des joueurs inactifs

## Variables d'env

- `BLACKJACK_DISCORD_TOKEN`
- `API_BASE_URL`
- `API_KEY`

## Cache Serenity (Phase 1)

**Tier : `minimal`** — peu d'interaction avec le cache Serenity.
