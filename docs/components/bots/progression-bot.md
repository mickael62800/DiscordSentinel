# progression-bot

**Rôle** : Suivi de l'expérience et des niveaux des utilisateurs avec badges, multiplicateurs, streaks et récompenses automatiques.

## Commandes / Events Discord principaux

- Slash `/stats user` — XP et niveau d'un utilisateur
- Slash `/stats server` — stats globales du serveur
- Slash `/stats top` — classement des plus actifs
- Slash `/level` — niveau du commandeur
- Event `message` / `voice_state_update` — gain d'XP en messages et vocal

## API interne (Phase 7A)

**Statut : full gRPC.** progression-bot a été le **bot pilote** de la migration Phase 7A.

**gRPC** (deux services sur `:50051`) :

`ProgressionService` (défini dans `services/proto/proto/progression.proto`) :
- `AddXp` — gain d'XP texte/voice (hot path : à chaque message + à chaque tick voice)
- `GetUserLevel` — lecture profil (avec gestion `NotFound → Ok(None)` pour parité HTTP 404)
- `GetLeaderboard` — classement par source (text/voice/total)
- `GetRewards` — liste des récompenses configurées

`StatsService` (défini dans `services/proto/proto/stats.proto`) :
- `RecordMessages` — compteur messages
- `RecordVoice` — compteur secondes vocal
- `GetUserStats` — stats individuelles
- `GetGuildOverview` — stats globales serveur
- `GetLeaderboard` — top membres

Tous wrappent `ManageLevelsUseCase` et `ManageStatsUseCase`. Les broadcasts WebSocket `xp_gained` / `stats_messages_recorded` / `stats_voice_recorded` sont émis identiquement à l'ancien path HTTP — le dashboard reçoit les mêmes events temps-réel.

**HTTP retenu** (`BaseApiClient`) — endpoints non couverts par les use cases v1 :
- `GET /api/levels/{guild_id}/{user_id}/streak` — streaks (jours consécutifs)
- `PATCH /api/levels/{guild_id}/{user_id}/streak` — update streaks
- `GET /infractions/{guild_id}` — domaine moderation, rare

## Comportement si l'API tombe

- **`add_xp` (gRPC)** : circuit breaker → `Err("API indisponible")`. L'XP du message courant est perdu (acceptable, le suivant repart au retour de l'API). Aucune action côté Discord, le bot n'envoie pas de notif level-up sur des données incomplètes.
- **`record_messages` / `record_voice` (gRPC fire-and-forget)** : court-circuités. Les compteurs in-memory du `StatsTracker` continuent d'accumuler localement et seront flushes au prochain tick quand l'API revient.
- **Commandes slash** (`/stats`, `/level`, `/top`) : répondent « API indisponible, réessayez dans quelques instants ». Pas de hang, pas de timeout côté Discord.
- **Streaks (HTTP)** : la commande échoue silencieusement (fire-and-forget patch). Le `streak_current` côté in-memory continue, sera resync au retour de l'API.

## Modules clés

- `src/tracker.rs` — suivi des XP et niveaux en mémoire (déduplication par user)
- `src/streaks.rs` — streaks actifs (jours consécutifs)
- `src/xp_cooldown.rs` — cooldown entre les gains d'XP
- `src/badges.rs` — badges basés sur les jalons (level 10, 50, etc.)
- `src/multipliers.rs` — boosters temporaires
- `src/api_client.rs` — wrapper gRPC `ProgressionService` + `StatsService` + HTTP pour les streaks

## Variables d'env

- `PROGRESSION_DISCORD_TOKEN`
- `API_BASE_URL`
- `GRPC_API_URL`
- `API_KEY`

## Cache Serenity (Phase 1)

**Tier : `small`** — cache channels pour les notifications de level-up dans le salon configuré.

## Note Phase 2

Le leaderboard lit maintenant depuis `mv_level_leaderboard` (refresh 5 min par cache-worker) — gain typique 100-1000× sur l'endpoint `/api/levels/{guild_id}/leaderboard`.
