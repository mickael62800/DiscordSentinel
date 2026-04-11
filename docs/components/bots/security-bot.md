# security-bot

**Rôle** : Détecte les raids, comptes alts, spams de join massifs et applique quarantaine, CAPTCHA, slowmode et lockdown automatiques.

## Commandes / Events Discord principaux

- Slash `/security status` — état des défenses (raid detector, quarantine, slowmode, lockdown)
- Slash `/security history` — derniers événements de sécurité (N=5 par défaut)
- Event `guild_member_addition` — vérification âge compte, détection raid, captcha
- Event `message` — slowmode adaptatif si raid détecté

## API interne (Phase 7A)

**Statut : full gRPC.** Tous les appels métier de security-bot passent par gRPC.

**gRPC** (deux services sur `:50051`) :

`SecurityService` (défini dans `services/proto/proto/security.proto`) :
- `ReportEvent` — push d'un événement de sécurité (raid détecté, alt, lockdown, etc.)
- `ListEvents` — listing pour `/security history`

`MembersService` (défini dans `services/proto/proto/members.proto` — **partagé avec welcome-bot**) :
- `SyncMembers` — sync batch des membres au démarrage / refresh
- `RegisterMember` — nouveau membre détecté
- `RemoveMember` — départ
- `UpdateMember` — changement pseudo/avatar/rôles
- `GetMember` — lookup individuel

`SecurityService` wrappe `ManageSecurityUseCase`, `MembersService` wrappe `ManageMembersUseCase`. Le champ `roles` (JSON arbitraire côté domain `serde_json::Value`) est sérialisé en `string roles_json` côté proto pour rester transparent.

**HTTP retenu** : aucun appel métier. Le `BaseApiClient` reste injecté pour le heartbeat partagé.

## Comportement si l'API tombe

- **`report_event` (gRPC)** : circuit breaker → erreur. Les détections (raid, alt, etc.) sont **toujours appliquées côté Discord** (kick/quarantine/lockdown via Serenity), mais la trace côté API est manquante. Acceptable : l'action de défense est prioritaire sur le logging.
- **`list_events` (gRPC)** : `/security history` répond une erreur claire au modérateur.
- **Members CRUD (gRPC fire-and-forget)** : sync échoue, pas d'impact immédiat sur Discord. Les retries automatiques ne sont pas implémentés ; la prochaine sync (au prochain join) tentera de rattraper.
- **Trackers in-memory** (`RaidDetector`, `AltDetector`, `QuarantineManager`, `LockdownManager`) : **autonomes**, ne dépendent pas de l'API. Les défenses continuent de marcher complètement même API down.

## Modules clés

- `src/security/raid_detector.rs` — suivi des joins récents et seuil d'alerte
- `src/security/account_checker.rs` — vérification de l'âge minimum des comptes
- `src/security/alt_detector.rs` — clustering par date de création et similarité de nom
- `src/security/quarantine.rs` / `lockdown.rs` / `slowmode.rs` — actions réactives
- `src/api_client.rs` — wrapper gRPC `SecurityService` + `MembersService` (full)

## Variables d'env

- `SECURITY_DISCORD_TOKEN`
- `API_BASE_URL`
- `GRPC_API_URL`
- `API_KEY`
- `RAID_JOIN_THRESHOLD` / `RAID_JOIN_WINDOW_SECS`
- `CAPTCHA_ENABLED`
- `QUARANTINE_ENABLED`

## Cache Serenity (Phase 1)

**Tier : `medium`** — cache messages récents pour contexte.

## Note Phase 2

Les events sont stockés dans `security_events.user_ids` en JSONB avec un index GIN (migration 100) → les queries du style `WHERE user_ids @> '["<user_id>"]'::jsonb` sont accélérées 10-50×.
