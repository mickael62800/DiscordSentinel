# discord-audit-sync-worker

Ingestion continue des audit-logs Discord via l'API REST. Reconcilie ce que
le bot voit en gateway avec ce que Discord a effectivement enregistre.

## Role

1. Pour chaque guild active, fetch `/guilds/{guild_id}/audit-logs?after=...`
2. Detecte les actions absentes de `audit_logs` (DB) et les ingere :
   - bans manuels (kick, member_update, role_update, etc.)
   - actions hors-bot (admin Discord modifiant les permissions)
3. Met a jour le cursor `last_audit_log_id` par guild

C'est la source de verite cote audit DB, plus fiable que les events gateway
qui peuvent etre rates en cas de disconnect.

## Variables d'environnement

| Var | Defaut | Role |
|---|---|---|
| `DATABASE_URL` | (requis) | Connexion Postgres |
| `API_URL` | (requis) | URL de l'API |
| `API_KEY` | (requis) | Bearer token API |
| `SENTINEL_DISCORD_TOKEN` | (requis) | Token Discord (VIEW_AUDIT_LOG required) |
| `AUDIT_SYNC_INTERVAL` | 300 | Interval sync (secondes, defaut 5 min) |
| `METRICS_PORT` | 9100 | Port d'expose Prometheus |

## Tables touchees

- `audit_logs` (ecriture INSERT, partition mensuelle)
- `audit_sync_state` (lecture/ecriture du cursor)

## Healthcheck

Endpoint `GET /metrics` sur `:9100`.
