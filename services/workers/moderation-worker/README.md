# moderation-worker

Worker multi-jobs autour de la moderation : regen conduite, cleanup bans,
sync ban_proposals, rappels SLA.

## Role (4 jobs)

1. **conduct_regen** (defaut 1h) : regenere les points de conduite des
   utilisateurs (formula `apply_conduct_regen` dans le domain)
2. **ban_cleanup** (defaut 5 min) : sync les bans expires Discord -> DB
3. **sync_ban_proposals** (defaut 5 min) : sync les ban proposals en attente
   d'approbation par d'autres mods
4. **send_reminders** (defaut 60s) : envoie les rappels de fin de sanction
   24h avant expiration (publie sur Redis `sanction_expiry_reminder`)

## Variables d'environnement

| Var | Defaut | Role |
|---|---|---|
| `DATABASE_URL` | (requis) | Connexion Postgres |
| `REDIS_URL` | (requis) | Pub/sub WS events |
| `API_URL` | (requis) | URL de l'API |
| `API_KEY` | (requis) | Bearer token API |
| `CONDUCT_REGEN_INTERVAL` | 1 | Interval regen (heures) |
| `BAN_CLEANUP_INTERVAL` | 5 | Interval cleanup (minutes) |
| `SYNC_BAN_PROPOSALS_INTERVAL` | 5 | Interval ban proposals (minutes) |
| `SEND_REMINDERS_INTERVAL` | 60 | Interval reminders (secondes) |
| `METRICS_PORT` | 9100 | Port d'expose Prometheus |

## Tables touchees

- `user_conduct_points` (regen)
- `bans` (cleanup)
- `ban_proposals` (sync)
- `sanction_reminders` (lecture/ecriture)

## Healthcheck

Endpoint `GET /metrics` sur `:9100`.
