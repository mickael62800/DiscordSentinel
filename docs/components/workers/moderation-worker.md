# moderation-worker

**Rôle** : Régénère les points de conduite, nettoie les bans vocaux expirés, synchronise les propositions de ban et envoie les rappels d'expiration de sanctions via Redis.

## Jobs périodiques (4 jobs)

| Job | Intervalle défaut | Fichier |
|---|---|---|
| `conduct_regen` | 1h (`CONDUCT_REGEN_INTERVAL`) | `src/jobs/conduct_regen.rs` |
| `cleanup_bans` | 1 min (`BAN_CLEANUP_INTERVAL`) | `src/jobs/cleanup_bans.rs` |
| `sync_ban_proposals` | 2 min (`SYNC_BAN_PROPOSALS_INTERVAL`) | `src/jobs/sync_ban_proposals.rs` |
| `send_reminders` | 30 s (`SEND_REMINDERS_INTERVAL`) | `src/jobs/send_reminders.rs` |

### Détails jobs

- **conduct_regen** — régénère les points de conduite pour tous les utilisateurs selon la config guild.
- **cleanup_bans** — supprime les entrées `voice_channel_bans` expirées.
- **sync_ban_proposals** — synchronise les propositions de ban en attente avec l'état Discord réel.
- **send_reminders** (Phase 4 B enrichi) — scanne `sanction_reminders WHERE status='pending' AND remind_at <= NOW()`, marque `status='sent'` **avant** broadcast (idempotence), puis publie l'event `sanction_expiry_reminder` sur Redis `sentinel:events`. Le `moderation-bot` écoute cet event et envoie un DM au modérateur.

## Dépendances externes

- PostgreSQL
- Redis (pub/sub pour les rappels)
- API interne (heartbeat)

## Modules clés

- `src/main.rs` — startup avec Redis client
- `src/config.rs` — 4 intervalles + `redis_url`
- `src/scheduler.rs` — enregistre les 4 jobs
- `src/jobs/*.rs` — 4 fichiers

## Variables d'env

- `DATABASE_URL` / `REDIS_URL` / `API_URL`
- `CONDUCT_REGEN_INTERVAL`
- `BAN_CLEANUP_INTERVAL`
- `SYNC_BAN_PROPOSALS_INTERVAL`
- `SEND_REMINDERS_INTERVAL`

## Tables DB

- `user_conduct_points` (UPDATE)
- `voice_channel_bans` (DELETE)
- `sanction_reminders` (UPDATE status)
- `pending_mod_actions` (SELECT/sync)
