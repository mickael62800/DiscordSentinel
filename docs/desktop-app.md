# DiscordSentinel — Application Desktop

Application d'administration pour la plateforme DiscordSentinel. Permet aux modérateurs de gérer les serveurs Discord sans passer par Discord.

---

## Stack technique

| Composant | Technologie |
|-----------|-------------|
| Framework | Tauri 2 |
| Frontend | Vue.js 3 + TypeScript |
| Backend (natif) | Rust |
| State management | Pinia + Composables |
| Routing | Vue Router 4 |
| Design system | Atomic Design |
| Architecture backend | Hexagonale (ports & adapters) |
| Persistance locale | LMDB (heed) |
| Communication API | HTTP (CRUD) + WebSocket (events temps réel) |
| Notifications | Tauri plugin-notification (natives OS) |
| Auth | Discord OAuth2 |

---

## Architecture

### Frontend — Atomic Design

```
src/
├── components/
│   ├── atoms/           # Composants de base (Badge, Toggle, Input, Button, Select, StatusDot)
│   ├── molecules/       # Combinaisons d'atoms (StatCard, NavItem, FilterBar)
│   ├── organisms/       # Sections complexes (SidebarNav, DataTable, RuleCard, NotificationPanel)
│   ├── templates/       # Layouts (MainLayout)
│   └── pages/           # Pages complètes (Dashboard, Logs, Infractions, Rules, Tickets, Settings, Login, Setup)
├── composables/         # Logique réutilisable (useAuth, useDashboard, useLogs, useRealtime, useNotifications...)
├── types/               # Interfaces TypeScript centralisées
├── router/              # Routes + navigation guard
└── styles/              # CSS global (dark theme Discord-like)
```

### Backend Rust — Architecture Hexagonale

```
src-tauri/src/
├── domain/
│   ├── entities.rs      # DiscordConfig, ApiConfig, DiscordUser, AuthToken, ServerStats, LogEntry,
│   │                    # Infraction, ModerationRule, Ticket, TicketMessage, TicketDetail, Notification
│   └── ports.rs         # Traits: StatsRepository, LogsRepository, InfractionsRepository,
│                        # RulesRepository, TicketsRepository + AppAdapter super-trait
├── application/
│   ├── auth_service.rs       # OAuth2 Discord (flow complet avec serveur local)
│   ├── realtime_service.rs   # Client WebSocket (connexion, reconnexion, dispatch events Tauri)
│   ├── dashboard_service.rs  # Stats
│   ├── logs_service.rs       # Logs
│   ├── infractions_service.rs # Infractions
│   ├── rules_service.rs      # Rules
│   └── tickets_service.rs    # Tickets
├── infrastructure/
│   ├── config_store.rs  # LMDB — persistance Discord config + API config
│   ├── mock_adapter.rs  # Données fictives (dev sans backend)
│   └── api_adapter.rs   # Appels HTTP vers le backend API (avec Bearer token)
├── presentation/
│   └── commands.rs      # Commandes Tauri (pont frontend ↔ backend natif)
└── lib.rs               # Wiring / injection de dépendances
```

---

## Fonctionnalités

### Authentification Discord OAuth2

- Flow OAuth2 complet : ouvre le navigateur → callback sur serveur local (port 19836) → échange code → fetch profil
- Session persistée via Tauri Store (survit aux fermetures)
- Avatar Discord et infos user dans la sidebar
- Bouton logout

### Setup initial (premier lancement)

**Étape 1 — Backend API :**
- URL de l'API (ex: `http://localhost:3000`)
- API Key (Bearer token, optionnel en dev)
- Sauvegardé en LMDB

**Étape 2 — Discord OAuth :**
- Client ID et Client Secret (depuis Discord Developer Portal)
- Redirect URI à configurer : `http://localhost:19836/callback`
- Sauvegardé en LMDB

### Dashboard

Statistiques globales : serveurs, utilisateurs, messages analysés, infractions du jour, bots en ligne.

- Route API : `GET /api/stats`

### Logs

Liste des logs d'activité des bots avec filtres par niveau (info/warn/error) et par bot.

- Route API : `GET /api/logs`

### Infractions

Tableau des infractions avec colonnes : user, serveur, type, raison, modérateur, date. Badges colorés par sévérité.

- Route API : `GET /api/infractions`

### Rules

Cartes de règles de modération avec toggle on/off. Badges par action (ban, mute, delete, warn).

- Routes API : `GET /api/rules`, `PATCH /api/rules/{id}`

### Tickets

Système complet de gestion de tickets :

- **Vue liste** : filtres statut/priorité, compteurs open/pending, badges
- **Vue détail** : conversation en bulles (user à gauche, staff à droite), infos ticket complètes
- **Actions** : répondre (Ctrl+Enter), fermer, assigner

Routes API :
- `GET /api/tickets`
- `GET /api/tickets/{id}`
- `POST /api/tickets/{id}/messages`
- `PATCH /api/tickets/{id}/close`
- `PATCH /api/tickets/{id}/assign`

### Notifications temps réel

Via WebSocket (`ws://{api_url}/ws`) :

- Connexion auto au login, reconnexion auto toutes les 5s
- Events reçus : `infraction_new`, `ticket_new`, `ticket_message`, `ticket_closed`, `ticket_assigned`, `bot_status`, `raid_detected`
- Panneau de notifications dans la sidebar (cloche avec badge compteur)
- Notifications natives OS (Windows toast) pour les alertes critical/high
- Indicateur connexion WS dans la sidebar (dot vert/rouge)

### Settings

- Modifier l'URL et la clé API du backend
- Reset complet des credentials (Discord + API)
- Configuration auto-refresh (pour les pages qui chargent en HTTP)

---

## Communication avec le backend

### HTTP — Chargement initial et actions CRUD

| Action | Méthode | Route |
|--------|---------|-------|
| Stats dashboard | GET | `/api/stats` |
| Logs | GET | `/api/logs` |
| Infractions | GET | `/api/infractions` |
| Rules list | GET | `/api/rules` |
| Toggle rule | PATCH | `/api/rules/{id}` |
| Tickets list | GET | `/api/tickets` |
| Ticket detail | GET | `/api/tickets/{id}` |
| Reply ticket | POST | `/api/tickets/{id}/messages` |
| Close ticket | PATCH | `/api/tickets/{id}/close` |
| Assign ticket | PATCH | `/api/tickets/{id}/assign` |

Toutes les requêtes HTTP incluent le header `Authorization: Bearer <API_KEY>` si une clé est configurée.

### WebSocket — Events push temps réel

Connexion : `ws://{api_url}/ws?token={api_key}`

| Event | Description |
|-------|-------------|
| `infraction_new` | Nouvelle infraction (action != none) |
| `ticket_new` | Nouveau ticket créé |
| `ticket_message` | Réponse ajoutée à un ticket |
| `ticket_closed` | Ticket fermé |
| `ticket_assigned` | Ticket assigné |
| `bot_status` | Bot connecté/déconnecté |
| `raid_detected` | Détection de raid |

---

## Persistance locale (LMDB)

Stocké dans `%LOCALAPPDATA%/discord-sentinel/` :

| Clé | Valeur | Usage |
|-----|--------|-------|
| `discord_client_id` | Client ID OAuth | Auth Discord |
| `discord_client_secret` | Client Secret OAuth | Auth Discord |
| `api_url` | URL du backend | Connexion API |
| `api_key` | Bearer token API | Auth API |

Session utilisateur Discord stockée via Tauri Store dans `auth.json`.

---

## Adapter dynamique

Au démarrage, l'app vérifie la config API dans LMDB :

- **Config présente** → `ApiAdapter` (appels HTTP réels + WebSocket)
- **Pas de config** → `MockAdapter` (données fictives, pas de WebSocket)

Pour passer d'un mode à l'autre, il suffit de configurer/supprimer l'URL API dans Settings.

---

## Sécurité

- Credentials Discord stockés en LMDB local (jamais en clair dans le code)
- API Key transmise via Bearer token HTTP et query param WS (pas dans les logs)
- OAuth2 avec serveur local éphémère (le port est ouvert uniquement pendant le flow auth)
- Champs password masqués dans les formulaires
- Route guard : impossible d'accéder aux pages sans authentification

---

## Développement

### Prérequis

- Node.js >= 18
- Rust >= 1.75
- npm

### Installation

```bash
cd apps/desktop
npm install
```

### Lancer en dev

```bash
npm run tauri dev
```

### Build production

```bash
npm run tauri build
```

L'exécutable sera généré dans `src-tauri/target/release/`.
