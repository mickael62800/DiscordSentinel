# ai-worker

File d'attente asynchrone pour l'inference IA. Decharge les bots des appels
synchrones a `/analyze` (~5s timeout cote Discord) en mettant en file les
demandes dans la table `ai_jobs` et en les traitant en arriere-plan.

## Role

1. Poll la table `ai_jobs` (status='pending') a intervalle court (defaut 2s)
2. Reclame le job (status -> 'processing') de maniere atomique (FOR UPDATE SKIP LOCKED)
3. Appelle l'inference de l'API (`POST /analyze` ou `/analyze/image`)
4. Persiste le resultat (status='done' + `result_payload`) ou marque en `failed`
5. Publie le resultat sur Redis `ai_result:{job_id}` (TTL 600s) pour les bots qui ecoutent

Job timeout : si un job reste 'processing' plus longtemps que `AI_JOB_TIMEOUT`,
il est remis en 'pending' (le worker a probablement crash).

## Variables d'environnement

| Var | Defaut | Role |
|---|---|---|
| `DATABASE_URL` | (requis) | Connexion Postgres |
| `REDIS_URL` | (requis) | Publication des resultats |
| `API_URL` | (requis) | URL de l'API d'inference |
| `API_KEY` | (requis) | Bearer token API |
| `AI_POLL_INTERVAL` | 2 | Interval de polling DB (secondes) |
| `AI_JOB_TIMEOUT` | 120 | Timeout d'un job en cours (secondes) |
| `METRICS_PORT` | 9100 | Port d'expose Prometheus |

## Healthcheck

Endpoint `GET /metrics` sur `:9100` (server lance par `worker-common`).

## Tables touchees

- `ai_jobs` (lecture/ecriture)
