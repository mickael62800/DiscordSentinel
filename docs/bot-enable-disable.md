# Système d'activation/désactivation des bots et workers

## Ce qui a été implémenté (UI desktop)

### Composant utilisé
`AppToggle.vue` (atom existant) — toggle slider avec `v-model` boolean.

### `BotConfigPage.vue`
- **Toggle par bot-card** : chaque carte affiche un `AppToggle` en haut à droite.
  - Sauvegarde **immédiate** (sans passer par le bouton "Enregistrer").
  - Le clic sur le toggle ne sélectionne pas le bot (`@click.stop`).
  - La carte est grisée (`opacity: 0.5`) quand le bot est désactivé.
- **Champs boolean dans le formulaire** : les champs de type `"boolean"` dans le `config_schema` sont rendus comme `AppToggle` + label "Activé/Désactivé" au lieu d'un `<input>` texte.

### `WorkerConfigPage.vue`
- Même pattern que les bots : toggle par worker-card avec sauvegarde immédiate et grisage.

### Logique de stockage
| État | Action en DB |
|------|-------------|
| Activé (default) | Clé `enabled` **supprimée** (DB propre, le défaut implicite est toujours `true`) |
| Désactivé | Clé `enabled` sauvegardée avec la valeur `"false"` |

La clé `enabled` est stockée comme toutes les autres configs dans la table `bot_guild_config` via les commandes Tauri `set_bot_config` / `delete_bot_config`.

---

## Ce qui reste à faire

### ~~1. Migration SQL — ajouter `enabled` aux schémas (optionnel mais recommandé)~~ ✅ DONE
> Fichier : `services/api/migrations/043_add_enabled_to_bot_schemas.sql`


Ajouter le champ `enabled` dans le `config_schema` de chaque bot pour qu'il apparaisse aussi dans la section formulaire avec une description claire :

```sql
-- Exemple pour automod-bot
UPDATE bot_definitions SET config_schema = config_schema || '[
  {"key": "enabled", "label": "Bot actif", "type": "boolean", "required": false, "default": "true"}
]'::jsonb WHERE bot_name = 'automod-bot';
```

À faire pour : `automod-bot`, `moderation-bot`, `security-bot`, `progression-bot`, `ticket-bot`, `voice-bot`, `image-bot`, `moderation-worker`, `analytics-worker`.

> Sans cette migration, le toggle fonctionne quand même — il lit/écrit la clé `enabled` directement. La migration est utile pour que la clé apparaisse dans la liste des paramètres du formulaire.

### ~~2. Côté bots Rust — vérifier la clé `enabled` au démarrage et à chaque event~~ ✅ DONE

Chaque bot doit lire sa config au démarrage et vérifier `enabled` avant de traiter les events Discord.

**Pattern à appliquer dans chaque handler :**

```rust
// Dans le handler d'event (ex: message, member_join, etc.)
let config = self.api.get_guild_config(&guild_id).await?;

// Vérifier si le bot est activé pour ce serveur
if !config_bool(&config, "enabled", true) {
    return Ok(()); // Bot désactivé pour ce serveur, on skip
}

// ... traitement normal
```

Le helper `config_bool()` existe déjà dans `bots/shared/src/api_client.rs`.

**Bots concernés :**
- `bots/automod-bot/`
- `bots/moderation-bot/`
- `bots/security-bot/`
- `bots/progression-bot/`
- `bots/ticket-bot/`
- `bots/voice-bot/`
- `bots/image-bot/`

### ~~3. Côté workers Rust — vérifier `enabled` dans la boucle principale~~ ✅ DONE

Les workers tournent en boucle avec des intervalles. Il faut vérifier `enabled` à chaque itération **par guild** (ou une seule fois au démarrage si le worker est global).

```rust
// Dans la boucle du worker, pour chaque guild
let config = api.get_guild_config(&guild_id).await?;
if !config_bool(&config, "enabled", true) {
    continue; // Worker désactivé pour cette guild
}
```

**Workers concernés :**
- `services/workers/moderation-worker/`
- `services/workers/analytics-worker/`

### 4. (Optionnel) Indicateur d'état en temps réel sur les cards

Actuellement, les cards affichent uniquement l'état de la config (`enabled = true/false`). On pourrait croiser avec le statut heartbeat Redis (`bot:online:{bot_name}`) pour afficher un indicateur "En ligne / Hors ligne" distinct du toggle enable/disable.

Cela nécessiterait un endpoint API supplémentaire ou d'intégrer le statut online dans la réponse des définitions.
