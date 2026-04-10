# cleanup-bot

**Rôle** : Purge les logs système, infractions et audit logs de la base de données selon une rétention (X jours). Fournit aussi les commandes de purge rapide de messages Discord.

## Commandes / Events Discord principaux

- Slash `/cleanup logs` — purger les logs système > X jours
- Slash `/cleanup infractions` — purger infractions > X jours
- Slash `/cleanup audit` — purger audit_logs > X jours
- Slash `/purge last <N>` — supprimer les N derniers messages
- Slash `/purge user <@user>` — supprimer messages d'un utilisateur

## Dépendances externes

- API interne pour les purges DB
- Discord REST pour la suppression de messages
- Discord Gateway

## Modules clés

- `src/commands/cleanup.rs` — purges DB via API
- `src/commands/purge.rs` — purges Discord (messages récents)
- `src/handler.rs` — dispatch des slash commands

## Variables d'env

- `CLEANUP_DISCORD_TOKEN`
- `API_BASE_URL`
- `API_KEY`

## Cache Serenity (Phase 1)

**Tier : `minimal`** — pas d'accès au cache, uniquement des appels API + Discord REST.

## Note

À ne pas confondre avec le `cleanup-worker` (rétention DB automatique en background). Le `cleanup-bot` expose les mêmes fonctions en mode **manuel** via slash commands staff.
