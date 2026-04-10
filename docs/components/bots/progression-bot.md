# progression-bot

**Rôle** : Suivi de l'expérience et des niveaux des utilisateurs avec badges, multiplicateurs, streaks et récompenses automatiques.

## Commandes / Events Discord principaux

- Slash `/stats user` — XP et niveau d'un utilisateur
- Slash `/stats server` — stats globales du serveur
- Slash `/stats top` — classement des plus actifs
- Slash `/level` — niveau du commandeur
- Event `message` / `voice_state_update` — gain d'XP en messages et vocal

## Dépendances externes

- API interne (`user_levels`, `level_config`, `level_rewards`)
- Discord Gateway

## Modules clés

- `src/tracker.rs` — suivi des XP et niveaux en mémoire (déduplication par user)
- `src/streaks.rs` — streaks actifs (jours consécutifs)
- `src/xp_cooldown.rs` — cooldown entre les gains d'XP
- `src/badges.rs` — badges basés sur les jalons (level 10, 50, etc.)
- `src/multipliers.rs` — boosters temporaires

## Variables d'env

- `PROGRESSION_DISCORD_TOKEN`
- `API_BASE_URL`
- `API_KEY`

## Cache Serenity (Phase 1)

**Tier : `small`** — cache channels pour les notifications de level-up dans le salon configuré.

## Note Phase 2

Le leaderboard lit maintenant depuis `mv_level_leaderboard` (refresh 5 min par cache-worker) — gain typique 100-1000× sur l'endpoint `/api/levels/{guild_id}/leaderboard`.
