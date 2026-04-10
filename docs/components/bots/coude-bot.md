# coude-bot

**Rôle** : Jeu RPG Discord avec progression, combat PvP, inventaire, shop, classes, assurances et défis multijoueurs (« Coup de Coude »).

## Commandes / Events Discord principaux

- Slash `/profil` — profil et statistiques du joueur
- Slash `/hp` — état de santé et classe
- Slash `/leaderboard` — classements serveur
- Slash `/casino` / `/pari` / `/train` / `/classe` / `/shop` — 20+ commandes de jeu
- Slash `/coude challenge` — interface de défis multijoueurs

## Dépendances externes

- API interne (`/api/coude/*` — hexagonal : players, combats, bets, economy, inventory, social)
- Discord Gateway

## Modules clés

- `src/commands/coude/` — commandes UI (sous-dossier, refactor Phase 3)
- `src/game/combat.rs` / `src/game/chaos.rs` — mécanique du combat PvP
- `src/game/progression.rs` / `src/game/classes.rs` — leveling et classes
- `src/game/shop.rs` — inventaire et commerce
- `src/guild_config.rs` — parse JSON config (déjà JSONB-ready)

## Variables d'env

- `COUDE_DISCORD_TOKEN`
- `API_BASE_URL`
- `API_KEY`

## Cache Serenity (Phase 1)

**Tier : `minimal`** — toute la logique métier est en DB/API, peu d'interactions avec le cache Discord.

## Note Phase 2

La colonne `coude_players.class` est passée en enum Postgres `coude_class` (migration 103). Le bot utilise les 4 valeurs : `bourrin`, `agile`, `fourbe`, `tank`. Les leaderboards lisent depuis `mv_coude_leaderboard` (refresh 5 min).
