# temp-roles-worker

**Rôle** : Scanne la table `temp_roles` pour détecter les rôles temporaires expirés et publie des events Redis pour que `community-bot` applique les retraits Discord. Créé en Phase 4 B.

## Jobs périodiques

| Job | Intervalle défaut | Fichier |
|---|---|---|
| `expire_temp_roles` | 60 s (`TEMP_ROLES_SCAN_INTERVAL`) | `src/jobs/expire_temp_roles.rs` |

### Logique

```rust
SELECT id, guild_id, user_id, role_id FROM temp_roles
WHERE expires_at <= NOW() ORDER BY expires_at ASC LIMIT 100;
```

Pour chaque ligne trouvée, publication sur Redis canal `sentinel:events` :

```json
{
  "event": "temp_role_expire",
  "data": { "guild_id": "...", "user_id": "...", "role_id": "..." }
}
```

Le `community-bot` écoute déjà ce canal via `sentinel_shared::redis_listener`. Lorsqu'il reçoit l'event, il :
1. Exécute `member.remove_role()` via Serenity (nécessite la connexion gateway — impossible côté worker)
2. Appelle `DELETE /api/temp-roles/{guild_id}/{user_id}/{role_id}` pour purger la ligne

⚠️ Le worker **ne supprime pas** la ligne `temp_roles` lui-même — c'est le bot qui le fait après confirmation du retrait Discord. Cela permet le retry naturel : si le bot est down, le worker re-détectera le rôle expiré au prochain scan.

## Dépendances externes

- PostgreSQL (SELECT sur `temp_roles`)
- Redis (PUBLISH sur `sentinel:events`)

## Modules clés

- `src/main.rs` — startup
- `src/config.rs` — intervalle
- `src/scheduler.rs` — enregistre le job
- `src/jobs/expire_temp_roles.rs` — query + publish

## Variables d'env

- `DATABASE_URL` / `REDIS_URL` / `API_URL`
- `TEMP_ROLES_SCAN_INTERVAL` (défaut 60s)

## Tables DB

- `temp_roles` (SELECT seulement — le bot fait le DELETE via l'API après retrait Discord confirmé)

## Note de coexistence

Le `community-bot` maintient encore son tracker in-memory + cleanup loop 60s (ancien système). Les deux cohabitent sans conflit (les events Redis sont idempotents côté bot). Pour Phase 5 ou 6, on peut supprimer le tracker in-memory du bot une fois le worker validé en prod.
