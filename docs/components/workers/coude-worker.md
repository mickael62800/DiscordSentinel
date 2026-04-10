# coude-worker

**Rôle** : Gère l'expiration des combats pendants et la résolution des combats en phase de paris pour le jeu « Coup de Coude ».

## Jobs périodiques

| Job | Intervalle défaut | Fichier |
|---|---|---|
| `expire_combats` | 24h (`COMBAT_EXPIRY_CHECK_SECS`) | `src/jobs/expire_combats.rs` |
| `resolve_betting` | 30 s (`BETTING_CHECK_SECS`) | `src/jobs/resolve_betting.rs` |

### Détails

- **expire_combats** — scanne `coude_combats WHERE status='pending' AND created_at < NOW() - INTERVAL ...`, rembourse les coins engagés et marque le combat comme expiré.
- **resolve_betting** — scanne les combats en phase `betting_accepted` dont la fenêtre de pari est écoulée, déclenche le combat et distribue les gains pari-mutuel aux parieurs gagnants (cf. `coude_bet.rs` domain entity avec 3 tests unitaires).

## Dépendances externes

- PostgreSQL
- API interne (heartbeat)
- Discord Gateway **indirect** : le worker utilise le token Discord uniquement pour certaines notifications REST (si besoin). Pas de connexion gateway persistante.

## Modules clés

- `src/main.rs` — startup avec `COUDE_DISCORD_TOKEN`
- `src/config.rs` — intervalles combats + paris
- `src/scheduler.rs` — enregistre les 2 jobs
- `src/jobs/expire_combats.rs`
- `src/jobs/resolve_betting.rs`

## Variables d'env

- `DATABASE_URL` / `API_URL`
- `COUDE_DISCORD_TOKEN`
- `COMBAT_EXPIRY_CHECK_SECS` (défaut 86400)
- `BETTING_CHECK_SECS` (défaut 30)

## Tables DB

- `coude_combats` (UPDATE status)
- `coude_bets` (UPDATE, compute payouts)
- `coude_players` (UPDATE coins, win/loss counters)
