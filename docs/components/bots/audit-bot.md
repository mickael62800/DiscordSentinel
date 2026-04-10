# audit-bot

**Rôle** : Enregistre et suit les modifications au sein d'une guild Discord (messages supprimés, rôles modifiés, membres bannis, etc.) avec détection d'anomalies et rapports hebdomadaires.

## Commandes / Events Discord principaux

- Slash `/audit search` — rechercher dans les logs d'audit par utilisateur et type d'événement
- Slash `/audit stats` — statistiques hebdomadaires
- Event `guild_member_addition` / `member_update` — suivi des modifications de membres
- Event `message_delete` / `message_update` — traçabilité des messages
- Event `guild_ban_addition` / `guild_member_remove` — logs des bannissements
- Event `channel_create` / `channel_update` / `role_update`
- Event `guild_invite_create` / `voice_state_update`

## Dépendances externes

- API interne via `BaseApiClient` (logs, audit_logs, anomalies)
- Discord Gateway (Serenity)
- Redis (optionnel) pour event publishing temps-réel

## Modules clés

- `src/handler/` — 9 sous-handlers (guild, member, channel, role, message, voice, invite, thread)
- `src/anomaly.rs` — détection mass ban / mass delete / mass role change
- `src/weekly_report.rs` — rapport hebdomadaire (tous les lundis 8h UTC)

## Variables d'env

- `AUDIT_DISCORD_TOKEN`
- `API_BASE_URL`
- `API_KEY`
- `REDIS_URL` (optionnel)

## Cache Serenity (Phase 1)

**Tier : `medium`** — cache 100 messages/channel pour reconstituer le contexte des suppressions.
