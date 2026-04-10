# roles-bot

**Rôle** : Synchronise les rôles Discord avec l'API backend, gère l'auto-attribution au join et les panneaux réactionnels.

## Commandes / Events Discord principaux

- Slash `/roles-panel deploy` — déployer un panel dans un salon
- Slash `/roles-panel list` — lister les panels
- Event `guild_member_addition` — auto-attribution de rôles avec délai optionnel
- Event `reaction_add` / `reaction_remove` — attribution/retrait par réaction
- Background task — sync périodique (5 min) des rôles Discord ↔ API

## Dépendances externes

- API interne (`discord_roles` table — via `POST /api/discord-roles/{guild_id}/sync`)
- Discord Gateway + REST

## Modules clés

- `src/handler.rs` — EventHandler, `sync_all_guild_roles`, auto-role logic
- `src/commands/roles_panel.rs` — déploiement des panneaux
- `src/api_client.rs` — sync rôles Discord vers l'API

## Variables d'env

- `ROLES_DISCORD_TOKEN`
- `API_BASE_URL`
- `API_KEY`

## Cache Serenity (Phase 1)

**Tier : `small`** — cache channels pour panneaux.

## Note Phase 2

La colonne `discord_roles.permissions` est maintenant un `BIGINT` (au lieu de `TEXT`). Le bot envoie toujours la valeur sous forme de `String` dans le payload JSON (safety JS côté desktop), et l'API parse en `i64`.
