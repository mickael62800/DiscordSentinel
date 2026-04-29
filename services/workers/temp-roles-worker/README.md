# temp-roles-worker

Scan des roles temporaires expires et publication d'un event Redis pour que
le bot community supprime le role Discord.

## Role

1. Scan `temp_roles` (expires_at < NOW()) a intervalle court (defaut 60s)
2. Pour chaque role expire :
   - Publie un event Redis `temp_role_expire` avec `{guild_id, user_id, role_id}`
   - DELETE de la row
3. Le bot community ecoute le pub/sub et supprime le role via Discord REST.

Architecture : pas d'appel Discord direct depuis le worker — il delegue au
bot pour eviter de devoir gerer le rate limit Discord ici.

## Variables d'environnement

| Var | Defaut | Role |
|---|---|---|
| `DATABASE_URL` | (requis) | Connexion Postgres |
| `REDIS_URL` | (requis) | Pub/sub WS events |
| `API_URL` | (requis) | URL de l'API |
| `API_KEY` | (requis) | Bearer token API |
| `TEMP_ROLES_SCAN_INTERVAL` | 60 | Interval scan (secondes) |
| `METRICS_PORT` | 9100 | Port d'expose Prometheus |

## Tables touchees

- `temp_roles` (lecture/ecriture)

## Cles Redis

- Pub/sub `sentinel:events` (publication `temp_role_expire`)

## Healthcheck

Endpoint `GET /metrics` sur `:9100`.
