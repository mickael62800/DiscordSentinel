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

## API interne

**Statut Phase 7A : non migré gRPC.** Décision **architecturale** : audit-bot reste sur le pattern **Redis Streams + HTTP** par design.

- **Redis Streams** (`event_bus`, Phase 5B) — push fire-and-forget de tous les events Gateway. Buffer naturel + replay côté API + workers, parfait pour des events à haut volume sans besoin de réponse synchrone.
- **HTTP `BaseApiClient`** — pour les rares appels qui ont besoin d'une réponse (`/audit/search`, `/audit/stats`).

**Pourquoi pas gRPC ?** Le hot path d'audit-bot est purement fire-and-forget : on enregistre, on n'attend pas de retour. Redis Streams donne déjà la garantie de buffering, replay et découplage. gRPC apporterait le contrat typé mais zéro gain de latence sur ce profil de trafic. La migration est possible mais pas prioritaire.

## Comportement si l'API tombe

- **Redis disponible** : aucun impact. Les events continuent d'être poussés sur le stream et seront consommés par les workers quand l'API revient. **Buffer naturel** côté Redis (rétention configurable).
- **Redis indisponible** : les events sont perdus (pas de file d'attente in-memory côté bot). Acceptable pour de l'audit (best-effort) — l'alternative serait de persister sur disque local, considéré overkill.
- **API HTTP indisponible** : `/audit/search` et `/audit/stats` (commandes slash staff) répondent une erreur. Aucun impact sur le pipeline d'enregistrement asynchrone.

## Modules clés

- `src/handler/` — 9 sous-handlers (guild, member, channel, role, message, voice, invite, thread)
- `src/anomaly.rs` — détection mass ban / mass delete / mass role change
- `src/weekly_report.rs` — rapport hebdomadaire (tous les lundis 8h UTC)

## Variables d'env

- `AUDIT_DISCORD_TOKEN`
- `API_BASE_URL`
- `API_KEY`
- `REDIS_URL` (recommandé — sans Redis le bot fonctionne en best-effort dégradé)

## Cache Serenity (Phase 1)

**Tier : `medium`** — cache 100 messages/channel pour reconstituer le contexte des suppressions.
