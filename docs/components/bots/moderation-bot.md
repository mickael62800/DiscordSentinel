# moderation-bot

**Rôle** : Central de modération avec ban/mute/warn temporaires ou permanents, historique utilisateur, appels staff, notes, templates de raison.

## Commandes / Events Discord principaux

- Slash `/ban` — bannir permanemment ou temporairement avec raison
- Slash `/unban` — débannir
- Slash `/warn` — avertissement enregistré
- Slash `/mute` — sourdine temporaire
- Slash `/history` — historique d'infractions d'un utilisateur
- Slash `/notes` — notes internes sur un utilisateur
- Slash `/call` — appeler un staff

## API interne (Phase 7A)

**Statut : partiel.** Hot path migré (log_action + get_history), le reste reste HTTP.

**gRPC** (`ModerationService` sur `:50051`, défini dans `services/proto/proto/moderation.proto` — **partagé avec voice-bot** pour le log anti-flood) :
- `LogAction` — appelé sur **chaque** ban/mute/warn (hot path n°1 du bot)
- `GetHistory` — consultation fréquente via `/history`

Wrappent `ManageModerationUseCase`. Conversion `DomainError → tonic::Status` alignée sur les codes HTTP.

**HTTP retenu** (`BaseApiClient`) :
- `POST /api/moderation/evidence` / `GET /api/moderation/evidence/{action_id}` — preuves attachées
- `POST /api/moderation/review` / `GET /api/moderation/review/{guild_id}/pending` / `PATCH .../resolve` — file de relecture
- `GET /api/moderation/modstats/{guild_id}` — stats par modérateur
- `POST /api/moderation/pending` / `PATCH /api/moderation/pending/{action_id}` — actions en attente d'approbation
- `GET /api/reminders/{guild_id}` — rappels actifs
- `POST /api/notes` — notes utilisateur
- `POST /api/bots/config` — set config bot

**Pourquoi pas full gRPC ?** Le `ManageModerationUseCase` côté API n'expose que 4 méthodes (`log_action`, `get_history`, `list_bans`, `delete_bans_for_user`). Les endpoints evidence/review/modstats/pending/notes utilisent des **repos directs** dans les handlers HTTP, pas un use case unifié. Pour les migrer, il faudrait d'abord refactor le domaine (ajouter des méthodes au use case ou créer des `ManageModerationEvidenceUseCase`/`ManageReviewQueueUseCase` séparés). À traiter dans une vague de consolidation après la première vague de migration.

## Comportement si l'API tombe

- **`log_action` (gRPC)** : circuit breaker → `Err("API indisponible")`. Une sanction ratée est loggée en erreur côté bot — **mais elle reste appliquée côté Discord** (le ban/mute a déjà été exécuté via Serenity AVANT l'appel API). Seul le log backend est manquant. Le modérateur est notifié via le retour Err pour qu'il sache que la trace n'a pas été enregistrée.
- **`get_history` (gRPC)** : retourne `Err("API indisponible...")`. La commande slash `/history` répond à l'utilisateur clairement.
- **Endpoints HTTP (evidence, review, etc.)** : comportement inchangé, `BaseApiClient` retry une fois puis remonte l'erreur.
- **Redis pub/sub (rappels d'expiration)** : indépendant de l'API HTTP/gRPC. Continue de fonctionner si Redis est dispo.

## Modules clés

- `src/commands/ban.rs` / `mute.rs` / `warn.rs` — actions de modération
- `src/commands/history.rs` — récupération de l'historique via gRPC
- `src/reason_templates.rs` — templates de raisons prédéfinies
- `src/handler.rs` — timeout/débannissement + listener Redis
- `src/api_client.rs` — wrapper gRPC `ModerationService` (2 méthodes) + HTTP pour le reste

## Variables d'env

- `MODERATION_DISCORD_TOKEN`
- `API_BASE_URL`
- `GRPC_API_URL`
- `API_KEY`
- `REDIS_URL` / `REDIS_CHANNEL`

## Cache Serenity (Phase 1)

**Tier : `full`** — cache complet requis pour résoudre les permissions et `voice_states`.

## Note Phase 2 / 4

Les rappels d'expiration de sanction (24h avant fin d'un mute/ban temporaire) arrivent désormais via Redis pub/sub depuis le `moderation-worker` job `send_reminders`. Le bot doit écouter l'event `sanction_expiry_reminder` et envoyer un DM au modérateur.
