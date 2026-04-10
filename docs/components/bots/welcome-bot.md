# welcome-bot

**Rôle** : Envoie un message de bienvenue templé aux nouveaux membres avec détection de guild et customisation.

## Commandes / Events Discord principaux

- Event `guild_member_addition` — détection d'un nouveau membre
- Envoi du message de bienvenue via template

## Dépendances externes

- API interne (`welcome_config`)
- Discord Gateway + REST

## Modules clés

- `src/template.rs` — modèles de messages (variables, embeds)
- `src/handler.rs` — EventHandler pour `guild_member_addition`
- `src/api_client.rs` — récupération du template depuis l'API

## Variables d'env

- `WELCOME_DISCORD_TOKEN`
- `API_BASE_URL`
- `API_KEY`

## Cache Serenity (Phase 1)

**Tier : `minimal`** — pas d'accès cache, appel API simple.
