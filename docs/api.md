# Sentinel API

Backend central de DiscordSentinel. C'est le **cerveau** de la plateforme : il reçoit les messages flaggés par les bots, calcule un score de sévérité, décide de l'action à appliquer, et enregistre les infractions.

L'API **ne se connecte jamais à Discord**. Elle prend des décisions, ce sont les bots qui exécutent les sanctions.

---

## Stack technique

| Composant | Technologie |
|-----------|-------------|
| Langage | Rust (edition 2021) |
| Framework HTTP | Axum 0.8 |
| Runtime async | Tokio |
| Base de données | PostgreSQL 16 (via sqlx 0.8) |
| Cache | Redis 7 |
| Sérialisation | Serde / serde_json |
| Logging | tracing + tracing-subscriber |
| Auth | Bearer token |

---

## Architecture hexagonale

L'API suit une architecture hexagonale (ports & adapters) stricte :

```
                    ┌─────────────────────────────┐
                    │         DOMAIN               │
                    │  (zéro dépendance externe)   │
                    │                              │
                    │  Entities : Rule, Infraction │
                    │  Value Objects : Action,     │
                    │    DetectionFlags, FlagType   │
                    │  Services : ScoringService   │
                    └──────────┬──────────────────┘
                               │
              ┌────────────────┼────────────────┐
              │                │                │
     ┌────────▼──────┐ ┌──────▼───────┐ ┌──────▼──────┐
     │  PORTS         │ │ APPLICATION  │ │  PORTS       │
     │  Inbound       │ │ (use cases)  │ │  Outbound    │
     │  (traits)      │ │              │ │  (traits)    │
     └────────┬──────┘ └──────────────┘ └──────┬──────┘
              │                                 │
     ┌────────▼──────┐                 ┌───────▼───────┐
     │  ADAPTERS      │                │  ADAPTERS      │
     │  Inbound       │                │  Outbound      │
     │  (Axum HTTP)   │                │  (Postgres,    │
     │                │                │   Redis)       │
     └───────────────┘                 └───────────────┘
```

### Couches

| Couche | Rôle | Dépendances externes |
|--------|------|---------------------|
| **Domain** | Entités, value objects, scoring | Aucune (serde, uuid, chrono uniquement) |
| **Ports** | Traits (contrats) pour les use cases et repos | Aucune |
| **Application** | Orchestration : cache → DB → scoring → persistance | Aucune (utilise les ports) |
| **Adapters Inbound** | HTTP handlers, DTOs, auth middleware | Axum |
| **Adapters Outbound** | Implémentations PostgreSQL et Redis | sqlx, redis |

---

## Structure du projet

```
services/api/
├── Cargo.toml
├── Dockerfile
├── .env.example
├── migrations/
│   ├── 001_create_rules.sql
│   ├── 002_create_infractions.sql
│   └── 003_create_tickets.sql
└── src/
    ├── main.rs                              # Bootstrap et injection de dépendances
    ├── config.rs                            # Configuration depuis .env
    ├── domain/
    │   ├── entities/
    │   │   ├── rule.rs                      # Règle de modération par guild
    │   │   ├── infraction.rs                # Infraction enregistrée
    │   │   └── message_analysis.rs          # Résultat d'analyse retourné
    │   ├── value_objects/
    │   │   ├── action.rs                    # Enum : None | Warn | Delete | Mute | Ban
    │   │   ├── detection_flags.rs           # { spam, insult, link }
    │   │   └── flag_type.rs                 # Enum : Spam | Insult | Link
    │   ├── services/
    │   │   └── scoring_service.rs           # Logique de scoring pure (9 tests)
    │   └── errors.rs                        # DomainError
    ├── ports/
    │   ├── inbound/
    │   │   ├── analyze_message.rs           # AnalyzeMessageUseCase
    │   │   ├── manage_rules.rs              # ManageRulesUseCase
    │   │   └── manage_infractions.rs        # ManageInfractionsUseCase
    │   └── outbound/
    │       ├── rule_repository.rs           # Trait RuleRepository
    │       ├── infraction_repository.rs     # Trait InfractionRepository
    │       └── cache.rs                     # Trait CachePort
    ├── application/
    │   ├── analyze_message_service.rs       # Cache → DB → Score → Persist
    │   ├── manage_rules_service.rs          # CRUD rules + invalidation cache
    │   └── manage_infractions_service.rs    # Lecture infractions avec filtres
    └── adapters/
        ├── inbound/http/
        │   ├── router.rs                    # Définition des routes Axum
        │   ├── state.rs                     # AppState (Arc<dyn UseCase>)
        │   ├── errors.rs                    # DomainError → HTTP status
        │   ├── middleware/auth.rs            # Bearer token auth
        │   ├── handlers/
        │   │   ├── analyze.rs               # POST /analyze
        │   │   ├── rules.rs                 # GET/POST/DELETE /rules
        │   │   ├── infractions.rs           # GET /infractions
        │   │   └── health.rs                # GET /health
        │   └── dto/
        │       ├── analyze.rs               # AnalyzeRequestDto / AnalyzeResponseDto
        │       ├── rules.rs                 # CreateRuleDto / RuleResponseDto
        │       └── infractions.rs           # InfractionQueryParams / InfractionResponseDto
        └── outbound/
            ├── postgres/
            │   ├── rule_repository.rs       # PgRuleRepository (UPSERT)
            │   └── infraction_repository.rs # PgInfractionRepository
            └── redis_cache.rs               # Cache rules TTL 5 min
```

---

## Configuration

### Variables d'environnement

Copier `.env.example` en `.env` et renseigner les valeurs :

| Variable | Obligatoire | Description | Défaut |
|----------|-------------|-------------|--------|
| `DATABASE_URL` | Oui | URL de connexion PostgreSQL | - |
| `REDIS_URL` | Oui | URL de connexion Redis | - |
| `API_KEY` | Non | Clé Bearer pour l'authentification | _(vide = auth désactivée)_ |
| `HOST` | Non | Adresse d'écoute | `0.0.0.0` |
| `PORT` | Non | Port d'écoute | `3000` |
| `RUST_LOG` | Non | Filtre de logs | `sentinel_api=info,tower_http=debug` |

### Authentification

Toutes les routes sauf `/health` sont protégées par un Bearer token.

- Si `API_KEY` est vide ou absent : l'auth est **désactivée** (mode dev)
- Si `API_KEY` est configuré : chaque requête doit inclure le header `Authorization: Bearer <API_KEY>`
- Réponse en cas d'échec : `401 Unauthorized`

---

## Endpoints

L'API expose deux groupes de routes :

- **Routes Bots** (`/analyze`) : consommées par les bots Discord
- **Routes App** (`/api/...`) : consommées par l'app desktop et le dashboard

Toutes les routes (sauf `/health`) sont protégées par Bearer token.

---

### `GET /health`

Healthcheck (non protégé).

**Réponse** `200` :

```json
{
  "status": "ok"
}
```

---

## Routes Bots

### `POST /analyze`

Endpoint principal. Reçoit un message flaggé par un bot, calcule le score, décide l'action, enregistre l'infraction.

**Requête** :

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
    "timestamp": "2026-03-26T10:30:00.000000+00:00"
  }
}
```

**Réponse** `200` :

```json
{
  "action": "delete",
  "reason": "Détection : insult (score: 5.0)",
  "duration": null
}
```

| Champ | Type | Description |
|-------|------|-------------|
| `action` | `"none"` \| `"warn"` \| `"delete"` \| `"mute"` \| `"ban"` | Action que le bot doit exécuter |
| `reason` | `string` | Raison lisible (flags + score). Absent si action = none |
| `duration` | `number \| null` | Durée en secondes (600 pour mute, null sinon) |

---

## Routes App — Dashboard

### `GET /api/stats`

Statistiques globales pour le dashboard.

**Réponse** `200` :

```json
{
  "total_servers": 12,
  "total_users": 4850,
  "messages_today": 23419,
  "infractions_today": 17,
  "bots_online": 3,
  "bots_total": 4
}
```

| Champ | Type | Description |
|-------|------|-------------|
| `total_servers` | `number` | Nombre de serveurs gérés |
| `total_users` | `number` | Nombre total d'utilisateurs |
| `messages_today` | `number` | Messages analysés aujourd'hui |
| `infractions_today` | `number` | Infractions enregistrées aujourd'hui |
| `bots_online` | `number` | Bots actuellement connectés |
| `bots_total` | `number` | Nombre total de bots configurés |

---

## Routes App — Logs

### `GET /api/logs`

Liste les logs d'activité de tous les bots.

**Réponse** `200` :

```json
[
  {
    "id": "log-001",
    "timestamp": "2026-03-26 10:32:15",
    "level": "warn",
    "bot": "Automod",
    "server": "Mon Serveur",
    "message": "Message supprimé — insulte détectée (score: 5.0)"
  }
]
```

| Champ | Type | Description |
|-------|------|-------------|
| `id` | `string` | Identifiant unique du log |
| `timestamp` | `string` | Date/heure (`YYYY-MM-DD HH:mm:ss`) |
| `level` | `"info"` \| `"warn"` \| `"error"` | Niveau de sévérité |
| `bot` | `string` | Nom du bot source |
| `server` | `string` | Nom du serveur Discord |
| `message` | `string` | Description de l'événement |

---

## Routes App — Infractions

### `GET /api/infractions`

Liste toutes les infractions (tous serveurs confondus).

**Réponse** `200` :

```json
[
  {
    "id": "660e8400-e29b-41d4-a716-446655440001",
    "user_id": "111222333",
    "username": "pseudo",
    "server": "Mon Serveur",
    "infraction_type": "mute",
    "reason": "Détection : spam, insult (score: 8.0)",
    "created_at": "2026-03-26T10:30:00+00:00",
    "moderator": "Automod"
  }
]
```

| Champ | Type | Description |
|-------|------|-------------|
| `id` | `string` | Identifiant unique |
| `user_id` | `string` | ID Discord de l'utilisateur |
| `username` | `string` | Pseudo |
| `server` | `string` | Nom du serveur |
| `infraction_type` | `"warn"` \| `"mute"` \| `"ban"` | Type d'infraction |
| `reason` | `string` | Raison de la sanction |
| `created_at` | `string` | Date ISO 8601 |
| `moderator` | `string` | Qui a appliqué la sanction (bot ou humain) |

---

### `GET /infractions/{guild_id}`

Liste les infractions d'un serveur spécifique avec filtres et pagination.

**Query params** :

| Param | Type | Défaut | Description |
|-------|------|--------|-------------|
| `user_id` | `string` | - | Filtrer par utilisateur |
| `action` | `string` | - | Filtrer par action |
| `limit` | `number` | `50` | Max résultats (max 200) |
| `offset` | `number` | `0` | Décalage pagination |

**Exemple** : `GET /infractions/123456789?user_id=111222333&action=mute&limit=10`

**Réponse** `200` :

```json
[
  {
    "id": "660e8400-e29b-41d4-a716-446655440001",
    "guild_id": "123456789",
    "channel_id": "987654321",
    "user_id": "111222333",
    "username": "pseudo",
    "message_id": "444555666",
    "content": "contenu du message original",
    "score": 8.0,
    "action": "mute",
    "reason": "Détection : spam, insult (score: 8.0)",
    "duration": 600,
    "created_at": "2026-03-26T10:30:00+00:00"
  }
]
```

---

## Routes App — Rules

### `GET /api/rules`

Liste toutes les règles de modération (tous serveurs confondus). Format adapté pour l'app desktop.

**Réponse** `200` :

```json
[
  {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "name": "Anti-Spam",
    "enabled": true,
    "rule_type": "content_filter",
    "action": "mute",
    "description": "Détecte les messages spam (majuscules, répétitions). Poids: 3.0"
  }
]
```

| Champ | Type | Description |
|-------|------|-------------|
| `id` | `string` | Identifiant unique |
| `name` | `string` | Nom lisible de la règle |
| `enabled` | `boolean` | Règle active ou non |
| `rule_type` | `string` | Type : `rate_limit`, `join_rate`, `content_filter` |
| `action` | `string` | Action par défaut : `warn`, `mute`, `delete`, `ban` |
| `description` | `string` | Description de la règle |

---

### `PATCH /api/rules/{id}`

Active ou désactive une règle.

**Requête** :

```json
{
  "enabled": false
}
```

**Réponse** `200` : succès implicite (le client renvoie la valeur envoyée).

---

### `GET /rules/{guild_id}`

Liste les règles de scoring d'un serveur (format technique pour les bots/API interne).

**Réponse** `200` :

```json
[
  {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "guild_id": "123456789",
    "flag_type": "insult",
    "weight": 5.0,
    "threshold_warn": 2.0,
    "threshold_delete": 4.0,
    "threshold_mute": 6.0,
    "threshold_ban": 9.0,
    "enabled": true,
    "created_at": "2026-03-26T10:00:00+00:00",
    "updated_at": "2026-03-26T10:00:00+00:00"
  }
]
```

---

### `POST /rules`

Crée ou met à jour une règle de scoring (UPSERT sur `guild_id + flag_type`).

**Requête** :

```json
{
  "guild_id": "123456789",
  "flag_type": "spam",
  "weight": 4.0,
  "threshold_warn": 2.0,
  "threshold_delete": 4.0,
  "threshold_mute": 6.0,
  "threshold_ban": 9.0,
  "enabled": true
}
```

| Champ | Type | Obligatoire | Description |
|-------|------|-------------|-------------|
| `guild_id` | `string` | Oui | ID du serveur Discord |
| `flag_type` | `"spam"` \| `"insult"` \| `"link"` | Oui | Type de détection |
| `weight` | `number` | Oui | Poids dans le score (>= 0) |
| `threshold_warn` | `number` | Oui | Seuil avertissement |
| `threshold_delete` | `number` | Oui | Seuil suppression |
| `threshold_mute` | `number` | Oui | Seuil mute |
| `threshold_ban` | `number` | Oui | Seuil ban |
| `enabled` | `boolean` | Non | Défaut: `true` |

**Validation** :
- `weight` >= 0
- Seuils strictement croissants : `warn < delete < mute < ban`

**Réponse** `200` : la règle créée/mise à jour (même format que `GET /rules/{guild_id}`)

**Erreur** `400` :

```json
{
  "error": "Les seuils doivent être croissants : warn < delete < mute < ban"
}
```

---

### `DELETE /rules/{guild_id}/{rule_id}`

Supprime une règle. Le serveur revient aux valeurs par défaut pour ce flag.

**Réponse** `200` :

```json
{
  "deleted": true
}
```

**Erreur** `404` :

```json
{
  "error": "Règle introuvable : 550e8400-..."
}
```

---

## Routes App — Tickets

### `GET /api/tickets`

Liste tous les tickets de support.

**Réponse** `200` :

```json
[
  {
    "id": "ticket-001",
    "title": "Utilisateur signalé pour harcèlement",
    "status": "open",
    "priority": "high",
    "author_id": "111222333",
    "author_name": "pseudo",
    "assigned_to": null,
    "server": "Mon Serveur",
    "category": "report",
    "created_at": "2026-03-26T09:00:00+00:00",
    "updated_at": "2026-03-26T10:15:00+00:00",
    "messages_count": 3
  }
]
```

| Champ | Type | Description |
|-------|------|-------------|
| `id` | `string` | Identifiant unique |
| `title` | `string` | Titre du ticket |
| `status` | `"open"` \| `"pending"` \| `"closed"` | Statut |
| `priority` | `"urgent"` \| `"high"` \| `"medium"` \| `"low"` | Priorité |
| `author_id` | `string` | ID Discord de l'auteur |
| `author_name` | `string` | Pseudo de l'auteur |
| `assigned_to` | `string \| null` | Modérateur assigné |
| `server` | `string` | Nom du serveur |
| `category` | `"permissions"` \| `"report"` \| `"appeal"` \| `"bug"` \| `"suggestion"` | Catégorie |
| `created_at` | `string` | Date de création ISO 8601 |
| `updated_at` | `string` | Date de dernière mise à jour ISO 8601 |
| `messages_count` | `number` | Nombre de messages dans le ticket |

---

### `GET /api/tickets/{id}`

Détail d'un ticket avec tous ses messages.

**Réponse** `200` :

```json
{
  "ticket": {
    "id": "ticket-001",
    "title": "Utilisateur signalé pour harcèlement",
    "status": "open",
    "priority": "high",
    "author_id": "111222333",
    "author_name": "pseudo",
    "assigned_to": "modo42",
    "server": "Mon Serveur",
    "category": "report",
    "created_at": "2026-03-26T09:00:00+00:00",
    "updated_at": "2026-03-26T10:15:00+00:00",
    "messages_count": 3
  },
  "messages": [
    {
      "id": "msg-001",
      "ticket_id": "ticket-001",
      "author_name": "pseudo",
      "author_role": "user",
      "content": "Je signale un utilisateur qui insulte tout le monde.",
      "created_at": "2026-03-26T09:00:00+00:00"
    },
    {
      "id": "msg-002",
      "ticket_id": "ticket-001",
      "author_name": "modo42",
      "author_role": "moderator",
      "content": "Merci pour le signalement, on s'en occupe.",
      "created_at": "2026-03-26T09:15:00+00:00"
    }
  ]
}
```

| Champ (message) | Type | Description |
|-----------------|------|-------------|
| `id` | `string` | Identifiant du message |
| `ticket_id` | `string` | ID du ticket parent |
| `author_name` | `string` | Pseudo de l'auteur |
| `author_role` | `"user"` \| `"moderator"` \| `"admin"` | Rôle de l'auteur |
| `content` | `string` | Contenu du message |
| `created_at` | `string` | Date ISO 8601 |

---

### `POST /api/tickets/{id}/messages`

Ajoute une réponse à un ticket.

**Requête** :

```json
{
  "content": "Nous avons mute l'utilisateur pour 24h."
}
```

**Réponse** `200` : succès.

---

### `PATCH /api/tickets/{id}/close`

Ferme un ticket.

**Requête** : aucun body.

**Réponse** `200` : succès.

---

### `PATCH /api/tickets/{id}/assign`

Assigne un ticket à un modérateur.

**Requête** :

```json
{
  "assignee": "modo42"
}
```

**Réponse** `200` : succès.

---

## WebSocket — Events temps réel

### `GET /ws`

Connexion WebSocket pour recevoir les events en temps réel. Utilisé par l'app desktop.

**Authentification** : via query param `?token=<API_KEY>` (même clé que le Bearer token HTTP).

**URL** : `ws://localhost:3000/ws?token=your_api_key`

Si `API_KEY` est vide (dev mode), la connexion est acceptée sans token.

---

### Format des events

Chaque event est un message JSON texte :

```json
{
  "event": "event_type",
  "data": { ... }
}
```

### Events disponibles

| Event | Déclenché quand | Data |
|-------|-----------------|------|
| `infraction_new` | `/analyze` retourne une action != `none` | `{ "username", "action", "reason" }` |
| `ticket_new` | `POST /api/tickets` crée un ticket | `{ "id", "title", "author_name", "priority" }` |
| `ticket_message` | `POST /api/tickets/{id}/messages` | `{ "ticket_id", "author_name" }` |
| `ticket_closed` | `PATCH /api/tickets/{id}/close` | `{ "ticket_id" }` |
| `ticket_assigned` | `PATCH /api/tickets/{id}/assign` | `{ "ticket_id", "assignee" }` |

### Exemple de session WebSocket

```
Client → Server: upgrade HTTP → WebSocket (GET /ws?token=xxx)
Server → Client: {"event":"infraction_new","data":{"username":"ToxicUser","action":"mute","reason":"Détection : spam, insult (score: 8.0)"}}
Server → Client: {"event":"ticket_new","data":{"id":"abc-123","title":"Report user","author_name":"Alice","priority":"high"}}
Server → Client: {"event":"ticket_message","data":{"ticket_id":"abc-123","author_name":"moderator"}}
```

### Détails techniques

- **Broadcast** : tous les clients WebSocket connectés reçoivent les mêmes events
- **Buffer** : 256 events en mémoire (les clients lents qui prennent du retard perdront les events les plus anciens)
- **Ping/Pong** : géré automatiquement
- **Reconnexion** : à la charge du client (l'app desktop se reconnecte automatiquement toutes les 5 secondes)

---

## Algorithme de scoring

Le scoring est la logique coeur de l'API. Il est implémenté dans `domain/services/scoring_service.rs`, sans aucune dépendance externe.

### Fonctionnement

```
Flags actifs du message (spam, insult, link)
                │
                ▼
   Pour chaque flag actif :
   ┌──────────────────────────────────────┐
   │  Règle custom pour ce guild+flag ?   │
   │  Oui → utiliser rule.weight          │
   │  Non → utiliser le poids par défaut  │
   └──────────────────────────────────────┘
                │
                ▼
        Sommer les poids → score total
                │
                ▼
   Comparer aux seuils (du plus sévère au moins)
   ┌──────────────────────────────────────┐
   │  score >= ban    → Ban               │
   │  score >= mute   → Mute (10 min)     │
   │  score >= delete  → Delete            │
   │  score >= warn   → Warn              │
   │  sinon           → None              │
   └──────────────────────────────────────┘
```

### Poids par défaut

| Flag | Poids |
|------|-------|
| Spam | 3.0 |
| Insult | 5.0 |
| Link | 1.0 |

### Seuils par défaut

| Action | Seuil |
|--------|-------|
| Warn | >= 2.0 |
| Delete | >= 4.0 |
| Mute | >= 6.0 |
| Ban | >= 9.0 |

### Exemples de scoring

| Flags détectés | Score | Action |
|----------------|-------|--------|
| link | 1.0 | None |
| spam | 3.0 | Warn |
| insult | 5.0 | Delete |
| spam + insult | 8.0 | Mute |
| spam + insult + link | 9.0 | Ban |
| link + spam | 4.0 | Delete |

### Personnalisation par guild

En créant des règles via `POST /rules`, un admin peut :

- **Augmenter la sévérité** : mettre le poids de `link` à 5.0 pour un serveur anti-pub
- **Baisser la sévérité** : mettre le poids de `spam` à 1.0 pour un serveur tolérant
- **Ajuster les seuils** : baisser le seuil de ban à 7.0 pour un serveur strict
- **Désactiver un flag** : mettre `enabled: false` sur la règle `spam`

Quand une règle est désactivée, le poids par défaut du flag est utilisé.

---

## Cache Redis

Les règles sont mises en cache dans Redis pour éviter de requêter PostgreSQL à chaque appel `/analyze`.

| Clé | Valeur | TTL |
|-----|--------|-----|
| `rules:{guild_id}` | JSON des règles du serveur | 5 minutes |

Le cache est **automatiquement invalidé** quand une règle est créée, modifiée ou supprimée via `POST /rules` ou `DELETE /rules`.

---

## Base de données

### Table `rules`

| Colonne | Type | Description |
|---------|------|-------------|
| `id` | `UUID` PK | Identifiant unique |
| `guild_id` | `TEXT` | ID du serveur Discord |
| `flag_type` | `TEXT` | `spam`, `insult` ou `link` |
| `weight` | `DOUBLE` | Poids dans le calcul du score |
| `threshold_warn` | `DOUBLE` | Seuil avertissement |
| `threshold_delete` | `DOUBLE` | Seuil suppression |
| `threshold_mute` | `DOUBLE` | Seuil mute |
| `threshold_ban` | `DOUBLE` | Seuil ban |
| `enabled` | `BOOLEAN` | Règle active ou non |
| `created_at` | `TIMESTAMPTZ` | Date de création |
| `updated_at` | `TIMESTAMPTZ` | Date de dernière modification |

Contrainte unique sur `(guild_id, flag_type)` : une seule règle par type de flag par serveur.

### Table `infractions`

| Colonne | Type | Description |
|---------|------|-------------|
| `id` | `UUID` PK | Identifiant unique |
| `guild_id` | `TEXT` | ID du serveur Discord |
| `channel_id` | `TEXT` | ID du salon |
| `user_id` | `TEXT` | ID de l'utilisateur |
| `username` | `TEXT` | Pseudo de l'utilisateur |
| `message_id` | `TEXT` | ID du message Discord |
| `content` | `TEXT` | Contenu du message |
| `flags` | `JSONB` | Flags de détection `{spam, insult, link}` |
| `score` | `DOUBLE` | Score calculé |
| `action` | `TEXT` | Action décidée |
| `reason` | `TEXT` | Raison générée |
| `duration` | `BIGINT` | Durée en secondes (nullable) |
| `created_at` | `TIMESTAMPTZ` | Date de l'infraction |

Index sur `guild_id` et `(guild_id, user_id)` pour les requêtes de listing.

### Table `tickets`

| Colonne | Type | Description |
|---------|------|-------------|
| `id` | `UUID` PK | Identifiant unique |
| `title` | `TEXT` | Titre du ticket |
| `status` | `TEXT` | `open`, `pending`, `closed` |
| `priority` | `TEXT` | `urgent`, `high`, `medium`, `low` |
| `author_id` | `TEXT` | ID Discord de l'auteur |
| `author_name` | `TEXT` | Pseudo de l'auteur |
| `assigned_to` | `TEXT` | Modérateur assigné (nullable) |
| `server` | `TEXT` | Nom du serveur Discord |
| `category` | `TEXT` | `report`, `appeal`, `permissions`, `bug`, `suggestion` |
| `created_at` | `TIMESTAMPTZ` | Date de création |
| `updated_at` | `TIMESTAMPTZ` | Date de dernière mise à jour |

### Table `ticket_messages`

| Colonne | Type | Description |
|---------|------|-------------|
| `id` | `UUID` PK | Identifiant unique |
| `ticket_id` | `UUID` FK | Référence vers `tickets.id` (CASCADE) |
| `author_name` | `TEXT` | Pseudo de l'auteur |
| `author_role` | `TEXT` | `user`, `moderator`, `admin` |
| `content` | `TEXT` | Contenu du message |
| `created_at` | `TIMESTAMPTZ` | Date d'envoi |

Index sur `ticket_id` pour les requêtes de messages.

---

## Flux complet d'un appel `/analyze`

```
Bot POST /analyze
       │
       ▼
  Auth middleware (Bearer token)
       │
       ▼
  Handler : désérialise AnalyzeRequestDto
       │
       ▼
  AnalyzeMessageService (application)
       │
       ├── 1. Cache Redis : rules:{guild_id} ?
       │      Hit  → utiliser les règles cachées
       │      Miss → charger depuis PostgreSQL → mettre en cache (TTL 5 min)
       │
       ├── 2. ScoringService::score(flags, rules)
       │      Calcul pur : poids × flags → score → action
       │
       ├── 3. Créer et persister l'Infraction dans PostgreSQL
       │
       └── 4. Retourner MessageAnalysis { action, reason, duration }
       │
       ▼
  Handler : sérialise AnalyzeResponseDto → JSON
       │
       ▼
  Bot reçoit { action: "mute", reason: "...", duration: 600 }
       │
       ▼
  Bot exécute l'action sur Discord
```

---

## Codes d'erreur HTTP

| Code | Cas |
|------|-----|
| `200` | Succès |
| `400` | Règle invalide (poids négatif, seuils non croissants) |
| `401` | Token Bearer manquant ou invalide |
| `404` | Règle ou infraction introuvable |
| `500` | Erreur interne (DB, Redis) |

Format des erreurs :

```json
{
  "error": "Description de l'erreur"
}
```

---

## Installation et lancement

### Prérequis

- Rust >= 1.75
- PostgreSQL 16
- Redis 7

### Avec Docker Compose (recommandé)

```bash
# Depuis la racine du projet
cp .env.example .env
# Renseigner les variables

docker compose up -d postgres redis
docker compose up -d api
```

### En local (développement)

```bash
cd services/api
cp .env.example .env
# Renseigner DATABASE_URL et REDIS_URL

# Les migrations s'exécutent automatiquement au démarrage
cargo run
```

### Lancer les tests

```bash
cargo test
```

Les 9 tests unitaires couvrent le scoring :
- Aucun flag → None
- Chaque flag isolé → action attendue
- Combinaisons de flags → escalade
- Règles custom → override des poids
- Règle désactivée → retour au défaut
- Contenu de la reason

---

## Logs

Le logging utilise `tracing` avec filtrage par `RUST_LOG`.

| Niveau | Événement |
|--------|-----------|
| `INFO` | Démarrage, migrations, requêtes traitées |
| `DEBUG` | Détails HTTP (tower_http) |
| `ERROR` | Erreurs API, DB, Redis |

Exemple :

```
2026-03-26T10:00:00  INFO sentinel_api: Démarrage de Sentinel API addr=0.0.0.0:3000
2026-03-26T10:00:00  INFO sentinel_api: Migrations appliquées
2026-03-26T10:00:00  INFO sentinel_api: Sentinel API prêt
2026-03-26T10:00:15 DEBUG tower_http: POST /analyze 200 12ms
```
