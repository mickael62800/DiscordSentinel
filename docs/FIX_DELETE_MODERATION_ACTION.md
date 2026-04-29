# Fix — DELETE /api/moderation/actions/{id} renvoie 404 sur des actions visibles dans le journal

## Symptôme

Cliquer **Annuler** sur une ligne `source="action"` (warn / unmute / ban appliqué) du
journal de modération renvoie `404 Not Found` :

```
API error 404: {"error":"Ressource introuvable : Action introuvable"}
```

La ligne reste visible dans le journal après refresh.

## Diagnostic

La table `moderation_actions` est **vide** (0 lignes). Les actions de
modération sont en réalité stockées dans `audit_logs` avec
`event_type LIKE 'mod_%'` (Phase 4 du refactor).

```sql
SELECT COUNT(*) FROM moderation_actions;       -- 0
SELECT COUNT(*) FROM audit_logs
  WHERE event_type LIKE 'mod_%';               -- 15
```

**Lecture** : déjà migrée vers `audit_logs`
(`moderation_repository.rs::find_all_for_guild` et `find_bans`).

**Écriture (DELETE)** : encore branchée sur `moderation_actions` →
404 systématique.

## Fichiers à modifier

### 1. `services/api/src/adapters/inbound/http/handlers/moderation.rs`

Dans `delete_action` (~ligne 725), le `SELECT` direct vise la mauvaise table :

```rust
// AVANT
let row: Option<(String, String, String, String)> = sqlx::query_as(
    "SELECT guild_id, target_id, target_name, action_type \
     FROM moderation_actions WHERE id = $1",
)
.bind(uuid)
.fetch_optional(&state.pg_pool)
.await
.map_err(|e| ApiError(DomainError::Internal(format!("fetch action: {e}"))))?;

let Some((guild_id, target_id, target_name, action_type)) = row else {
    return Err(ApiError(crate::domain::errors::DomainError::NotFound(
        "Action introuvable".into(),
    )));
};
```

```rust
// APRES
// Phase 4 : on lit depuis audit_logs (event_type LIKE 'mod_%').
// L ID expose au front est soit audit_logs.id, soit details->>'action_id'
// selon la presence dans details — on match sur les deux.
let row: Option<(String, Option<String>, Option<String>, String)> = sqlx::query_as(
    "SELECT guild_id, target_id, target_name, event_type \
     FROM audit_logs \
     WHERE event_type LIKE 'mod_%' \
       AND (id = $1 OR details->>'action_id' = $2) \
     LIMIT 1",
)
.bind(uuid)
.bind(uuid.to_string())
.fetch_optional(&state.pg_pool)
.await
.map_err(|e| ApiError(DomainError::Internal(format!("fetch action: {e}"))))?;

let Some((guild_id, target_id_opt, target_name_opt, event_type)) = row else {
    return Err(ApiError(crate::domain::errors::DomainError::NotFound(
        "Action introuvable".into(),
    )));
};
let target_id = target_id_opt.unwrap_or_default();
let target_name = target_name_opt.unwrap_or_default();
// event_type = "mod_<type>" (ex: mod_ban, mod_mute_temp). On retire le prefixe.
let action_type = event_type.strip_prefix("mod_").unwrap_or(&event_type).to_string();
```

> Le reste du handler (logique de reversal Discord `unban_user` / `remove_timeout`
> selon `action_type`, puis `state.moderation_uc.delete_action(uuid)`) reste inchangé.

### 2. `services/api/src/adapters/outbound/postgres/moderation_repository.rs`

#### `find_by_id` (~ligne 134)

```rust
// AVANT
"SELECT ... FROM audit_logs \
 WHERE event_type LIKE 'mod_%' AND details->>'action_id' = $1 \
 LIMIT 1"
```

```rust
// APRES — match aussi sur audit_logs.id pour les entrees sans action_id
"SELECT ... FROM audit_logs \
 WHERE event_type LIKE 'mod_%' \
   AND (id = $1 OR details->>'action_id' = $2) \
 LIMIT 1"
```

Bindings : `.bind(id).bind(id.to_string())`.

#### `delete_action` (~ligne 257)

```rust
// AVANT
"DELETE FROM audit_logs \
 WHERE event_type LIKE 'mod_%' AND details->>'action_id' = $1"
```

```rust
// APRES
"DELETE FROM audit_logs \
 WHERE event_type LIKE 'mod_%' \
   AND (id = $1 OR details->>'action_id' = $2)"
```

Bindings : `.bind(id).bind(id.to_string())`.

## Pourquoi le double match `id = $1 OR details->>'action_id' = $2`

Dans `AuditModRow → ModerationAction` (ligne 53-58), l'ID exposé au front est :

```rust
let id = row.details.get("action_id")
    .and_then(|v| v.as_str())
    .and_then(|s| Uuid::from_str(s).ok())
    .unwrap_or(row.id);  // ← fallback sur audit_logs.id si action_id absent
```

Donc selon que l'`audit_logs.details` contient ou non `action_id`, l'ID
côté front correspond à des colonnes différentes. Le double match couvre
les deux formats coexistants.

## Validation

Après application + redéploiement de l'API :

```bash
# Cliquer "Annuler" sur une ligne "Applique" du journal → doit renvoyer 204.
# Vérifier en BDD :
docker exec -it sentinel-postgres psql -U sentinel -d discord_sentinel \
  -c "SELECT COUNT(*) FROM audit_logs WHERE event_type LIKE 'mod_%';"
# Le compteur doit décroître de 1 à chaque suppression réussie.
```

## Notes

- Côté frontend (`useInfractions.ts`), un fallback silencieux sur 404
  rafraîchit le journal au lieu d'afficher une erreur. Cela reste utile
  même après le fix backend (cas d'admins concurrents).
- `moderation_actions` peut être considérée comme dépréciée depuis Phase 4 :
  écrire dans `audit_logs` est déjà fait (cf. `manage_moderation_service`),
  reste à drop la table dans une migration de cleanup.
