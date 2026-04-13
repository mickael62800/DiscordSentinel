# Cleanup Worker — Extension avec jobs de réconciliation DB ↔ Discord

**Status** : proposition, à discuter
**Auteur** : review voice-bot 2026-04-13
**Scope** : étendre le `cleanup-worker` existant avec des jobs de réconciliation entre l'état DB et l'état Discord réel

---

## Contexte

Le code review de voice-bot a identifié 2 items critiques (C1, C2) liés aux incohérences entre la DB et l'état Discord :

- **C1** — Race condition création de salons temporaires : si le bot crash entre la création Discord et l'enregistrement DB, on a un salon orphelin côté Discord (existe mais inconnu du bot).
- **C2** — Channels orphelins après crash / intervention manuelle : inverse de C1, le bot a une ligne DB active pour un channel qui n'existe plus dans Discord (supprimé manuellement par un admin, crash pendant `check_and_delete_empty`, etc.).

Le même problème existe probablement pour d'autres bots :
- **ticket-bot** : tickets avec status `open` en DB mais channel Discord supprimé à la main
- **community-bot** : temp_roles expirés en DB mais rôle toujours assigné sur Discord
- **audit-bot** : catégories audit orphelines
- **moderation-bot** : actions en `pending` jamais résolues

---

## Décision : étendre `cleanup-worker`, pas créer un nouveau worker

### Pourquoi pas un nouveau worker
- Multiplication des process = overhead (process, logs, monitoring, supervision)
- Duplication du scheduler / config / DB pool
- Plus de points de failure

### Pourquoi `cleanup-worker`
- Il existe déjà avec un `scheduler.rs` fonctionnel
- Il a déjà accès à Postgres et à la config retention
- Son rôle est exactement ça : nettoyage périodique
- Ajouter un job = créer 1 fichier dans `services/workers/cleanup-worker/src/jobs/`

---

## Ce que fait cleanup-worker aujourd'hui

Fichiers existants :
- `jobs/cleanup_old_data.rs` : DELETE des vieilles lignes temporelles (voice_sessions, logs, audit_logs, ticket_messages, user_activity_log)
- `jobs/vacuum_tables.rs` : VACUUM ANALYZE sur les grosses tables
- `scheduler.rs` : boucle périodique qui invoque les jobs

**Limite actuelle** : il ne fait que du **cleanup temporel** (par retention). Pas de **réconciliation d'état**.

---

## Ce qu'il manque : jobs de réconciliation

### Prérequis technique

Le worker actuel ne parle qu'à Postgres. Pour checker l'état Discord, il a besoin de :
1. **Un token Discord** — soit réutiliser celui d'un bot existant (voice-bot ?), soit créer un token "infrastructure" dédié
2. **Un client HTTP Discord** — `reqwest` avec les bons headers, ou importer `serenity::http` en mode standalone (sans gateway)
3. **Rate limiting** — Discord limite à ~50 req/s par bot, le worker doit respecter ça (déjà géré par `serenity::http`)

### Approche recommandée

Utiliser `serenity::http::Http::new(token)` en standalone (pas de WebSocket), c'est le pattern le plus simple. Pas besoin de `Client::builder()`.

### Nouveau job 1 : `reconcile_voice_channels`

**Fréquence** : toutes les 15 minutes

**Algorithme** :
```
channels = SELECT * FROM voice_channels WHERE channel_status = 'open'

pour chaque channel :
    result = discord.get_channel(channel.channel_id)
    si result == 404 :
        # C2 : channel existe en DB mais pas dans Discord
        UPDATE voice_channels SET channel_status = 'closed' WHERE id = channel.id
        log info "reconciled: channel {id} marked closed (not found in Discord)"
        continue

    si result == Ok(ch) :
        # Check si le vocal est vide depuis trop longtemps
        voice_members = discord.get_voice_members(channel_id)
        si voice_members.is_empty() et channel.last_activity_at < NOW() - 1h :
            # Le bot a raté check_and_delete_empty
            discord.delete_channel(channel.channel_id)
            UPDATE voice_channels SET channel_status = 'closed' WHERE id = channel.id
            # Cleanup des channels associés (category, panels, queue)
            ...
```

**Protection** : si le cache Serenity n'est pas encore chargé ou si la guild est inaccessible (bot offline), **ne rien faire** pour éviter de marquer comme closed des channels valides.

### Nouveau job 2 : `reconcile_orphan_categories` (pour C1)

**Fréquence** : toutes les 30 minutes

**Algorithme** :
```
pour chaque guild configurée :
    categories = discord.get_guild_channels(guild_id).filter(type == Category)

    pour chaque category :
        # Critères pour détecter une catégorie orpheline créée par voice-bot
        si category.name.starts_with("Salon de ") ou category.name.starts_with("🎮 ") :
            # Check si elle est référencée dans voice_channels.category_id
            count = SELECT COUNT(*) FROM voice_channels
                    WHERE category_id = category.id AND channel_status = 'open'
            si count == 0 :
                # Catégorie orpheline → supprimer elle et ses enfants
                children = discord.get_channel_children(category.id)
                pour chaque child :
                    discord.delete_channel(child.id)
                discord.delete_channel(category.id)
                log info "reconciled: deleted orphan category {id}"
```

**Protection** : seulement supprimer si le nom match le pattern voice-bot (`Salon de ...` ou `🎮 ...`). Ne jamais toucher une catégorie avec un autre nom. Ignorer les catégories créées il y a moins de 5 min (évite les races avec le create_temp_channel en cours).

### Nouveau job 3 : `reconcile_tickets`

**Fréquence** : toutes les heures

**Algorithme** :
```
tickets = SELECT * FROM tickets WHERE status = 'open'

pour chaque ticket :
    result = discord.get_channel(ticket.channel_id)
    si result == 404 :
        UPDATE tickets SET status = 'closed', closed_at = NOW() WHERE id = ticket.id
        log info "reconciled: ticket {id} marked closed (channel gone)"
```

### Nouveau job 4 : `reconcile_temp_roles` (community-bot)

**Fréquence** : toutes les 15 minutes (complément au temp-roles-worker existant)

**Algorithme** :
```
# Vérifie que les lignes 'active' en DB correspondent bien à des roles assignés Discord
temp_roles = SELECT * FROM user_temp_roles WHERE status = 'active' AND expires_at > NOW()

pour chaque temp_role :
    member = discord.get_member(guild_id, user_id)
    si member == None :
        # User a quitté le serveur → marquer comme inactif
        UPDATE user_temp_roles SET status = 'revoked' WHERE id = temp_role.id
        continue
    si role_id not in member.roles :
        # Role non assigné Discord → soit re-assigner, soit marquer comme revoke
        # (choix de politique : re-assigner est plus safe)
        discord.add_role(guild_id, user_id, role_id)
        log warn "reconciled: re-added missing temp_role {role_id} to user {user_id}"
```

### Nouveau job 5 : `reconcile_moderation_pending`

**Fréquence** : toutes les 6 heures

**Algorithme** :
```
# Nettoyer les pending_mod_actions abandonnées (> 7 jours sans resolution)
UPDATE pending_mod_actions
SET status = 'expired'
WHERE status = 'pending' AND created_at < NOW() - INTERVAL '7 days'
```

---

## Configuration proposée

Ajouter à `cleanup-worker/src/config.rs` :

```rust
pub struct ReconcileConfig {
    /// Intervalle entre les jobs de réconciliation (secondes).
    /// Chaque job a son propre multiplicateur (voice=1x, tickets=4x, etc.)
    pub base_interval_secs: u64,  // default 900 (15 min)

    /// Token Discord pour les appels HTTP de réconciliation.
    /// Peut être réutilisé depuis VOICE_DISCORD_TOKEN ou dédié.
    pub discord_token: Option<String>,

    /// Activer/désactiver chaque job indépendamment
    pub reconcile_voice_enabled: bool,
    pub reconcile_tickets_enabled: bool,
    pub reconcile_orphan_categories_enabled: bool,
    pub reconcile_temp_roles_enabled: bool,
    pub reconcile_moderation_pending_enabled: bool,

    /// Minimum age avant qu'un channel soit considéré orphelin (évite les
    /// races avec le create_temp_channel en cours). Default 5 min.
    pub min_orphan_age_secs: u64,

    /// Voice channel vide depuis combien de temps avant force-delete
    pub voice_empty_force_delete_secs: u64,  // default 3600 (1h)
}
```

Variables d'env à ajouter dans `docker-compose.yml` :
```yaml
RECONCILE_DISCORD_TOKEN: ${VOICE_DISCORD_TOKEN}  # reuse existant
RECONCILE_VOICE_ENABLED: "true"
RECONCILE_TICKETS_ENABLED: "true"
RECONCILE_ORPHAN_CATEGORIES_ENABLED: "true"
RECONCILE_TEMP_ROLES_ENABLED: "true"
RECONCILE_MODERATION_PENDING_ENABLED: "true"
```

---

## Effort estimé

| Job | Effort dev | Risque |
|---|---|---|
| Setup `serenity::http` standalone + config | 1h | Bas |
| `reconcile_voice_channels` | 2h | Moyen (DELETE Discord) |
| `reconcile_orphan_categories` | 2h | Haut (DELETE agressif, doit être bien gardé) |
| `reconcile_tickets` | 30 min | Bas |
| `reconcile_temp_roles` | 1h | Moyen |
| `reconcile_moderation_pending` | 15 min | Bas (DB only) |
| Tests + docker compose | 1h | - |
| **TOTAL** | **~8h** | |

---

## Points d'attention

1. **Rate limiting Discord** : 50 req/s par bot. Pour 1000 salons voice à check toutes les 15 min, c'est OK (1.1 req/s). Mais pour `reconcile_orphan_categories` qui fetch TOUS les channels du guild, il faut du backoff.

2. **Sécurité** : les jobs qui DELETE dans Discord (orphan categories, force-delete voice empty) doivent être **très bien gardés**. Un bug qui supprime la mauvaise catégorie est catastrophique. Tests unitaires obligatoires avec mocks.

3. **Dry-run mode** : implémenter un flag `RECONCILE_DRY_RUN=true` qui log ce qui serait fait sans rien exécuter. Pour valider en prod avant d'activer.

4. **Métriques** : chaque job doit compter combien d'items il a réconciliés et logger en info. Sur Grafana on pourrait monitorer si un worker commence à réconcilier beaucoup d'items (signe d'un autre bug en amont).

5. **Ordre d'exécution** : si plusieurs jobs touchent les mêmes tables, serialiser. Ex : `reconcile_voice_channels` doit finir avant `reconcile_orphan_categories` sinon on peut supprimer une catégorie dont un salon vient d'être réconcilié.

---

## Alternative (simpler) — startup reconciliation par bot

Au lieu d'un worker, chaque bot pourrait faire sa propre reconciliation **au startup**. Code dans le `ready()` handler :

```rust
// Dans voice-bot/handler.rs ready()
let db_channels = api.list_channels(&guild_id).await?;
for ch in db_channels {
    if ctx.cache.channel(ch.channel_id).is_none() {
        api.close_channel(&ch.channel_id).await?;
    }
}
```

**Avantages** : plus simple, pas de nouveau process, fait au moment le plus utile (après un crash).
**Inconvénients** : seulement au startup, pas de protection contre les incohérences qui apparaissent en cours de journée (admin qui supprime manuellement). Chaque bot doit implémenter sa propre logique.

---

## Ma recommandation

**Combiner les 2 approches** :

1. **Startup reconciliation par bot** (rapide, simple) : chaque bot check ses propres ressources au `ready()`. Répare immédiatement les incohérences post-crash. Effort : ~30 min par bot.

2. **Cleanup-worker extended** (robuste, centralisé) : pour les incohérences qui apparaissent à l'usage (admins qui modifient manuellement, Discord qui supprime, race conditions). Effort : ~8h mais fait pour 5 bots d'un coup.

Priorité :
1. D'abord la startup reconciliation de voice-bot (C1+C2 résolus à 80%)
2. Puis le worker si on voit que des incohérences apparaissent en cours d'usage

---

## TODO si on décide d'y aller

- [ ] Décision : worker-only, startup-only, ou les deux ?
- [ ] Choix du token Discord pour le worker (réutiliser ou dédié ?)
- [ ] Implémenter dry-run mode en premier
- [ ] Tests unitaires avec mock Discord HTTP
- [ ] Métriques Prometheus pour monitoring
- [ ] Rollout progressif : 1 job à la fois, en dry-run d'abord, puis prod
- [ ] Alerte Slack/Discord si > X items réconciliés en une run (signal d'un autre bug)
