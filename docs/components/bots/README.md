# Bots Discord — Index

15 bots Serenity + 1 librairie partagée (`shared`).

| Bot | Rôle | Cache tier |
|---|---|---|
| [audit-bot](./audit-bot.md) | Logs d'audit + détection d'anomalies + rapports hebdo | medium |
| [automod-bot](./automod-bot.md) | Modération auto (spam, phishing, insultes, unicode) | small |
| [blackjack-bot](./blackjack-bot.md) | Tables de blackjack interactives (jetons + sessions) | minimal |
| [cleanup-bot](./cleanup-bot.md) | Purge des logs, infractions, audit (rétention) | minimal |
| [community-bot](./community-bot.md) | Rôles temporaires, auto-attribution, parrainage | minimal |
| [coude-bot](./coude-bot.md) | Jeu RPG PvP (combat, inventaire, shop, classes) | minimal |
| [game-bot](./game-bot.md) | Mini-jeux textuels (devinettes, trivia) | minimal |
| [image-bot](./image-bot.md) | Analyse d'images (NSFW, perceptual hash) | small |
| [moderation-bot](./moderation-bot.md) | Ban/mute/warn + historique + templates | full |
| [progression-bot](./progression-bot.md) | XP, niveaux, badges, streaks, multipliers | small |
| [roles-bot](./roles-bot.md) | Sync rôles Discord ↔ API + panels réactionnels | small |
| [security-bot](./security-bot.md) | Raid detection, alt detection, quarantaine, lockdown | medium |
| [ticket-bot](./ticket-bot.md) | Tickets support + SLA + transcripts + satisfaction | small |
| [voice-bot](./voice-bot.md) | Salons vocaux temporaires + AFK sweep + vote-kick | full |
| [welcome-bot](./welcome-bot.md) | Messages de bienvenue templés | minimal |
| [shared](./shared.md) | **Librairie commune** (api_client, config, embeds, etc.) | — |

## Tiers de cache Serenity (Phase 1)

- **minimal** — aucun message, pas de channels/users. Pour bots sans besoin de contexte.
- **small** — cache channels uniquement. Pour bots qui postent des notifications ciblées.
- **medium** — cache 100 msg/channel. Pour audit/security qui ont besoin du contexte des suppressions.
- **full** — défaut Serenity. Nécessaire pour voice_states (voice-bot) et résolution permissions (moderation-bot).

## Variables d'env communes

Tous les bots partagent :
- `<NOM>_DISCORD_TOKEN` — token du bot Discord
- `API_BASE_URL` — URL de l'API interne (`services/api`)
- `API_KEY` — bearer token vers l'API
- `REDIS_URL` — optionnel, pour publier des events temps-réel
