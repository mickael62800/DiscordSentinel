# Ticket Bot

Bot de gestion de tickets pour Discord. Il permet aux utilisateurs de créer des tickets de support, signaler des problèmes, faire appel de sanctions, et aux modérateurs de gérer ces tickets.

Conformément à la philosophie du projet, le bot est une **interface légère** : il crée les threads Discord et transmet tout au backend API qui persiste et gère les données.

---

## Stack technique

| Composant | Technologie |
|-----------|-------------|
| Langage | Rust (edition 2021) |
| Framework Discord | Serenity 0.12 |
| Runtime async | Tokio |
| Client HTTP | Reqwest 0.12 (rustls) |
| Sérialisation | Serde / serde_json |
| Configuration | dotenvy (.env) |
| Logging | tracing + tracing-subscriber |

---

## Structure du projet

```
bots/ticket-bot/
├── Cargo.toml
├── Dockerfile
├── .dockerignore
├── .env.example
└── src/
    ├── main.rs              # Point d'entrée, init client Discord
    ├── config.rs            # Chargement config depuis .env
    ├── api_client.rs        # Client HTTP vers le backend (CRUD tickets)
    ├── handler.rs           # EventHandler : slash commands + sync messages
    └── commands/
        ├── mod.rs           # Enregistrement des slash commands
        └── ticket.rs        # /ticket create | close | assign
```

---

## Slash commands

### `/ticket create <title> <category> [priority]`

Crée un nouveau ticket.

| Paramètre | Obligatoire | Type | Description |
|-----------|-------------|------|-------------|
| `title` | Oui | string | Titre du ticket |
| `category` | Oui | choix | `report`, `appeal`, `permissions`, `bug`, `suggestion` |
| `priority` | Non | choix | `urgent`, `high`, `medium` (défaut), `low` |

**Ce qui se passe :**
1. Le bot envoie `POST /api/tickets` au backend avec les infos
2. Le backend crée le ticket en base et retourne l'ID
3. Le bot crée un **thread privé** nommé `ticket-{id}` dans le salon actuel
4. Le bot envoie un message d'ouverture dans le thread avec les détails
5. L'utilisateur reçoit une réponse éphémère avec le lien vers le thread

### `/ticket close`

Ferme le ticket du salon actuel (doit être utilisé dans un thread `ticket-*`).

**Ce qui se passe :**
1. Le bot envoie `PATCH /api/tickets/{id}/close` au backend
2. Le backend passe le statut à `closed`
3. Le bot archive et verrouille le thread Discord

### `/ticket assign <moderator>`

Assigne un modérateur au ticket.

| Paramètre | Obligatoire | Type | Description |
|-----------|-------------|------|-------------|
| `moderator` | Oui | @utilisateur | Le modérateur à assigner |

**Ce qui se passe :**
1. Le bot envoie `PATCH /api/tickets/{id}/assign` au backend
2. Le backend enregistre l'assignation
3. Le bot ajoute le modérateur au thread Discord

---

## Sync automatique des messages

Tout message envoyé par un utilisateur dans un thread `ticket-*` est automatiquement transmis au backend via `POST /api/tickets/{id}/messages`. Cela permet à l'app desktop de voir la conversation complète.

Le bot ignore les messages de bots pour éviter les boucles.

---

## Communication avec le backend

| Action | Méthode | Endpoint | Description |
|--------|---------|----------|-------------|
| Créer un ticket | `POST` | `/api/tickets` | Crée le ticket en base |
| Voir un ticket | `GET` | `/api/tickets/{id}` | Détail + messages |
| Répondre | `POST` | `/api/tickets/{id}/messages` | Ajoute un message |
| Fermer | `PATCH` | `/api/tickets/{id}/close` | Passe en statut `closed` |
| Assigner | `PATCH` | `/api/tickets/{id}/assign` | Assigne un modérateur |

L'authentification se fait via le header `Authorization: Bearer <API_KEY>`.

### Requête de création

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

### Réponse de création

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

---

## Flux complet

```
Utilisateur tape /ticket create "Harcèlement" report high
       │
       ▼
┌──────────────────────────────────────┐
│           TICKET BOT                 │
│                                      │
│  1. Récupère les infos de la commande│
│  2. POST /api/tickets → Backend      │
└──────────────┬───────────────────────┘
               │
               ▼
┌──────────────────────────────────────┐
│           API BACKEND                │
│                                      │
│  1. Crée le ticket en base (PG)     │
│  2. Retourne le ticket avec son ID  │
└──────────────┬───────────────────────┘
               │
               ▼
┌──────────────────────────────────────┐
│           TICKET BOT                 │
│                                      │
│  3. Crée un thread privé sur Discord │
│  4. Envoie le message d'ouverture   │
│  5. Réponse éphémère à l'utilisateur│
└──────────────────────────────────────┘
               │
               ▼
  L'utilisateur et les modos discutent dans le thread
  → Chaque message est sync vers POST /api/tickets/{id}/messages
               │
               ▼
  Un modo tape /ticket close
       │
       ▼
  Bot → PATCH /api/tickets/{id}/close → Backend
  Bot → Archive et verrouille le thread Discord
```

---

## Configuration

### Variables d'environnement

Copier `.env.example` en `.env` :

| Variable | Obligatoire | Description | Défaut |
|----------|-------------|-------------|--------|
| `DISCORD_TOKEN` | Oui | Token du bot Discord | - |
| `API_BASE_URL` | Non | URL du backend | `http://localhost:3000` |
| `API_KEY` | Non | Clé API pour l'authentification | _(vide)_ |
| `TICKET_CATEGORY_ID` | Non | ID de catégorie Discord pour les threads | _(vide)_ |

### Intents Discord requis

- **GUILD_MESSAGES** : recevoir les messages dans les threads
- **MESSAGE_CONTENT** : lire le contenu des messages pour la sync
- **GUILDS** : accéder aux infos des serveurs

### Permissions Discord requises

| Permission | Raison |
|------------|--------|
| Send Messages | Envoyer des messages dans les threads |
| Create Private Threads | Créer les threads de tickets |
| Manage Threads | Archiver/verrouiller les threads fermés |
| Send Messages in Threads | Écrire dans les threads |
| Use Slash Commands | Enregistrer et utiliser les commandes |

---

## Installation et lancement

### Prérequis

- Rust >= 1.75
- Un token de bot Discord avec les intents activés
- Le backend Sentinel API lancé

### Commandes

```bash
cd bots/ticket-bot

cp .env.example .env
# Renseigner DISCORD_TOKEN et API_BASE_URL

cargo run
```

### Docker

```bash
docker compose up -d ticket-bot
```

---

## Logs

| Niveau | Événement |
|--------|-----------|
| `INFO` | Démarrage, ticket créé/fermé/assigné, slash commands enregistrées |
| `ERROR` | Erreur API, erreur Discord |

Exemple :

```
2026-03-26T10:00:00  INFO ticket_bot: Démarrage du ticket bot api_url=http://localhost:3000
2026-03-26T10:00:01  INFO ticket_bot::handler: Ticket bot connecté bot=TicketBot
2026-03-26T10:00:01  INFO ticket_bot::handler: Slash commands enregistrées
2026-03-26T10:05:00  INFO ticket_bot::commands::ticket: Ticket créé ticket_id=550e8400 author=pseudo guild=MonServeur
2026-03-26T10:30:00  INFO ticket_bot::commands::ticket: Ticket fermé ticket_id=550e8400
```
