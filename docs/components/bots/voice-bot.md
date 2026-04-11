# voice-bot

**Rôle** : Gère les salons vocaux temporaires, transfert d'ownership, vote-kick, AFK sweep, limite de membres et session cards de logs.

## Commandes / Events Discord principaux

- Slash `/voice setup`, `/voice transfer`, `/voice co-admin`, `/voice queue`, `/voice access-control`
- Event `voice_state_update` — création/destruction de salons temporaires
- Event `message` — commandes et votes dans les text-channels associés
- Background task — AFK sweep (60s) + nettoyage des salons vides

## API interne (Phase 7A)

**Statut : full gRPC.** Tous les appels métier de voice-bot passent par gRPC (voice channels + log de modération anti-flood).

**gRPC** (deux services sur `:50051`) :

`VoiceChannelsService` (défini dans `services/proto/proto/voice.proto`) :
- `ListChannels` — liste des salons d'une guild (utilisé au démarrage pour restaurer les maps)
- `CreateChannel` — création d'un salon temporaire
- `DeleteChannel` — suppression
- `UpdateChannel` — visibility, locked, queue_enabled, name, status, member_limit (tri-state via wrapper proto), queue_channel_id
- `GetChannel` — lookup par channel_id Discord
- `TransferOwnership` — transfert au nouvel owner
- `AddCoAdmin` — ajouter un co-admin
- `AddToWhitelist` — whitelist d'un user pour un owner
- `BanFromChannel` — ban temporaire ou permanent

`ModerationService` (réutilisé du moderation-bot, défini dans `services/proto/proto/moderation.proto`) :
- `LogAction` — log des mutes anti-flood. **Premier vrai cas de réutilisation cross-bot d'un service gRPC** dans le projet.

Wrappent `ManageVoiceChannelsUseCase` (9/22 méthodes du trait — themes et invite-links non utilisés par api_client.rs) et `ManageModerationUseCase`.

**HTTP retenu** : aucun appel métier. Le `BaseApiClient` reste injecté pour le heartbeat partagé.

## Pattern d'implémentation : `OnceLock` global

Le voice-bot a **27 call sites** de `ApiClient::new(base.clone())` dispersés dans 10+ fichiers (`handlers/`, `interactions/`, `commands/`). Pour éviter de tous les patcher, le `SentinelGrpcClient` est stocké dans un **`OnceLock<Arc<SentinelGrpcClient>>`** statique dans `src/api_client.rs`, initialisé depuis `main.rs` via `init_grpc()`. La signature `ApiClient::new(base)` est préservée et résout le client gRPC depuis le static — **zéro modification des call sites**.

C'est un pattern process-wide (initialisé une fois, lu de manière thread-safe, jamais muté), parfaitement adapté à un client de service singleton.

## Comportement si l'API tombe

- **`list_channels` (gRPC)** : appelé au démarrage. En panne, le bot ne charge aucun salon → les maps `text_to_voice` / `members_to_voice` / `voice_owner` sont vides. **Résultat** : les salons existants ne sont pas reconnus tant que l'API ne revient pas, mais les nouveaux peuvent être créés normalement.
- **`create_channel` / `update_channel` / `delete_channel`** : circuit breaker → `Err("API indisponible")`. Les commandes slash répondent une erreur. Les events `voice_state_update` qui déclenchent des créations automatiques sont droppés — l'utilisateur devra réessayer.
- **`log_moderation_action` (anti-flood)** : drop silencieux. Le mute Discord est appliqué quand même (via Serenity directement), seule la trace API manque.
- **AFK sweep (60s)** : task background autonome, ne dépend pas de l'API pour le tracking in-memory. Le kick AFK fonctionne indépendamment de l'API.

## Modules clés

- `src/handlers/voice/channel_lifecycle.rs` — création/destruction des salons temporaires
- `src/interactions/channel_management.rs` — limites, permissions, customisation
- `src/interactions/vote_kick.rs` — vote-kick avec majorité
- `src/session_card.rs` — carte de session vocale en logs (live updates)
- `src/state/afk_tracker.rs` — suivi AFK et kick automatique
- `src/tasks/afk_sweep.rs` — sweep périodique (60s)
- `src/api_client.rs` — wrapper gRPC `VoiceChannelsService` + `ModerationService` avec `OnceLock` pour le client global

## Variables d'env

- `VOICE_DISCORD_TOKEN`
- `API_BASE_URL`
- `GRPC_API_URL`
- `API_KEY`
- `VOICE_GUILD_ID`
- `VOICE_PUBLIC_CREATOR_CHANNEL_ID`
- `VOICE_PRIVATE_CREATOR_CHANNEL_ID`
- `VOICE_LOG_CHANNEL_ID` (optionnel)

## Cache Serenity (Phase 1)

**Tier : `full`** — cache complet requis pour résoudre les `voice_states`.

## Note Phase 2 / 4

- Les colonnes `voice_channels.kind` et `voice_channels.channel_status` sont maintenant typées (enum Postgres `voice_channel_kind` + index partiel `idx_voice_channels_active WHERE channel_status='open'`).
- Le `voice-afk-worker` a été **différé** (sweep 100% in-memory difficile à extraire). Le sweep reste dans le bot pour l'instant.
