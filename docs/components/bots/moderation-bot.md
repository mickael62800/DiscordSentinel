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

## Dépendances externes

- API interne (moderation_actions, infractions, notes)
- Discord Gateway + REST
- Redis (listener `sentinel:events` pour `sanction_expiry_reminder` depuis Phase 4 B)

## Modules clés

- `src/commands/ban.rs` / `mute.rs` / `warn.rs` — actions de modération
- `src/commands/history.rs` — récupération de l'historique via API
- `src/reason_templates.rs` — templates de raisons prédéfinies
- `src/handler.rs` — timeout/débannissement + listener Redis

## Variables d'env

- `MODERATION_DISCORD_TOKEN`
- `API_BASE_URL`
- `API_KEY`
- `REDIS_URL` / `REDIS_CHANNEL`

## Cache Serenity (Phase 1)

**Tier : `full`** — cache complet requis pour résoudre les permissions et `voice_states`.

## Note Phase 2 / 4

Les rappels d'expiration de sanction (24h avant fin d'un mute/ban temporaire) arrivent désormais via Redis pub/sub depuis le `moderation-worker` job `send_reminders`. Le bot doit écouter l'event `sanction_expiry_reminder` et envoyer un DM au modérateur.
