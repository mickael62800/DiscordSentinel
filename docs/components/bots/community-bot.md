# community-bot

**Rôle** : Gère les rôles temporaires, réactions d'auto-attribution, groupes exclusifs et parrainage avec suivi de durée.

## Commandes / Events Discord principaux

- Slash `/roles-panel deploy` — déployer un panel réactionnel
- Slash `/roles-panel list` — lister les panels
- Slash `/sponsor` — gérer le parrainage
- Event `reaction_add` / `reaction_remove` — auto-attribution de rôles
- Background task — nettoyage des rôles temporaires (60s)

## API interne (Phase 7A)

**Statut : partiel.** Les role-panels passent en gRPC, sponsorships et temp-roles restent HTTP.

**gRPC** (`RolePanelsService` sur `:50051`, défini dans `services/proto/proto/roles.proto` — **partagé avec roles-bot**) :
- `GetPanel` — détail d'un panel par ID
- `GetPanelByMessage` — lookup par message Discord
- `ListPanels` — tous les panels d'une guild
- `SetMessageId` — attache l'ID de message Discord à un panel
- `ListAutoRoles` — auto-roles configurés (avec délai)

Tous wrappent `ManageRolePanelsUseCase`. Conversion propre `DomainError::NotFound → panel: None` pour préserver le contrat HTTP 404 d'origine.

**HTTP retenu** (`BaseApiClient`) :
- `POST /api/sponsorships` — création (fire-and-forget) — pas de use case unifié
- `POST /api/temp-roles` — création (fire-and-forget)
- `GET /api/temp-roles/{guild_id}` — listing pour cleanup
- `DELETE /api/temp-roles/{guild_id}/{user_id}/{role_id}` — suppression

**Pourquoi pas gRPC pour temp-roles/sponsorships ?** Pas de use case unifié côté API (repos directs). Migration possible quand `ManageTempRolesUseCase` et `ManageSponsorshipsUseCase` seront consolidés.

## Comportement si l'API tombe

- **Role panels (gRPC)** : circuit breaker → `Err("API indisponible")`. Les commandes `/roles-panel deploy/list` répondent un message d'erreur clair. Les interactions sur panels existants tombent en erreur — l'utilisateur peut réessayer.
- **Sponsorships (HTTP fire-and-forget)** : la création échoue silencieusement côté bot. **Acceptable** pour un parrainage occasionnel ; perte de la trace côté API mais pas de l'effet Discord.
- **Temp-roles cleanup (HTTP)** : la background task de nettoyage saute simplement le tour. Les rôles expirés seront re-tentés au cycle suivant (60 s) — le tracker in-memory continue de tracer. Le `temp-roles-worker` côté API rattrape aussi de son côté.

## Modules clés

- `src/temp_roles.rs` — suivi des rôles temporaires et expiration
- `src/sponsorship.rs` — tracker de parrainage
- `src/exclusive_groups.rs` — groupes de rôles mutuellement exclusifs
- `src/prerequisites.rs` — conditions d'accès
- `src/api_client.rs` — wrapper gRPC `RolePanelsService` + HTTP pour le reste

## Variables d'env

- `COMMUNITY_DISCORD_TOKEN`
- `API_BASE_URL`
- `GRPC_API_URL`
- `API_KEY`

## Cache Serenity (Phase 1)

**Tier : `minimal`**.

## Note Phase 4

Le cleanup in-memory des `temp_roles` (60s tokio loop) cohabite maintenant avec le nouveau `temp-roles-worker` qui scanne la DB et publie des events Redis. À terme, le bot peut écouter les events au lieu de maintenir son tracker.
