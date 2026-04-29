# blackjack-cleanup-worker

Nettoyage des tables de blackjack inactives. Permet le scaling horizontal du
bot blackjack en centralisant la GC.

## Role

1. Scan `blackjack_tables` (status='active' + last_action > 30 min)
2. Pour chaque table abandonnee :
   - Marque `status='abandoned'`
   - Refunds les mises en cours via `wallet_transactions`
   - Publie un event WS `blackjack_table_abandoned`

## Variables d'environnement

| Var | Defaut | Role |
|---|---|---|
| `DATABASE_URL` | (requis) | Connexion Postgres |
| `REDIS_URL` | (requis) | Pub/sub WS events |
| `API_URL` | (requis) | URL de l'API |
| `API_KEY` | (requis) | Bearer token API |
| `BLACKJACK_CLEANUP_SCAN_INTERVAL` | 60 | Interval de scan (secondes) |
| `METRICS_PORT` | 9100 | Port d'expose Prometheus |

## Tables touchees

- `blackjack_tables` (lecture/ecriture)
- `wallet_transactions` (ecriture refunds)

## Healthcheck

Endpoint `GET /metrics` sur `:9100`.
