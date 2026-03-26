# Automod Bot

Bot d'auto-modération pour Discord. Il analyse chaque message en temps réel, détecte les contenus problématiques (spam, insultes, liens) et transmet les messages flaggés au backend DiscordSentinel pour décision.

Conformément à la philosophie du projet, le bot est une **interface légère** : il capture les événements et exécute les actions, mais c'est le backend qui prend les décisions.

---

## Stack technique

| Composant | Technologie |
|-----------|-------------|
| Langage | Rust (edition 2021) |
| Framework Discord | Serenity 0.12 |
| Runtime async | Tokio |
| Client HTTP | Reqwest 0.12 (rustls) |
| Sérialisation | Serde / serde_json |
| Détection patterns | Regex |
| Configuration | dotenvy (.env) |
| Logging | tracing + tracing-subscriber |

---

## Structure du projet

```
bots/automod-bot/
├── Cargo.toml                # Dépendances et métadonnées
├── .env.example              # Template des variables d'environnement
└── src/
    ├── main.rs               # Point d'entrée, initialisation du client Discord
    ├── config.rs             # Chargement de la configuration depuis .env
    ├── api_client.rs         # Client HTTP pour communiquer avec le backend
    ├── handler.rs            # Gestionnaire d'événements Discord (messages, ready)
    └── detectors/
        ├── mod.rs            # Orchestrateur : combine les résultats des détecteurs
        ├── spam.rs           # Détection de spam
        ├── insult.rs         # Détection d'insultes (FR + EN)
        └── link.rs           # Détection de liens et invitations Discord
```

---

## Flux de traitement

```
Message Discord reçu
        │
        ▼
  Le message vient d'un bot ?
        │
    Oui │   Non
    ────┘    │
  (ignoré)   ▼
        Analyse locale
    ┌───────────────────┐
    │  Spam ?           │
    │  Insulte ?        │
    │  Lien ?           │
    └───────────────────┘
        │
  Aucun flag levé ?
        │
    Oui │   Non
    ────┘    │
  (ignoré)   ▼
        POST /analyze → Backend
        │
        ▼
  Réponse du backend
  ┌─────────────────────────┐
  │ action: none            │  → Rien
  │ action: warn            │  → Réponse d'avertissement
  │ action: delete          │  → Suppression du message
  │ action: mute            │  → Suppression + timeout 10 min
  │ action: ban             │  → Ban de l'utilisateur
  └─────────────────────────┘
        │
  Backend injoignable ?
        │
    Oui │
        ▼
  Fallback : si insulte détectée → suppression du message
```

---

## Détecteurs

Les détecteurs effectuent une analyse locale rapide **avant** de solliciter le backend. Seuls les messages avec au moins un flag levé sont envoyés à l'API.

### Spam (`detectors/spam.rs`)

Détecte trois formes de spam :

| Règle | Condition | Exemple |
|-------|-----------|---------|
| Majuscules | Message >= 8 caractères, entièrement en majuscules | `ACHETE MON PRODUIT MAINTENANT` |
| Répétition de caractères | 6+ caractères identiques consécutifs | `aaaaaaa`, `hello!!!!!!` |
| Répétition de mots | 5+ mots, tous identiques | `buy buy buy buy buy` |

### Insultes (`detectors/insult.rs`)

Détecte les insultes via des expressions régulières case-insensitive, en français et en anglais.

**Mots détectés (français)** : connard, connasse, putain, merde, enculé, fdp, ntm, nique, bâtard, pd, pédé, salope, salopard, bordel, ta gueule, ferme-la, dégage.

**Mots détectés (anglais)** : fuck, shit, bitch, asshole, bastard, dickhead, cunt, stfu, idiot, moron, retard, dumbass.

Les patterns utilisent des regex avec gestion des accents (`[eé]`, `[aâ]`) et des variantes (espaces, tirets).

### Liens (`detectors/link.rs`)

Détecte les URLs et invitations Discord :

| Pattern | Exemple |
|---------|---------|
| `https?://...` | `https://example.com`, `http://malware.xyz` |
| `discord.gg/...` | `discord.gg/abc123` |
| `discord.com/invite/...` | `discord.com/invite/test` |

Note : les domaines sans protocole (ex: `example.com`) ne sont **pas** détectés volontairement pour éviter les faux positifs.

---

## Communication avec le backend

### Requête (`POST /analyze`)

Le bot envoie un JSON au backend pour chaque message flaggé :

```json
{
  "guild_id": "123456789",
  "channel_id": "987654321",
  "user_id": "111222333",
  "username": "pseudo",
  "content": "contenu du message",
  "flags": {
    "spam": false,
    "insult": true,
    "link": false
  },
  "metadata": {
    "message_id": "444555666",
    "timestamp": "2026-03-26T10:30:00.000Z"
  }
}
```

L'authentification est faite via un header `Authorization: Bearer <API_KEY>` si une clé API est configurée.

### Réponse

Le backend retourne l'action à exécuter :

```json
{
  "action": "delete",
  "reason": "Insulte détectée",
  "duration": null
}
```

| Champ | Type | Description |
|-------|------|-------------|
| `action` | `none` \| `warn` \| `delete` \| `mute` \| `ban` | Action à exécuter |
| `reason` | `string?` | Raison affichée à l'utilisateur (optionnel) |
| `duration` | `number?` | Durée en secondes (optionnel, réservé pour usage futur) |

---

## Actions exécutées

| Action | Comportement |
|--------|-------------|
| `none` | Aucune action |
| `warn` | Réponse au message avec un avertissement |
| `delete` | Suppression du message |
| `mute` | Suppression du message + timeout Discord de 10 minutes |
| `ban` | Ban de l'utilisateur (suppression des messages des dernières 24h) |

### Fallback (backend indisponible)

Si le backend ne répond pas, le bot applique une règle locale de sécurité :
- **Insulte détectée** : le message est supprimé
- **Autres flags** : aucune action (pour éviter les faux positifs)

---

## Configuration

### Variables d'environnement

Copier `.env.example` en `.env` et renseigner les valeurs :

| Variable | Obligatoire | Description | Défaut |
|----------|-------------|-------------|--------|
| `DISCORD_TOKEN` | Oui | Token du bot Discord | - |
| `API_BASE_URL` | Non | URL du backend DiscordSentinel | `http://localhost:3000` |
| `API_KEY` | Non | Clé API pour l'authentification | _(vide)_ |

### Intents Discord requis

Le bot nécessite deux intents privileged dans le portail développeur Discord :

- **GUILD_MESSAGES** : recevoir les événements de messages dans les serveurs
- **MESSAGE_CONTENT** : accéder au contenu textuel des messages

Ces intents doivent être activés sur la page de l'application dans le [Discord Developer Portal](https://discord.com/developers/applications), section **Bot > Privileged Gateway Intents**.

---

## Installation et lancement

### Prérequis

- Rust >= 1.75 (pour `LazyLock` en stable)
- Un token de bot Discord avec les intents activés
- Le backend DiscordSentinel lancé (ou le bot fonctionnera en mode fallback)

### Commandes

```bash
# Se placer dans le dossier du bot
cd bots/automod-bot

# Copier et configurer l'environnement
cp .env.example .env
# Editer .env avec votre token Discord

# Lancer en mode développement
cargo run

# Compiler en release
cargo build --release

# Lancer le binaire compilé
./target/release/automod-bot
```

### Lancer les tests

```bash
cargo test
```

Les 11 tests unitaires couvrent chaque détecteur (spam, insultes, liens) avec des cas positifs et négatifs.

---

## Permissions Discord requises

Le bot a besoin des permissions suivantes sur les serveurs où il opère :

| Permission | Raison |
|------------|--------|
| Read Messages | Recevoir les messages |
| Send Messages | Envoyer des avertissements |
| Manage Messages | Supprimer les messages flaggés |
| Moderate Members | Appliquer des timeouts (mute) |
| Ban Members | Bannir les utilisateurs |

---

## Logs

Le bot utilise `tracing` pour le logging structuré. Les événements loggés :

| Niveau | Événement |
|--------|-----------|
| `INFO` | Démarrage du bot, message flaggé, action exécutée |
| `WARN` | Backend injoignable (fallback activé) |
| `ERROR` | Erreur d'exécution d'une action, ApiClient manquant |

Exemple de sortie :

```
2026-03-26T10:30:00  INFO automod_bot::main: Démarrage de l'automod bot api_url=http://localhost:3000
2026-03-26T10:30:01  INFO automod_bot::handler: Automod bot connecté bot=AutomodBot
2026-03-26T10:30:15  INFO automod_bot::handler: Message flaggé guild_id=Some(123) user=pseudo flags.spam=false flags.insult=true flags.link=false
2026-03-26T10:30:15  INFO automod_bot::handler: Réponse du backend action=Delete reason=Some("Insulte")
2026-03-26T10:30:15  INFO automod_bot::handler: Message supprimé message_id=456
```
