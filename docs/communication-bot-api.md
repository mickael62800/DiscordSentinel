# Communication Bots — API

Document de synthèse expliquant comment les bots et l'API backend communiquent ensemble.

---

## Principe

Les bots et l'API ont des rôles distincts et ne partagent aucune dépendance directe :

| Composant | Rôle | Connexion Discord |
|-----------|------|-------------------|
| **Bot Automod** | Capte les messages, exécute les sanctions | Oui |
| **Bot Ticket** | Crée des threads, sync les messages | Oui |
| **Bot Security** | Détecte les raids, vérifie les comptes | Oui |
| **API Backend** | Cerveau — analyse, score, décide, persiste | Non |

Les bots sont les seuls à parler à Discord. L'API ne connaît pas Discord, elle reçoit des données brutes et retourne des décisions.

---

## Vue d'ensemble

```
┌─────────────┐  POST /analyze          ┌──────────────────┐
│ Automod Bot │ ──────────────────────▶  │                  │
└─────────────┘  ◀── { action, reason }  │                  │
                                         │                  │
┌─────────────┐  POST /api/tickets       │   API BACKEND    │
│ Ticket Bot  │ ──────────────────────▶  │                  │
└─────────────┘  PATCH .../close         │  PostgreSQL      │
                 POST .../messages       │  Redis           │
                                         │  WebSocket       │
┌─────────────┐  POST /api/security/     │                  │
│Security Bot │ ────── events ────────▶  │                  │
└─────────────┘                          └────────┬─────────┘
                                                  │ WebSocket
                                                  ▼
                                         ┌──────────────────┐
                                         │   App Desktop    │
                                         │   (temps réel)   │
                                         └──────────────────┘
```

---

## Flux complet

```
Discord (message d'un utilisateur)
       │
       ▼
┌──────────────────────────────────────┐
│           BOT AUTOMOD                │
│                                      │
│  1. Réception du message (Serenity)  │
│  2. Ignore si auteur = bot           │
│  3. Analyse locale rapide :          │
│     - Spam ? (caps, répétitions)     │
│     - Insulte ? (regex FR/EN)        │
│     - Lien ? (URLs, discord.gg)      │
│  4. Si aucun flag → STOP             │
│  5. Si flag(s) → envoyer à l'API    │
└──────────────┬───────────────────────┘
               │
               │  POST /analyze
               │  Authorization: Bearer <API_KEY>
               │  Content-Type: application/json
               │
               ▼
┌──────────────────────────────────────┐
│           API BACKEND                │
│                                      │
│  1. Auth middleware (Bearer token)   │
│  2. Charger les règles du guild :    │
│     - Cache Redis (TTL 5 min)        │
│     - Sinon PostgreSQL               │
│  3. Scoring :                        │
│     - Poids × flags actifs → score   │
│     - Score vs seuils → action       │
│  4. Enregistrer l'infraction (PG)    │
│  5. Retourner la décision            │
└──────────────┬───────────────────────┘
               │
               │  { action, reason, duration }
               │
               ▼
┌──────────────────────────────────────┐
│           BOT AUTOMOD                │
│                                      │
│  Exécute l'action sur Discord :      │
│  - none   → rien                     │
│  - warn   → reply avertissement      │
│  - delete → supprime le message      │
│  - mute   → supprime + timeout 10min │
│  - ban    → ban l'utilisateur        │
└──────────────────────────────────────┘
```

---

## Requête : Bot → API

**Endpoint** : `POST /analyze`

**Headers** :

```
Authorization: Bearer <API_KEY>
Content-Type: application/json
```

**Body** :

```json
{
  "guild_id": "123456789",
  "channel_id": "987654321",
  "user_id": "111222333",
  "username": "pseudo",
  "content": "ferme ta gueule espèce de connard",
  "flags": {
    "spam": false,
    "insult": true,
    "link": false
  },
  "metadata": {
    "message_id": "444555666",
    "timestamp": "2026-03-26T10:30:00.000000+00:00"
  }
}
```

### Détail des champs

| Champ | Source dans le bot | Description |
|-------|-------------------|-------------|
| `guild_id` | `msg.guild_id` | ID du serveur Discord |
| `channel_id` | `msg.channel_id` | ID du salon |
| `user_id` | `msg.author.id` | ID de l'auteur du message |
| `username` | `msg.author.name` | Pseudo de l'auteur |
| `content` | `msg.content` | Texte du message |
| `flags.spam` | `detectors::spam::detect()` | Majuscules, répétitions chars/mots |
| `flags.insult` | `detectors::insult::detect()` | Regex insultes FR + EN |
| `flags.link` | `detectors::link::detect()` | URLs http(s), discord.gg |
| `metadata.message_id` | `msg.id` | ID unique du message Discord |
| `metadata.timestamp` | `msg.timestamp` | Horodatage du message |

### Quand le bot envoie-t-il la requête ?

**Uniquement** si au moins un flag est `true`. Si les trois flags sont `false`, le bot ignore le message silencieusement et ne contacte jamais l'API.

Cela signifie que la grande majorité des messages ne génèrent aucun appel réseau.

---

## Réponse : API → Bot

**Status** : `200 OK`

```json
{
  "action": "delete",
  "reason": "Détection : insult (score: 5.0)",
  "duration": null
}
```

### Détail des champs

| Champ | Type | Description |
|-------|------|-------------|
| `action` | `string` | Action à exécuter : `none`, `warn`, `delete`, `mute`, `ban` |
| `reason` | `string?` | Raison lisible. Absent si `action = none` |
| `duration` | `number?` | Durée en secondes. Présent uniquement pour `mute` (défaut: 600) |

### Actions et ce que le bot fait

| Action | Comportement du bot | Permissions Discord requises |
|--------|--------------------|-----------------------------|
| `none` | Rien | - |
| `warn` | Reply au message avec la raison | Send Messages |
| `delete` | Supprime le message | Manage Messages |
| `mute` | Supprime le message + timeout 10 min | Manage Messages, Moderate Members |
| `ban` | Ban l'utilisateur (supprime messages des dernières 24h) | Ban Members |

---

## Scoring côté API

Le bot envoie les **flags bruts**, c'est l'API qui calcule le score et prend la décision.

### Poids par défaut

| Flag | Poids |
|------|-------|
| `spam` | 3.0 |
| `insult` | 5.0 |
| `link` | 1.0 |

### Seuils par défaut

| Score | Action |
|-------|--------|
| < 2.0 | `none` |
| >= 2.0 | `warn` |
| >= 4.0 | `delete` |
| >= 6.0 | `mute` |
| >= 9.0 | `ban` |

### Exemples concrets

| Message | Flags | Score | Action API | Le bot fait |
|---------|-------|-------|------------|-------------|
| `https://scam.xyz` | link | 1.0 | `none` | Rien |
| `ACHETE MON PRODUIT` | spam | 3.0 | `warn` | Reply avertissement |
| `t'es un connard` | insult | 5.0 | `delete` | Supprime le message |
| `FERME TA GUEULE` | spam + insult | 8.0 | `mute` | Supprime + timeout 10min |
| `fdp https://virus.com SPAM SPAM SPAM SPAM SPAM` | spam + insult + link | 9.0 | `ban` | Ban l'utilisateur |
| `Salut tout le monde !` | _(aucun)_ | - | _(pas d'appel)_ | Rien |

---

## Fallback : API indisponible

Si l'API ne répond pas (timeout, erreur réseau, 5xx), le bot applique une règle locale de sécurité :

| Flag détecté | Action fallback |
|-------------|-----------------|
| `insult = true` | Supprime le message |
| Autres flags | Rien (évite les faux positifs) |

Le bot log un warning :

```
WARN automod_bot::handler: Backend injoignable — action locale par défaut
```

---

## Configuration réseau

### En Docker (production)

```
Bot  ──POST──▶  http://api:3000/analyze
```

Le bot utilise le nom de service Docker `api` pour joindre le backend sur le réseau interne. Aucun port n'est exposé sur internet.

Variables d'environnement du bot (dans `docker-compose.yml`) :

```yaml
environment:
  API_BASE_URL: http://api:3000
  API_KEY: ${API_KEY}
```

### En local (développement)

```
Bot  ──POST──▶  http://localhost:3000/analyze
```

Variables dans `.env` du bot :

```
API_BASE_URL=http://localhost:3000
API_KEY=your_api_key_here
```

### Authentification

Si `API_KEY` est configuré côté API, le bot doit envoyer le même token dans le header :

```
Authorization: Bearer <API_KEY>
```

Si `API_KEY` est vide côté API (mode dev), l'authentification est désactivée et le bot peut appeler sans header.

---

## Contrat de compatibilité

Les deux services doivent rester alignés sur ces structures :

### `DetectionFlags` (partagé)

```
{
  spam: bool,
  insult: bool,
  link: bool
}
```

Défini dans :
- Bot : `src/detectors/mod.rs` → `DetectionFlags` (Serialize)
- API : `src/domain/value_objects/detection_flags.rs` → `DetectionFlags` (Deserialize)

### `Action` (partagé)

Valeurs possibles : `"none"`, `"warn"`, `"delete"`, `"mute"`, `"ban"`

Sérialisé en snake_case des deux côtés.

Défini dans :
- Bot : `src/api_client.rs` → `Action` enum (Deserialize, `rename_all = "snake_case"`)
- API : `src/domain/value_objects/action.rs` → `Action` enum (Serialize, `rename_all = "snake_case"`)

Le DTO de réponse de l'API sérialise l'action via `action.as_str()` en `String`, ce qui produit les mêmes valeurs snake_case que le bot attend.

### `metadata` (structure)

```
{
  message_id: String,
  timestamp: String
}
```

Défini dans :
- Bot : `src/api_client.rs` → `MessageMetadata` (Serialize)
- API : `src/adapters/inbound/http/dto/analyze.rs` → `MetadataDto` (Deserialize)

---

## Séquence temporelle

```
T+0ms     Discord envoie le message au bot (WebSocket gateway)
T+1ms     Bot reçoit l'événement message
T+1ms     Bot vérifie : auteur = bot ? → non, continuer
T+2ms     Bot analyse locale : spam=false, insult=true, link=false
T+2ms     Au moins un flag → préparer la requête
T+3ms     Bot envoie POST /analyze à l'API
T+5ms     API reçoit, auth OK
T+6ms     API charge les règles (cache Redis hit)
T+6ms     API calcule le score : 5.0 → action = delete
T+7ms     API persiste l'infraction dans PostgreSQL
T+8ms     API retourne { action: "delete", reason: "..." }
T+9ms     Bot reçoit la réponse
T+10ms    Bot supprime le message sur Discord
T+10ms    Bot log : "Message supprimé"
```

Latence totale estimée : **~10ms** en réseau local Docker.

---
---

# Ticket Bot — API

## Flux

```
Utilisateur : /ticket create "Harcèlement" report high
       │
       ▼
┌──────────────────────────────────────┐
│           TICKET BOT                 │
│  POST /api/tickets → Backend         │
└──────────────┬───────────────────────┘
               ▼
┌──────────────────────────────────────┐
│           API BACKEND                │
│  Crée le ticket en base (PG)        │
│  Broadcast WebSocket: ticket_new     │
│  Retourne { id, title, status, ... } │
└──────────────┬───────────────────────┘
               ▼
┌──────────────────────────────────────┐
│           TICKET BOT                 │
│  Crée un thread privé sur Discord   │
│  Envoie le message d'ouverture      │
└──────────────────────────────────────┘
```

## Endpoints utilisés

| Action bot | Méthode | Endpoint | Direction |
|------------|---------|----------|-----------|
| `/ticket create` | `POST` | `/api/tickets` | Bot → API |
| `/ticket close` | `PATCH` | `/api/tickets/{id}/close` | Bot → API |
| `/ticket assign` | `PATCH` | `/api/tickets/{id}/assign` | Bot → API |
| Message dans thread | `POST` | `/api/tickets/{id}/messages` | Bot → API |

## Requête : création de ticket

```json
{
  "title": "Utilisateur signalé pour harcèlement",
  "priority": "high",
  "author_id": "111222333",
  "author_name": "pseudo",
  "server": "Mon Serveur",
  "category": "report"
}
```

## Réponse

```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "title": "Utilisateur signalé pour harcèlement",
  "status": "open",
  "priority": "high",
  "author_id": "111222333",
  "author_name": "pseudo",
  "assigned_to": null,
  "server": "Mon Serveur",
  "category": "report",
  "created_at": "2026-03-26T09:00:00+00:00",
  "updated_at": "2026-03-26T09:00:00+00:00",
  "messages_count": 0
}
```

## Sync des messages

Chaque message envoyé dans un thread `ticket-*` est automatiquement transmis au backend via `POST /api/tickets/{id}/messages`. Le backend met à jour `updated_at` et passe le statut à `pending`.

## Actions Discord après réponse API

| Route | Ce que le bot fait sur Discord |
|-------|-------------------------------|
| `POST /api/tickets` | Crée un thread privé `ticket-{id}` |
| `PATCH .../close` | Archive + verrouille le thread |
| `PATCH .../assign` | Ajoute le modérateur au thread |

---
---

# Security Bot — API

## Flux anti-raid

```
10 utilisateurs rejoignent en 10 secondes
       │
       ▼
┌──────────────────────────────────────┐
│          SECURITY BOT                │
│                                      │
│  1. guild_member_addition × 10      │
│  2. RaidDetector : seuil atteint    │
│  3. POST /api/security/events       │
│  4. Active lockdown Discord         │
│  5. Alerte dans un salon            │
└──────────────┬───────────────────────┘
               ▼
┌──────────────────────────────────────┐
│           API BACKEND                │
│                                      │
│  1. Persiste l'événement (PG)       │
│  2. Broadcast WebSocket:             │
│     security_event (critical)        │
└──────────────────────────────────────┘
               ▼
         App Desktop reçoit l'alerte en temps réel
```

## Flux compte suspect

```
Nouveau membre avec un compte de 2h
       │
       ▼
┌──────────────────────────────────────┐
│          SECURITY BOT                │
│                                      │
│  1. guild_member_addition            │
│  2. AccountChecker : compte < 24h   │
│  3. POST /api/security/events       │
└──────────────┬───────────────────────┘
               ▼
┌──────────────────────────────────────┐
│           API BACKEND                │
│  Persiste + broadcast WebSocket      │
└──────────────────────────────────────┘
```

## Endpoint utilisé

| Action | Méthode | Endpoint |
|--------|---------|----------|
| Signaler un événement | `POST` | `/api/security/events` |
| Lister les événements | `GET` | `/api/security/events` |
| Filtrer par guild | `GET` | `/api/security/events?guild_id=123` |

## Requête : signalement d'événement

```json
{
  "guild_id": "123456789",
  "event_type": "raid_detected",
  "severity": "critical",
  "description": "Raid détecté : 15 joins en quelques secondes",
  "user_ids": ["111", "222", "333"]
}
```

## Types d'événements

| `event_type` | `severity` | Déclencheur |
|-------------|-----------|-------------|
| `raid_detected` | `critical` | X joins en Y secondes (configurable) |
| `suspicious_account` | `warning` | Compte créé il y a moins de 24h |

## Actions Discord par le bot

| Événement | Actions Discord |
|-----------|----------------|
| Raid détecté | Passe le serveur en vérification `Higher` + alerte dans un salon texte |
| Compte suspect | Log uniquement (action future via l'API) |

## WebSocket broadcast

L'API broadcast un event `security_event` sur le WebSocket pour que l'app desktop affiche l'alerte en temps réel :

```json
{
  "event": "security_event",
  "data": {
    "guild_id": "123456789",
    "event_type": "raid_detected",
    "severity": "critical",
    "description": "Raid détecté : 15 joins en quelques secondes"
  }
}
```

---
---

# Réseau et authentification (commun)

## En Docker

Tous les bots utilisent le réseau Docker interne pour joindre l'API :

```
automod-bot   ──▶  http://api:3000/analyze
ticket-bot    ──▶  http://api:3000/api/tickets
security-bot  ──▶  http://api:3000/api/security/events
```

## En local

```
Tous les bots  ──▶  http://localhost:3000/...
```

## Authentification

Identique pour tous les bots : header `Authorization: Bearer <API_KEY>`.

Si `API_KEY` est vide côté API (mode dev), l'auth est désactivée.
