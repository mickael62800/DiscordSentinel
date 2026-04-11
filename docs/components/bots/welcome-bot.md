# welcome-bot

**Rôle** : Envoie un message de bienvenue templé aux nouveaux membres avec détection de guild et customisation.

## Commandes / Events Discord principaux

- Event `guild_member_addition` — détection d'un nouveau membre
- Envoi du message de bienvenue via template

## API interne (Phase 7A)

**Statut : partiel.** Lookup membre en gRPC, config welcome reste HTTP.

**gRPC** (`MembersService` sur `:50051`, défini dans `services/proto/proto/members.proto` — **partagé avec security-bot**) :
- `GetMember` — utilisé par `is_known_member()` pour distinguer un nouveau membre d'un retour. Hot path : appelé sur **chaque** `guild_member_addition`. Conversion `NotFound → false` pour parité avec l'ancien check HTTP 404.

**HTTP retenu** (`BaseApiClient`) :
- `GET /api/welcome/{guild_id}` — `get_config()`. Le `WelcomeConfig` est un blob de 22 champs (welcome/leave/rules/counter/anniversary), pas de use case unifié côté API, lecture peu fréquente (1 fois par event member). Le coût d'un `welcome.proto` ne se justifie pas pour un seul appel non-critique. Reste sur HTTP.

## Comportement si l'API tombe

- **`is_known_member` (gRPC)** : circuit breaker → `Err`. Le bot considère par défaut que le membre est **inconnu** (parité avec le comportement « 404 = nouveau »). Conséquence : le message envoyé sera celui de bienvenue normale, pas le message « rejoin ». Acceptable comme dégradation gracieuse.
- **`get_config` (HTTP)** : si l'appel échoue, le bot **ne peut pas envoyer le message** (pas de template). Il loggue l'erreur et passe au suivant. Le membre rejoint quand même Discord normalement, juste sans accueil personnalisé. Au retour de l'API, les nouveaux membres reprennent le flow normal.

## Modules clés

- `src/template.rs` — modèles de messages (variables, embeds)
- `src/handler.rs` — EventHandler pour `guild_member_addition`
- `src/api_client.rs` — wrapper gRPC `MembersService` (1 méthode) + HTTP pour `get_config`

## Variables d'env

- `WELCOME_DISCORD_TOKEN`
- `API_BASE_URL`
- `GRPC_API_URL`
- `API_KEY`

## Cache Serenity (Phase 1)

**Tier : `minimal`** — pas d'accès cache, appel API simple.
