# ticket-bot

**Rôle** : Gestion des tickets de support avec panel de création, escalade SLA, transcripts, satisfaction rating et FAQ.

## Commandes / Events Discord principaux

- Slash `/ticket panel` — déployer le panel de création
- Slash `/ticket close` — fermer le ticket du salon courant
- Slash `/ticket invite` — inviter un membre au ticket
- Event `interaction` (buttons) — création, fermeture, actions interactives
- Background task — escalade SLA (5 min) pour tickets sans réponse

## API interne (Phase 7A)

**Statut : full gRPC sur le domaine tickets.** Endpoints SLA et priority restent HTTP.

**gRPC** (`TicketsService` sur `:50051`, défini dans `services/proto/proto/tickets.proto`) :
- `ListTickets` — listing avec filtres (status, priority, search, author_id, limit, offset)
- `GetTicketDetail` — ticket + messages
- `CreateTicket` — création
- `ReplyTicket` — ajout d'un message
- `CloseTicket` — fermeture
- `UpdateStatus` — changement de status
- `AssignTicket` — assignation à un staff
- `UpdateTicketChannel` — sync `voice_channel_id` / `invited_user_id`

Wrappent `ManageTicketsUseCase`. Les 11 call sites de `ApiClient::new(base.clone())` (dispersés dans 6 fichiers : `handler.rs`, `commands/ticket/*`) ont été mis à jour pour passer le `GrpcClientKey` en plus du `BaseApiClient`.

**HTTP retenu** (`BaseApiClient`) :
- `PATCH /api/tickets/{id}/status` (avec payload `{priority}`) — `update_ticket_priority` réutilise l'endpoint flexible status. Pas de RPC dédié en v1, à voir plus tard si on ajoute `UpdatePriority` au proto.
- `PATCH /api/tickets/{id}/sla` — handler API ad hoc (pas de use case unifié), `update_ticket_sla` reste fire-and-forget HTTP.

## Comportement si l'API tombe

- **gRPC tickets** : circuit breaker → `Err("API indisponible")`. Les commandes `/ticket panel/close/invite` répondent un message d'erreur clair. Les boutons d'interaction tombent en erreur — l'utilisateur peut réessayer.
- **Background task SLA (5 min)** : appelle `list_tickets` toutes les 5 min. Si l'API est down, la task **saute simplement le tour** (`Err → return`). Pas d'escalade pendant la panne, repart au tour suivant. Les tickets en cours côté Discord ne sont pas affectés.
- **`update_ticket_priority` / `update_ticket_sla` (HTTP fire-and-forget)** : drop silencieux. Les valeurs SLA sont historiques (pas critiques), seront resync au prochain événement.
- **Redis pub/sub** : indépendant. Les notifications de nouveau message côté staff continuent si Redis est dispo.

## Modules clés

- `src/sla.rs` — tracker d'escalade (SLA, timeouts tickets)
- `src/transcript.rs` — génération des transcripts
- `src/templates.rs` — templates de réponse prédéfinies
- `src/satisfaction.rs` — rating post-closure
- `src/api_client.rs` — wrapper gRPC `TicketsService` + HTTP fallback (priority, SLA)

## Variables d'env

- `TICKET_DISCORD_TOKEN`
- `API_BASE_URL`
- `GRPC_API_URL`
- `API_KEY`
- `REDIS_URL` / `REDIS_CHANNEL`

## Cache Serenity (Phase 1)

**Tier : `small`** — cache channels pour panneaux.
