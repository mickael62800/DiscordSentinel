# Bots Discord — Index

15 bots Serenity + 1 librairie partagée (`shared`).

| Bot | Rôle | Cache tier | gRPC (Phase 7A) |
|---|---|---|---|
| [audit-bot](./audit-bot.md) | Logs d'audit + détection d'anomalies + rapports hebdo | medium | ❌ Redis Streams |
| [automod-bot](./automod-bot.md) | Modération auto (spam, phishing, insultes, unicode) | small | ✅ full (`AutomodService`) |
| [blackjack-bot](./blackjack-bot.md) | Tables de blackjack interactives (jetons + sessions) | minimal | ✅ full solo (`BlackjackService`) |
| [cleanup-bot](./cleanup-bot.md) | Purge des logs, infractions, audit (rétention) | minimal | ❌ skip (admin/maintenance) |
| [community-bot](./community-bot.md) | Rôles temporaires, auto-attribution, parrainage | minimal | 🟡 partiel (`RolePanelsService`) |
| [coude-bot](./coude-bot.md) | Jeu RPG PvP (combat, inventaire, shop, classes) | minimal | 🟡 partiel (`CoudePlayerService`) |
| [game-bot](./game-bot.md) | Mini-jeux textuels (devinettes, trivia) | minimal | ❌ skip (faible trafic) |
| [image-bot](./image-bot.md) | Analyse d'images (NSFW, perceptual hash) | small | ✅ full (`ImagesService`, bytes natifs) |
| [moderation-bot](./moderation-bot.md) | Ban/mute/warn + historique + templates | full | 🟡 partiel (`ModerationService`) |
| [progression-bot](./progression-bot.md) | XP, niveaux, badges, streaks, multipliers | small | ✅ full (`ProgressionService` + `StatsService`) |
| [roles-bot](./roles-bot.md) | Sync rôles Discord ↔ API + panels réactionnels | small | 🟡 partiel (`RolePanelsService`) |
| [security-bot](./security-bot.md) | Raid detection, alt detection, quarantaine, lockdown | medium | ✅ full (`SecurityService` + `MembersService`) |
| [ticket-bot](./ticket-bot.md) | Tickets support + SLA + transcripts + satisfaction | small | ✅ full (`TicketsService`) |
| [voice-bot](./voice-bot.md) | Salons vocaux temporaires + AFK sweep + vote-kick | full | ✅ full (`VoiceChannelsService` + `ModerationService`) |
| [welcome-bot](./welcome-bot.md) | Messages de bienvenue templés | minimal | 🟡 partiel (`MembersService`) |
| [shared](./shared.md) | **Librairie commune** (api_client, grpc_client, circuit_breaker, etc.) | — | — |

**Légende statut gRPC** :
- ✅ **full** — toutes les méthodes API du bot passent par gRPC.
- 🟡 **partiel** — hot path migré, certaines méthodes (rares ou cross-domain) restent sur HTTP.
- ❌ **skip** — pas de migration gRPC. Soit le bot a un trafic négligeable (cleanup, game), soit il utilise un autre transport (audit-bot → Redis Streams).

## Architecture API : coexistence HTTP + gRPC (Phase 7A)

Depuis la **Phase 7A**, l'API Sentinel expose **deux transports en parallèle** :

- **HTTP/Axum** sur `:3000` — historique, toujours actif, zéro régression. Utilisé par les bots non migrés et pour les endpoints qui n'ont pas (encore) d'équivalent gRPC.
- **gRPC/tonic** sur `:50051` — Phase 7A. Wrappe les **mêmes use cases** que les handlers HTTP (zéro duplication de logique métier). 12 services, ~50 RPCs distincts.

Les deux transports partagent le même `AppState`, la même DB, les mêmes broadcasts WebSocket. Aucun handler HTTP n'a été touché par la migration.

### Pourquoi gRPC en interne

- **Binaire (protobuf)** au lieu de JSON texte → payloads plus petits, sérialisation plus rapide. Pour image-bot, **plus de base64** (gain ~33% sur la bande passante d'images).
- **HTTP/2 multiplexé** → connexion TCP unique persistante par bot, requêtes concurrentes sans head-of-line blocking.
- **Contrats typés** (`.proto`) → désynchronisations API/bot impossibles, le compilateur garantit la compatibilité.

### Comportement uniforme « API down »

Tous les bots migrés utilisent le **circuit breaker** de `bots/shared/src/circuit_breaker.rs` :

1. 5 échecs consécutifs (`Unavailable` / `DeadlineExceeded` / `Internal`) → **breaker ouvert pendant 10 s**
2. Pendant l'ouverture : tous les appels gRPC renvoient instantanément `Err("API indisponible (circuit breaker ouvert)")` — pas de timeout, pas de hang
3. Après le cooldown : transition **half-open**, un seul appel test autorisé
4. Succès → referme. Échec → nouvelle ouverture pour 10 s.

La stratégie de dégradation est **propre à chaque bot** (slash command qui répond avec un message clair, fire-and-forget qui drop, retry au tour suivant, etc.) — voir la section « Comportement si l'API tombe » dans chaque doc bot.

## Tiers de cache Serenity (Phase 1)

- **minimal** — aucun message, pas de channels/users. Pour bots sans besoin de contexte.
- **small** — cache channels uniquement. Pour bots qui postent des notifications ciblées.
- **medium** — cache 100 msg/channel. Pour audit/security qui ont besoin du contexte des suppressions.
- **full** — défaut Serenity. Nécessaire pour voice_states (voice-bot) et résolution permissions (moderation-bot).

## Variables d'env communes

Tous les bots partagent :
- `<NOM>_DISCORD_TOKEN` — token du bot Discord
- `API_BASE_URL` — URL HTTP de l'API interne (`services/api`, défaut `http://127.0.0.1:3000`)
- `GRPC_API_URL` — URL gRPC de l'API interne (défaut `http://127.0.0.1:50051`). **Phase 7A**.
- `API_KEY` — bearer token, partagé entre HTTP et gRPC (interceptor `authorization: Bearer <key>`)
- `REDIS_URL` — optionnel, pour publier des events temps-réel
