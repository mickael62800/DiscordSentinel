# monitoring-worker

**Rôle** : Surveille la disponibilité des bots et workers via les heartbeats Redis, détecte les transitions offline/online et publie des alertes.

## Particularité : pas de `spawn_periodic`

Contrairement aux autres workers, `monitoring-worker` **n'utilise pas** `spawn_periodic` ni PostgreSQL. Sa boucle principale est dans `src/monitor.rs` :

```rust
loop {
    tokio::time::sleep(Duration::from_secs(check_interval)).await;
    // 1. lire les heartbeats Redis
    // 2. comparer aux derniers états connus
    // 3. détecter offline (heartbeat manquant > threshold)
    // 4. publier les transitions sur l'API pour alerting
}
```

Intervalle : 30 secondes par défaut (`MONITOR_CHECK_INTERVAL`).

## Dépendances externes

- Redis (lecture des heartbeats publiés par les autres services)
- API interne (POST des transitions offline/online)

## Modules clés

- `src/main.rs` — startup (pas de PgPool)
- `src/config.rs` — intervalle de check
- `src/monitor.rs` — boucle de monitoring et détection offline

## Variables d'env

- `REDIS_URL` / `API_URL`
- `MONITOR_CHECK_INTERVAL` (défaut 30s)

## Tables DB

**Aucune** — le worker est stateless, uniquement Redis + API.
