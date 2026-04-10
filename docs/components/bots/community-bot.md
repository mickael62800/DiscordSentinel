# community-bot

**Rôle** : Gère les rôles temporaires, réactions d'auto-attribution, groupes exclusifs et parrainage avec suivi de durée.

## Commandes / Events Discord principaux

- Slash `/roles-panel deploy` — déployer un panel réactionnel
- Slash `/roles-panel list` — lister les panels
- Slash `/sponsor` — gérer le parrainage
- Event `reaction_add` / `reaction_remove` — auto-attribution de rôles
- Background task — nettoyage des rôles temporaires (60s)

## Dépendances externes

- API interne (panels, sponsorships, temp_roles)
- Discord Gateway + REST

## Modules clés

- `src/temp_roles.rs` — suivi des rôles temporaires et expiration
- `src/sponsorship.rs` — tracker de parrainage
- `src/exclusive_groups.rs` — groupes de rôles mutuellement exclusifs
- `src/prerequisites.rs` — conditions d'accès

## Variables d'env

- `COMMUNITY_DISCORD_TOKEN`
- `API_BASE_URL`
- `API_KEY`

## Cache Serenity (Phase 1)

**Tier : `minimal`**.

## Note Phase 4

Le cleanup in-memory des `temp_roles` (60s tokio loop) cohabite maintenant avec le nouveau `temp-roles-worker` qui scanne la DB et publie des events Redis. À terme, le bot peut écouter les events au lieu de maintenir son tracker.
