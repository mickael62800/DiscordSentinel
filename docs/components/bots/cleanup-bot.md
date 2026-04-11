# cleanup-bot

**Rôle** : Purge les logs système, infractions et audit logs de la base de données selon une rétention (X jours). Fournit aussi les commandes de purge rapide de messages Discord.

## Commandes / Events Discord principaux

- Slash `/cleanup logs` — purger les logs système > X jours
- Slash `/cleanup infractions` — purger infractions > X jours
- Slash `/cleanup audit` — purger audit_logs > X jours
- Slash `/purge last <N>` — supprimer les N derniers messages
- Slash `/purge user <@user>` — supprimer messages d'un utilisateur

## API interne

**Statut Phase 7A : non migré gRPC.** Décision **architecturale** : cleanup-bot reste sur HTTP par design.

**HTTP `BaseApiClient`** :
- `DELETE /api/purge/infractions` — purge des infractions par âge
- `DELETE /api/purge/audit-logs` — purge des audit logs par âge
- `DELETE /api/purge/logs` — purge des logs système par âge

**Pourquoi pas gRPC ?** Bot d'admin/maintenance, **trafic très faible** (purges quotidiennes manuelles ou planifiées). Le coût de définir un `purge.proto`, d'écrire l'impl gRPC et de migrer le wrapper n'a pas de retour sur investissement quand il n'y a aucun problème de latence à régler. Exactement le cas d'usage où la **stratégie hybride** dit « reste sur HTTP ».

## Comportement si l'API tombe

- Les commandes slash répondent une erreur HTTP standard (`reqwest::Error` formaté).
- Aucun impact sur Discord lui-même : les commandes `/purge last`/`/purge user` (suppression de messages Discord) fonctionnent indépendamment de l'API via Discord REST.
- Pas de retry automatique — les purges sont idempotentes, l'utilisateur peut relancer manuellement.

## Modules clés

- `src/commands/cleanup.rs` — purges DB via API
- `src/commands/purge.rs` — purges Discord (messages récents)
- `src/handler.rs` — dispatch des slash commands
- `src/api_client.rs` — wrapper HTTP `BaseApiClient` (pas de gRPC)

## Variables d'env

- `CLEANUP_DISCORD_TOKEN`
- `API_BASE_URL`
- `API_KEY`

## Cache Serenity (Phase 1)

**Tier : `minimal`** — pas d'accès au cache, uniquement des appels API + Discord REST.

## Note

À ne pas confondre avec le `cleanup-worker` (rétention DB automatique en background). Le `cleanup-bot` expose les mêmes fonctions en mode **manuel** via slash commands staff.
