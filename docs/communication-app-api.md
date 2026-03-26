# Communication App Desktop — API

Document de synthèse expliquant comment l'application bureau et l'API backend communiquent ensemble.

---

## Principe

L'app desktop et l'API ont des rôles distincts :

| Composant | Rôle | Connexion Discord |
|-----------|------|-------------------|
| **App Desktop** | Interface d'administration — visualise, configure, modère | Non (via OAuth2 pour l'auth utilisateur uniquement) |
| **API Backend** | Cerveau — stocke, analyse, décide | Non |

L'app desktop ne parle jamais à Discord directement. Elle consomme l'API backend via deux canaux :

| Canal | Usage | Direction |
|-------|-------|-----------|
| **HTTP** | Chargement de données, actions CRUD | App → API → App |
| **WebSocket** | Events temps réel (notifications, updates live) | API → App (push) |

---

## Architecture de communication

```
┌─────────────────────────────────────────────────┐
│              APP DESKTOP (Tauri)                 │
│                                                  │
│  ┌──────────────┐    ┌───────────────────────┐  │
│  │  ApiAdapter   │    │  RealtimeService       │  │
│  │  (reqwest)    │    │  (tokio-tungstenite)   │  │
│  │              │    │                        │  │
│  │  HTTP CRUD   │    │  WebSocket Events      │  │
│  └──────┬───────┘    └──────────┬────────────┘  │
│         │                       │                │
└─────────┼───────────────────────┼────────────────┘
          │                       │
          │  Authorization:       │  ?token=<API_KEY>
          │  Bearer <API_KEY>     │
          │                       │
          ▼                       ▼
┌─────────────────────────────────────────────────┐
│              API BACKEND (Axum)                  │
│                                                  │
│  ┌──────────────┐    ┌───────────────────────┐  │
│  │  HTTP Routes  │    │  /ws Endpoint          │  │
│  │  /api/*       │    │  (axum::ws)            │  │
│  └──────┬───────┘    └──────────┬────────────┘  │
│         │                       │                │
│         ▼                       ▼                │
│  ┌──────────────────────────────────────────┐   │
│  │          EventBroadcaster                 │   │
│  │     (tokio::sync::broadcast)              │   │
│  │     Quand un handler agit → broadcast     │   │
│  └───────────────────────────────────────────┘   │
│                                                  │
│  PostgreSQL    Redis                             │
└─────────────────────────────────────────────────┘
```

---

## Canal 1 : HTTP — Chargement et actions

### Authentification

Toutes les requêtes HTTP incluent le header :

```
Authorization: Bearer <API_KEY>
```

Si `API_KEY` est vide côté API (mode dev), l'authentification est désactivée.

La clé API est stockée en LMDB sur la machine de l'utilisateur (`%LOCALAPPDATA%/discord-sentinel/`).

---

### Routes consommées par l'app

#### Dashboard

| Action | Méthode | Route | Description |
|--------|---------|-------|-------------|
| Charger les stats | GET | `/api/stats` | Serveurs, users, messages, infractions, bots |

**Réponse** :

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

#### Logs

| Action | Méthode | Route | Description |
|--------|---------|-------|-------------|
| Charger les logs | GET | `/api/logs` | Logs d'activité de tous les bots |

**Réponse** :

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

#### Infractions

| Action | Méthode | Route | Description |
|--------|---------|-------|-------------|
| Charger les infractions | GET | `/api/infractions` | Toutes les infractions, tous serveurs |

**Réponse** :

```json
[
  {
    "id": "660e8400-...",
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

#### Rules

| Action | Méthode | Route | Description |
|--------|---------|-------|-------------|
| Charger les règles | GET | `/api/rules` | Toutes les règles (format app) |
| Activer/désactiver | PATCH | `/api/rules/{id}` | Toggle enabled |

**Toggle — Requête** :

```json
{ "enabled": false }
```

#### Tickets

| Action | Méthode | Route | Description |
|--------|---------|-------|-------------|
| Liste des tickets | GET | `/api/tickets` | Tous les tickets |
| Détail d'un ticket | GET | `/api/tickets/{id}` | Ticket + messages |
| Répondre | POST | `/api/tickets/{id}/messages` | Ajouter un message |
| Fermer | PATCH | `/api/tickets/{id}/close` | Passer en "closed" |
| Assigner | PATCH | `/api/tickets/{id}/assign` | Assigner un modérateur |

**Reply — Requête** :

```json
{ "content": "Nous avons mute l'utilisateur pour 24h." }
```

**Assign — Requête** :

```json
{ "assignee": "modo42" }
```

---

## Canal 2 : WebSocket — Events temps réel

### Connexion

```
ws://localhost:3000/ws?token=<API_KEY>
```

- L'app se connecte au WebSocket dès le login
- Auth via query param `?token=` (même clé que le Bearer HTTP)
- Si `API_KEY` est vide : connexion acceptée sans token
- Reconnexion automatique toutes les 5 secondes en cas de déconnexion

### Format des events

Chaque event est un message JSON texte :

```json
{
  "event": "event_type",
  "data": { ... }
}
```

### Events reçus par l'app

| Event | Déclenché par | Data | Sévérité notification |
|-------|---------------|------|-----------------------|
| `infraction_new` | `POST /analyze` (action ≠ none) | `{ "username", "action", "reason" }` | high si ban, medium sinon |
| `ticket_new` | `POST /api/tickets` | `{ "id", "title", "author_name", "priority" }` | high si urgent, medium sinon |
| `ticket_message` | `POST /api/tickets/{id}/messages` | `{ "ticket_id", "author_name" }` | low |
| `ticket_closed` | `PATCH /api/tickets/{id}/close` | `{ "ticket_id" }` | — |
| `ticket_assigned` | `PATCH /api/tickets/{id}/assign` | `{ "ticket_id", "assignee" }` | — |
| `bot_status` | _(futur : heartbeat bots)_ | `{ "bot", "online" }` | high si offline, low si online |
| `raid_detected` | _(futur : analyse raids)_ | `{ "message" }` | critical |

### Flux d'un event

```
Utilisateur Discord envoie un message toxique
       │
       ▼
Bot Automod → POST /analyze → API Backend
       │
       ▼
API décide : action = "mute"
       │
       ├── Persiste l'infraction en DB
       │
       ├── Retourne { action: "mute" } au bot (HTTP)
       │
       └── Broadcast WebSocket :
           { "event": "infraction_new", "data": { "username": "ToxicUser", "action": "mute", "reason": "..." } }
                │
                ▼
       App Desktop reçoit l'event via WebSocket
                │
                ├── Ajoute une notification in-app (panneau cloche)
                ├── Si sévérité high/critical → notification native OS (Windows toast)
                └── Indicateur sidebar mis à jour
```

---

## Ce qui passe par HTTP vs WebSocket

| Besoin | Canal | Pourquoi |
|--------|-------|----------|
| Charger une liste (stats, logs, tickets...) | HTTP GET | Données complètes, pagination possible |
| Action utilisateur (toggle rule, reply ticket...) | HTTP POST/PATCH | Requête-réponse, confirmation de succès |
| Nouvelle infraction détectée | WebSocket push | Temps réel, pas de polling |
| Nouveau ticket créé | WebSocket push | Notification immédiate |
| Message ajouté à un ticket | WebSocket push | Conversation live |
| Bot connecté/déconnecté | WebSocket push | Statut live |
| Raid détecté | WebSocket push | Alerte critique immédiate |

**Règle simple** : l'app _demande_ en HTTP, l'API _pousse_ en WebSocket.

---

## Gestion de la connexion

### Côté app desktop (Rust — RealtimeService)

```
App démarre
    │
    ├── Lit API config depuis LMDB (url + key)
    │
    ├── Connexion WebSocket → ws://{url}/ws?token={key}
    │     │
    │     ├── Succès → emit "ws:connected" vers le frontend Vue
    │     │
    │     └── Échec → retry dans 5 secondes
    │
    ├── Boucle de réception :
    │     │
    │     ├── Message texte → parse JSON → emit "ws:{event}" vers Vue
    │     ├── Ping → Pong
    │     └── Close / Erreur → reconnexion auto (5s)
    │
    └── Logout → ferme le WebSocket proprement
```

### Côté API (Rust — Axum WebSocket handler)

```
Client se connecte à /ws?token=xxx
    │
    ├── Vérification token (si API_KEY configuré)
    │     ├── OK → upgrade en WebSocket
    │     └── KO → 401 Unauthorized
    │
    ├── Subscribe au broadcast channel (tokio::broadcast)
    │
    ├── Boucle :
    │     ├── Event reçu du broadcast → forward au client en JSON
    │     ├── Ping reçu → Pong
    │     └── Close / Erreur → déconnexion propre
    │
    └── Client déconnecté → log info, cleanup automatique
```

---

## Sécurité

| Aspect | Implémentation |
|--------|---------------|
| Auth HTTP | Bearer token dans header `Authorization` |
| Auth WebSocket | Token dans query param `?token=` |
| Clé API | Même valeur pour HTTP et WS (`API_KEY` dans `.env`) |
| Stockage côté app | LMDB local (`%LOCALAPPDATA%/discord-sentinel/`) |
| Transport | HTTP/WS en clair en dev, HTTPS/WSS en production |
| Credentials Discord | Stockés en LMDB, jamais transmis à l'API backend |

---

## Configuration réseau

### En Docker (production)

```
App Desktop  ──HTTP──▶  https://api.sentinel.example.com/api/*
App Desktop  ──WS────▶  wss://api.sentinel.example.com/ws?token=xxx
```

### En local (développement)

```
App Desktop  ──HTTP──▶  http://localhost:3000/api/*
App Desktop  ──WS────▶  ws://localhost:3000/ws?token=xxx
```

L'URL est configurable dans l'app (Setup initial ou Settings).

---

## Contrat de compatibilité

### Entities partagées (même format JSON)

| Entité | App Desktop (TypeScript) | API (Rust DTO) |
|--------|-------------------------|----------------|
| ServerStats | `types/index.ts` | — (handler direct) |
| LogEntry | `types/index.ts` | — (handler direct) |
| Infraction | `types/index.ts` | `dto/infractions.rs` |
| ModerationRule | `types/index.ts` | `dto/rules.rs` |
| Ticket | `types/index.ts` | `dto/tickets.rs → TicketResponseDto` |
| TicketMessage | `types/index.ts` | `dto/tickets.rs → TicketMessageDto` |
| TicketDetail | `types/index.ts` | `dto/tickets.rs → TicketDetailDto` |

### WsEvent (format WebSocket)

```
{
  event: string,    // Nom de l'event (ex: "infraction_new")
  data: object      // Payload JSON libre
}
```

Défini dans :
- API : `adapters/inbound/ws/broadcaster.rs` → `WsEvent` (Serialize)
- App Desktop : `composables/useRealtime.ts` → `WsEvent` interface
- App Desktop Rust : `application/realtime_service.rs` → `WsEvent` (Deserialize)

---

## Séquence temporelle

### Chargement d'une page (HTTP)

```
T+0ms     User clique sur "Tickets" dans la sidebar
T+1ms     Vue Router navigue vers TicketsPage
T+1ms     Composable useTickets() → onMounted → invoke("get_tickets")
T+2ms     Tauri command → TicketsService → TicketsRepository (ApiAdapter)
T+3ms     ApiAdapter envoie GET /api/tickets (avec Bearer token)
T+15ms    API répond avec la liste JSON
T+16ms    ApiAdapter parse la réponse
T+17ms    Vue reçoit les données, render la liste
```

**Latence estimée : ~17ms** en réseau local.

### Notification temps réel (WebSocket)

```
T+0ms     Bot envoie POST /analyze → API
T+5ms     API décide action = "ban", persiste l'infraction
T+6ms     API broadcast WsEvent { event: "infraction_new", data: {...} }
T+6ms     API retourne la réponse HTTP au bot
T+7ms     WebSocket server forward l'event au client app desktop
T+8ms     Tauri RealtimeService reçoit le message WS
T+8ms     RealtimeService emit "ws:infraction_new" vers Vue
T+9ms     useNotifications() reçoit l'event, crée une notification in-app
T+9ms     Badge compteur dans la sidebar se met à jour
T+10ms    Notification native Windows toast apparaît (si sévérité high/critical)
```

**Latence estimée : ~10ms** entre l'action sur Discord et la notification sur le bureau de l'admin.
