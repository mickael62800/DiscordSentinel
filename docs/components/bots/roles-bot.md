# roles-bot

**Rôle** : Synchronise les rôles Discord avec l'API backend, gère l'auto-attribution au join et les panneaux réactionnels.

## Commandes / Events Discord principaux

- Slash `/roles-panel deploy` — déployer un panel dans un salon
- Slash `/roles-panel list` — lister les panels
- Event `guild_member_addition` — auto-attribution de rôles avec délai optionnel
- Event `reaction_add` / `reaction_remove` — attribution/retrait par réaction
- Background task — sync périodique (5 min) des rôles Discord ↔ API

## API interne (Phase 7A)

**Statut : partiel.** Role-panels via gRPC, sync_discord_roles reste HTTP.

**gRPC** (`RolePanelsService` sur `:50051`, défini dans `services/proto/proto/roles.proto` — **partagé avec community-bot**) :
- `GetPanel` — détail d'un panel par ID
- `GetPanelByMessage` — lookup par message Discord
- `ListPanels` — tous les panels d'une guild
- `SetMessageId` — attache l'ID de message Discord à un panel
- `ListAutoRoles` — auto-roles configurés

Wrappent `ManageRolePanelsUseCase`.

**HTTP retenu** (`BaseApiClient`) :
- `POST /api/discord-roles/{guild_id}/sync` — sync des rôles Discord vers la table `discord_roles`. Pas de use case unifié (`SyncDiscordRolesUseCase`) côté API → reste sur HTTP. Migration possible quand le domaine sera consolidé.

## Comportement si l'API tombe

- **Role panels (gRPC)** : circuit breaker → `Err("API indisponible")`. Les commandes `/roles-panel deploy/list` répondent un message d'erreur clair. Les interactions sur panels existants tombent en erreur — l'utilisateur peut réessayer.
- **Auto-roles au join** : `list_auto_roles` peut échouer pendant la panne → le bot **ne donne pas le rôle automatiquement**. Best-effort, le membre devra le récupérer manuellement ou via une re-sync.
- **Sync périodique (HTTP, 5 min)** : la background task saute simplement le tour. Aucune perte — la prochaine itération réussira au retour de l'API.

## Modules clés

- `src/handler.rs` — EventHandler, `sync_all_guild_roles`, auto-role logic
- `src/commands/roles_panel.rs` — déploiement des panneaux
- `src/api_client.rs` — wrapper gRPC `RolePanelsService` + HTTP pour `sync_discord_roles`

## Variables d'env

- `ROLES_DISCORD_TOKEN`
- `API_BASE_URL`
- `GRPC_API_URL`
- `API_KEY`

## Cache Serenity (Phase 1)

**Tier : `small`** — cache channels pour panneaux.

## Note Phase 2

La colonne `discord_roles.permissions` est maintenant un `BIGINT` (au lieu de `TEXT`). Le bot envoie toujours la valeur sous forme de `String` dans le payload JSON (safety JS côté desktop), et l'API parse en `i64`.
