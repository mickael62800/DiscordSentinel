# export-worker

File d'attente asynchrone des exports (CSV/JSON) demandes via le dashboard.
Decharge l'API des longues queries d'export.

## Role

1. Poll `export_jobs` (status='pending') a intervalle court (defaut 5s)
2. Reclame le job de maniere atomique
3. Execute l'export via gRPC vers l'API (`ExportService.Execute`) :
   - infractions
   - audit_logs
   - moderation_actions
4. Persiste le fichier resultant et met a jour le job (status='done',
   `result_path`, `row_count`)
5. Le dashboard polle / WS pour recuperer le download URL

## Variables d'environnement

| Var | Defaut | Role |
|---|---|---|
| `DATABASE_URL` | (requis) | Connexion Postgres |
| `API_URL` | (requis) | URL de l'API HTTP (heartbeat) |
| `GRPC_API_URL` | (requis) | URL de l'API gRPC (port 50051) |
| `API_KEY` | (requis) | Bearer token API |
| `EXPORT_SCAN_INTERVAL` | 5 | Interval polling (secondes) |
| `METRICS_PORT` | 9100 | Port d'expose Prometheus |

## Tables touchees

- `export_jobs` (lecture/ecriture)

## Healthcheck

Endpoint `GET /metrics` sur `:9100`.
