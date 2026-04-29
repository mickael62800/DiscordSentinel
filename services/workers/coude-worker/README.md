# coude-worker

Worker du jeu Coup de Coude. Gere les expirations, resolutions de paris,
regen HP et caisse communautaire.

## Role (multi-jobs)

1. **combat_expiry** : expire les combats en attente apres N jours sans
   resolution (defaut 24h)
2. **betting_resolve** : resout les paris des combats termines (defaut 30s)
3. **hp_regen** : tick HP des joueurs blesses (defaut 5 min)
4. **cashbox_redistribute** : redistribue la cagnotte aux joueurs eligibles
   apres N jours (defaut 7j)
5. **bounty_expiry** : expire les primes inactives
6. **steal_protection_expiry** : expire les protections vol expirees
7. **steal_boost_expiry** : expire les boosts vol expires
8. **insurance_expiry** : expire les assurances payees
9. **prison_release** : libere les joueurs en prison apres expiration

## Variables d'environnement

| Var | Defaut | Role |
|---|---|---|
| `DATABASE_URL` | (requis) | Connexion Postgres |
| `API_URL` | (requis) | URL de l'API HTTP |
| `GRPC_API_URL` | (requis) | URL de l'API gRPC (port 50051) |
| `API_KEY` | (requis) | Bearer token API |
| `SENTINEL_DISCORD_TOKEN` | (requis) | Token Discord (DM notifications) |
| `COMBAT_EXPIRY_CHECK_SECS` | 86400 | Interval expiry (24h) |
| `BETTING_CHECK_SECS` | 30 | Interval bets resolve |
| `HP_REGEN_TICK_SECS` | 300 | Interval HP regen |
| `CASHBOX_TICK_SECS` | (defaut) | Interval cashbox |
| `CASHBOX_MIN_DAYS` | (defaut) | Anciennete min avant redistribution |
| `METRICS_PORT` | 9100 | Port d'expose Prometheus |

## Tables touchees

- `coude_combats`, `coude_bets`, `coude_players` (lecture/ecriture)
- `coude_cashbox`, `coude_bounty`, `coude_steal_*`, `coude_insurance` (lecture/ecriture)

## Healthcheck

Endpoint `GET /metrics` sur `:9100`.
