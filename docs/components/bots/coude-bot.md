# coude-bot

**Rôle** : Jeu RPG Discord avec progression, combat PvP, inventaire, shop, classes, assurances et défis multijoueurs (« Coup de Coude »).

## Commandes / Events Discord principaux

- Slash `/profil` — profil et statistiques du joueur
- Slash `/hp` — état de santé et classe
- Slash `/leaderboard` — classements serveur
- Slash `/casino` / `/pari` / `/train` / `/classe` / `/shop` — 20+ commandes de jeu
- Slash `/coude challenge` — interface de défis multijoueurs

## API interne (Phase 7A)

**Statut : partiel.** Hot path joueurs en gRPC (6 RPCs), tout le reste reste HTTP.

**gRPC** (`CoudePlayerService` sur `:50051`, défini dans `services/proto/proto/coude.proto`) :
- `GetOrCreatePlayer` — appelé sur **chaque interaction utilisateur** (hot path n°1)
- `GetPlayer` — lecture profil (avec gestion `NotFound → Ok(None)` pour parité HTTP 404)
- `UpdatePlayerClass` — changement de classe avec validation enum
- `AddXp` — gain d'XP (chaque action de jeu)
- `AdjustCoins` — ajustement du solde (gains/pertes)
- `UpdateHp` — mise à jour HP

Wrappe `ManageCoudePlayersUseCase`. Couvre les 6 méthodes les plus chaudes sur les 18 du trait.

**HTTP retenu** (`BaseApiClient`) — ~75 méthodes restantes :
- **Combat** : `create_combat`, `resolve_combat`, `expire_combat`, `set_defender_special`, etc. (`ManageCoudeCombatsUseCase`)
- **Bets** : `place_bet`, `resolve_bets`, `refund_bets` (`ManageCoudeBetsUseCase`)
- **Économie** : transferts, vols, casino (`ManageCoudeEconomyUseCase`)
- **Inventaire** : items, primes, assurances (`ManageCoudeInventoryUseCase`)
- **Social** : leaderboards (`ManageCoudeSocialUseCase`)
- **Saison** : `get_current_season`

**Pourquoi seulement 6 RPCs ?** Le coude-bot a ~80 méthodes API réparties sur 5 use cases. Migrer le tout d'un coup = 5 protos × ~15 RPCs chacun. La stratégie a été de **migrer le hot path d'abord** (ce qui est appelé à chaque interaction) et garder le reste HTTP. Une vague de consolidation ultérieure peut migrer combat/bets/economy quand le profil de trafic le justifie.

## Comportement si l'API tombe

- **gRPC (hot path joueurs)** : circuit breaker → `Err("API indisponible")`. Les commandes `/profil`, `/hp`, etc. répondent un message d'erreur. Les actions de jeu (`/train`, `/casino`) qui nécessitent `add_xp`/`adjust_coins` échouent et ne consomment pas de coins — la BDD reste cohérente.
- **HTTP (combat/bets/casino/etc.)** : erreurs HTTP standard, pas de circuit breaker. Les défis multijoueurs en cours peuvent rester dans un état pending → expirés naturellement par `expire_combat` au retour de l'API.
- **Cohérence** : aucune action n'est appliquée côté Discord sans confirmation backend — le bot ne fait pas d'optimistic update.

## Modules clés

- `src/commands/coude/` — commandes UI (sous-dossier, refactor Phase 3)
- `src/game/combat.rs` / `src/game/chaos.rs` — mécanique du combat PvP
- `src/game/progression.rs` / `src/game/classes.rs` — leveling et classes
- `src/game/shop.rs` — inventaire et commerce
- `src/guild_config.rs` — parse JSON config (déjà JSONB-ready)
- `src/api_client.rs` — wrapper hybride gRPC (6 hot path) + HTTP (~75 endpoints)

## Variables d'env

- `COUDE_DISCORD_TOKEN`
- `API_BASE_URL`
- `GRPC_API_URL`
- `API_KEY`

## Cache Serenity (Phase 1)

**Tier : `minimal`** — toute la logique métier est en DB/API, peu d'interactions avec le cache Discord.

## Note Phase 2

La colonne `coude_players.class` est passée en enum Postgres `coude_class` (migration 103). Le bot utilise les 4 valeurs : `bourrin`, `agile`, `fourbe`, `tank`. Les leaderboards lisent depuis `mv_coude_leaderboard` (refresh 5 min).
