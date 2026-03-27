# DiscordSentinel

Plateforme de moderation distribuee pour serveurs Discord. Architecture microservices : bots legers (interfaces), API centrale (intelligence), app desktop (administration).

---

## Architecture globale

```
Discord Messages / Events
       |
       v
+---------------------+     +---------------------+     +---------------------+
|   Automod Bot       |     |   Moderation Bot    |     |   Security Bot      |
|   Detection locale  |     |   /warn /mute /ban  |     |   Anti-raid         |
|   + appel API       |     |   + log API         |     |   Comptes suspects  |
+--------+------------+     +--------+------------+     +--------+------------+
         |                           |                           |
         +---------------------------+---------------------------+
                                     |
                              POST /analyze, /api/...
                                     |
                                     v
                          +---------------------+
                          |   API Backend       |  <-- Scoring, decisions, persistance
                          |   (Axum / Rust)     |
                          +--------+------------+
                             |         |      |
                             v         v      v
                          PostgreSQL  Redis  WebSocket
                             16        7     (temps reel)
                                     |
         +---------------------------+---------------------------+
         |                           |                           |
         v                           v                           v
+---------------------+     +---------------------+     +---------------------+
|   Ticket Bot        |     |   Stats Bot         |     |   Desktop App       |
|   /ticket create    |     |   /stats user       |     |   Tauri + Vue 3     |
|   /ticket close     |     |   /stats server     |     |   Admin complete    |
+---------------------+     |   /stats top        |     +---------------------+
                             +---------------------+
```

**Philosophie** : Bots = interfaces (legers, pas de logique metier) | API = cerveau (decisions) | App = controle (admin)

---

## Stack technique

| Composant | Technologie | Details |
|-----------|------------|---------|
| API Backend | Rust, Axum 0.8, Tokio | Architecture hexagonale (ports & adapters) |
| Base de donnees | PostgreSQL 16 | 7 tables : rules, infractions, tickets, ticket_messages, security_events, moderation_actions, user_stats |
| Cache | Redis 7 | Cache regles TTL 5min, cache stats overview TTL 60s |
| Automod Bot | Rust, Serenity 0.12 | Detection spam/insultes/liens + appel API |
| Moderation Bot | Rust, Serenity 0.12 | /warn /mute /ban /unmute /unban /history |
| Security Bot | Rust, Serenity 0.12, DashMap | Anti-raid + detection comptes suspects |
| Stats Bot | Rust, Serenity 0.12 | /stats user, server, top + tracking temps reel |
| Ticket Bot | Rust, Serenity 0.12 | /ticket create, close, assign |
| Desktop App Frontend | Vue 3, TypeScript, Vite, Pinia | Atomic design (atoms/molecules/organisms), 11 pages |
| Desktop App Backend | Tauri 2.x, Rust | Architecture hexagonale, HEED/LMDB local, WebSocket |
| Containerisation | Docker (Alpine), Docker Compose | Multi-stage builds |
| Migrations SQL | sqlx 0.8 | 7 migrations |

**Dependances Rust cles** : serde, reqwest 0.12, sqlx 0.8, chrono, uuid, thiserror, tracing, async-trait, regex, tower-http (CORS, rate limiting, tracing), dashmap, futures-util, tokio-tungstenite

---

## Structure du projet

```
DiscordSentinel/
|
|-- apps/
|   +-- desktop/                    # App admin Tauri + Vue 3
|       |-- src/                    # Frontend Vue 3 + TypeScript
|       |   |-- components/         # Atomic design (atoms, molecules, organisms, pages)
|       |   |-- router/             # Vue Router (11 routes)
|       |   |-- composables/        # 12 composables Vue
|       |   |-- types/              # TypeScript interfaces
|       |   +-- styles/             # CSS global
|       |-- src-tauri/              # Backend Tauri (Rust)
|       |   +-- src/
|       |       |-- application/    # 10 services (auth, dashboard, infractions, rules, tickets, logs, notifications, security, moderation, realtime)
|       |       |-- domain/         # Entites & ports
|       |       |-- infrastructure/ # Adapters (API client, config store LMDB, mock)
|       |       +-- presentation/   # 26 Tauri commands (IPC)
|       +-- package.json
|
|-- services/
|   +-- api/                        # API centrale (Axum)
|       |-- src/
|       |   |-- main.rs             # Bootstrap, DI, demarrage serveur
|       |   |-- config.rs           # Config env
|       |   |-- domain/
|       |   |   |-- entities/       # Rule, Infraction, Ticket, UserStats, SecurityEvent, ModerationAction, MessageAnalysis
|       |   |   |-- value_objects/  # Action, DetectionFlags, FlagType
|       |   |   |-- services/       # ScoringService
|       |   |   +-- errors.rs       # Erreurs domaine -> HTTP
|       |   |-- ports/
|       |   |   |-- inbound/        # Traits UseCase
|       |   |   +-- outbound/       # Traits Repository + Cache
|       |   |-- application/        # Implementations use cases (analyze, rules, infractions, tickets, security, moderation, stats)
|       |   +-- adapters/
|       |       |-- inbound/
|       |       |   |-- http/       # Handlers, DTOs, middleware (auth, rate_limit), router
|       |       |   +-- ws/         # WebSocket broadcaster + handler
|       |       +-- outbound/       # PostgreSQL repos, Redis cache
|       |-- migrations/             # 7 SQL migrations
|       |-- Dockerfile
|       +-- Cargo.toml
|
|-- bots/
|   |-- automod-bot/                # Bot auto-moderation
|   |   +-- src/
|   |       |-- main.rs
|   |       |-- handler.rs          # EventHandler (message, ready)
|   |       |-- api_client.rs       # Client HTTP vers API /analyze
|   |       |-- config.rs
|   |       +-- detectors/          # Detection locale (spam, insult, link)
|   |-- moderation-bot/             # Bot moderation manuelle
|   |   +-- src/
|   |       |-- main.rs
|   |       |-- handler.rs
|   |       |-- api_client.rs
|   |       |-- config.rs
|   |       +-- commands/           # warn, mute, ban, history
|   |-- security-bot/               # Bot securite serveur
|   |   +-- src/
|   |       |-- main.rs
|   |       |-- handler.rs
|   |       |-- api_client.rs
|   |       |-- config.rs
|   |       |-- raid_detector.rs    # Detection anti-raid (DashMap, thread-safe)
|   |       +-- account_checker.rs  # Verification age compte
|   |-- stats-bot/                  # Bot statistiques
|   |   +-- src/
|   |       |-- main.rs
|   |       |-- handler.rs          # Tracking messages + voice
|   |       |-- api_client.rs
|   |       |-- config.rs
|   |       |-- tracker.rs          # Cache local (RwLock + HashMap)
|   |       +-- commands/stats.rs   # /stats user, server, top
|   +-- ticket-bot/                 # Bot tickets support
|       +-- src/
|           |-- main.rs
|           |-- handler.rs
|           |-- api_client.rs
|           |-- config.rs
|           +-- commands/ticket.rs  # /ticket create, close, assign
|
|-- docs/                           # Documentation technique
|   |-- api.md                      # Architecture API + endpoints
|   |-- automod-bot.md              # Design automod bot
|   |-- ticket-bot.md               # Design ticket bot
|   |-- desktop-app.md              # Design app desktop
|   |-- communication-bot-api.md    # Protocole bot <-> API
|   +-- communication-app-api.md    # Protocole app <-> API
|
|-- docker-compose.yml              # Orchestration complete
|-- dev.sh                          # Script dev local
|-- .env.example                    # Template variables d'environnement
+-- README.md
```

---

## Schema base de donnees (PostgreSQL)

### Table `rules` - Regles de moderation par serveur

| Colonne | Type | Description |
|---------|------|-------------|
| id | UUID PK | Identifiant unique |
| guild_id | VARCHAR | ID du serveur Discord |
| flag_type | VARCHAR | Type : Spam, Insult, Link |
| weight | FLOAT | Poids dans le scoring (defaut: Spam=3, Insult=5, Link=1) |
| threshold_warn | FLOAT | Seuil warn (defaut: 2.0) |
| threshold_delete | FLOAT | Seuil delete (defaut: 4.0) |
| threshold_mute | FLOAT | Seuil mute (defaut: 6.0) |
| threshold_ban | FLOAT | Seuil ban (defaut: 9.0) |
| enabled | BOOLEAN | Regle active/inactive |
| created_at, updated_at | TIMESTAMP | Horodatage |

Contrainte unique : `(guild_id, flag_type)` | Index : `guild_id`

### Table `infractions` - Violations enregistrees

| Colonne | Type | Description |
|---------|------|-------------|
| id | UUID PK | Identifiant unique |
| guild_id, channel_id, user_id | VARCHAR | Contexte Discord |
| username | VARCHAR | Nom utilisateur |
| message_id | VARCHAR | ID message original |
| content | TEXT | Contenu du message |
| flags | JSONB | Flags detectes (spam, insult, link avec details) |
| score | FLOAT | Score calcule |
| action | VARCHAR | Action executee (none/warn/delete/mute/ban) |
| reason | TEXT | Raison |
| duration | INTEGER NULL | Duree en secondes (mute) |
| created_at | TIMESTAMP | Date |

Index : `guild_id`, `(guild_id, user_id)`, `created_at DESC`, `action`

### Table `tickets` + `ticket_messages` - Systeme de tickets

- `tickets` : id, title, status (open/closed), priority, author_id, author_name, assigned_to, server, category, timestamps, messages_count
- `ticket_messages` : id, ticket_id (FK cascade), author_name, author_role, content, timestamps

Index : `status`, `author_id`, `assigned_to`, `server`, `created_at DESC`

### Table `security_events` - Evenements de securite

| Colonne | Type | Description |
|---------|------|-------------|
| id | UUID PK | Identifiant unique |
| guild_id | VARCHAR | ID du serveur |
| event_type | VARCHAR | Type (raid_detected, suspicious_account) |
| severity | VARCHAR | Severite (critical, high, medium, warning) |
| description | TEXT | Description de l'evenement |
| user_ids | JSONB | IDs utilisateurs impliques |
| created_at | TIMESTAMP | Date |

Index : `guild_id`, `event_type`, `created_at DESC`, `severity`

### Table `moderation_actions` - Historique de moderation

| Colonne | Type | Description |
|---------|------|-------------|
| id | UUID PK | Identifiant unique |
| guild_id, channel_id | VARCHAR | Contexte Discord |
| moderator_id, moderator_name | VARCHAR | Moderateur |
| target_id, target_name | VARCHAR | Utilisateur cible |
| action_type | VARCHAR | Type (warn/mute/ban) |
| reason | TEXT | Raison |
| gravity | VARCHAR NULL | Gravite (low/medium/high) |
| duration | INTEGER NULL | Duree en secondes |
| created_at | TIMESTAMP | Date |

Index : `guild_id`, `(guild_id, target_id)`, `action_type`, `created_at DESC`

### Table `user_stats` - Statistiques utilisateurs

| Colonne | Type | Description |
|---------|------|-------------|
| id | UUID PK | Identifiant unique |
| guild_id | VARCHAR | ID du serveur |
| user_id | VARCHAR | ID utilisateur |
| username | VARCHAR | Nom utilisateur |
| message_count | BIGINT | Nombre de messages |
| voice_seconds | BIGINT | Temps vocal en secondes |
| updated_at | TIMESTAMP | Derniere mise a jour |

Contrainte unique : `(guild_id, user_id)` | Index : `guild_id`

---

## Endpoints API

### Authentification

Toutes les routes (sauf `/health`) necessitent : `Authorization: Bearer <API_KEY>`
Si `API_KEY` est vide dans la config, l'auth est desactivee (mode dev).

### WebSocket

| Methode | Route | Description |
|---------|-------|-------------|
| GET | `/ws?token=<api_key>` | Flux d'evenements temps reel |

Evenements broadcasts : `infraction_new`, `ticket_new`, `ticket_message`, `ticket_closed`, `ticket_assigned`, `security_event`, `moderation_action`

### Routes Bot - Analyse

| Methode | Route | Description |
|---------|-------|-------------|
| POST | `/analyze` | Analyse un message et retourne l'action a executer |

**Body** : `{ guild_id, channel_id, user_id, username, content, flags: { spam, insult, link }, metadata: { message_id, timestamp } }`
**Reponse** : `{ action: "none"|"warn"|"delete"|"mute"|"ban", reason: "...", duration: null|seconds }`

### Routes Admin - Rules

| Methode | Route | Description |
|---------|-------|-------------|
| GET | `/rules/{guild_id}` | Liste les regles du serveur |
| POST | `/rules` | Creer/modifier une regle |
| DELETE | `/rules/{guild_id}/{rule_id}` | Supprimer une regle |

### Routes Admin - Infractions

| Methode | Route | Description |
|---------|-------|-------------|
| GET | `/infractions/{guild_id}` | Liste les infractions (query: user_id, action, limit, offset) |

### Routes Admin - Tickets

| Methode | Route | Description |
|---------|-------|-------------|
| GET | `/api/tickets` | Lister les tickets |
| POST | `/api/tickets` | Creer un ticket |
| GET | `/api/tickets/{id}` | Detail d'un ticket avec messages |
| POST | `/api/tickets/{id}/messages` | Repondre a un ticket |
| PATCH | `/api/tickets/{id}/close` | Fermer un ticket |
| PATCH | `/api/tickets/{id}/assign` | Assigner un ticket |

### Routes Admin - Security

| Methode | Route | Description |
|---------|-------|-------------|
| POST | `/api/security/events` | Reporter un evenement de securite |
| GET | `/api/security/events` | Lister les evenements (query: guild_id) |

### Routes Admin - Moderation

| Methode | Route | Description |
|---------|-------|-------------|
| POST | `/api/moderation/actions` | Logger une action de moderation |
| GET | `/api/moderation/history/{guild_id}/{user_id}` | Historique moderation d'un utilisateur |

### Routes Admin - Stats

| Methode | Route | Description |
|---------|-------|-------------|
| POST | `/api/stats/messages` | Enregistrer des messages (guild_id, user_id, username, count) |
| POST | `/api/stats/voice` | Enregistrer du temps vocal (guild_id, user_id, username, seconds) |
| GET | `/api/stats/{guild_id}/user/{user_id}` | Stats d'un utilisateur |
| GET | `/api/stats/{guild_id}/overview` | Vue d'ensemble du serveur (cache 60s) |
| GET | `/api/stats/{guild_id}/leaderboard` | Classement des membres (query: limit, max 50) |

### Route publique

| Methode | Route | Description |
|---------|-------|-------------|
| GET | `/health` | Healthcheck (PostgreSQL, Redis, API status) |

---

## Bots Discord

### Automod Bot - Auto-moderation

Detection locale rapide avant appel API :
- **Spam** : majuscules excessives (>=8 chars), repetition caracteres (>=6), repetition mots (>=5)
- **Insultes** : dictionnaire regex francais + anglais
- **Liens** : URLs http/https, invitations discord.gg

Si flags detectes -> appel `POST /analyze` -> execution de l'action (warn/delete/mute/ban).
**Fallback** : si API injoignable, suppression locale du message.

### Moderation Bot - Moderation manuelle

| Commande | Description |
|----------|-------------|
| `/warn <user> <gravity> <reason>` | Avertissement (gravity: low/medium/high) + DM a l'utilisateur |
| `/mute <user> <reason> [duration_minutes]` | Timeout Discord (max 28 jours) |
| `/unmute <user>` | Retrait du timeout |
| `/ban <user> <reason> [duration_hours]` | Bannissement (DM avant ban) |
| `/unban <user_id>` | Debannissement |
| `/history <user>` | Historique de moderation (warns/mutes/bans) |

Toutes les actions sont loguees via `POST /api/moderation/actions`.

### Security Bot - Securite serveur

Fonctionne par evenements (pas de commandes), surveille en temps reel :

- **Anti-raid** : detection de joins massifs (defaut: 10 joins en 10s), activation automatique du niveau de verification le plus eleve, alerte dans le premier canal texte
- **Comptes suspects** : flag les comptes de moins de 24h (configurable)

Tous les evenements sont reportes via `POST /api/security/events`.

### Stats Bot - Statistiques

| Commande | Description |
|----------|-------------|
| `/stats user [target]` | Stats d'un utilisateur (messages, temps vocal, infractions) |
| `/stats server` | Stats globales du serveur |
| `/stats top [limit]` | Classement des membres les plus actifs (1-25) |

Tracking automatique en arriere-plan :
- Compte les messages par utilisateur
- Mesure le temps en vocal (join/leave)
- Cache local (RwLock + HashMap) + sync backend

### Ticket Bot - Tickets support

| Commande | Description |
|----------|-------------|
| `/ticket create <title> <category> [priority]` | Creer un ticket (thread prive) |
| `/ticket close` | Fermer et archiver le ticket |
| `/ticket assign <moderator>` | Assigner un moderateur |

Categories : report, appeal, permissions, bug, suggestion
Priorites : urgent, high, medium, low

---

## Systeme de scoring

Le scoring determine l'action a executer sur un message. Chaque type de flag a un poids configurable par serveur.

**Poids par defaut** : Spam = 3.0, Insult = 5.0, Link = 1.0

**Calcul** : `score = somme(poids des flags actifs)`

**Seuils par defaut** :
- `score >= 2.0` -> warn
- `score >= 4.0` -> delete
- `score >= 6.0` -> mute (10 min)
- `score >= 9.0` -> ban

L'action la plus severe dont le seuil est atteint est appliquee. Si aucune regle n'existe pour le serveur, des regles par defaut sont creees automatiquement.

---

## Desktop App (Tauri)

### Pages (11 ecrans)

| Page | Fonctionnalite |
|------|---------------|
| Setup | Configuration initiale (URL API + credentials Discord OAuth) |
| Login | Connexion Discord OAuth |
| Dashboard | Stats globales (serveurs, utilisateurs, messages, infractions, bots) |
| Logs | Logs d'activite avec filtres (niveau, bot) |
| Infractions | Table des infractions avec details |
| Rules | Gestion des regles (toggle, edition seuils/poids) |
| Bans | Liste des bans avec recherche et filtres |
| Moderation | Application d'actions + consultation historique |
| Security | Monitoring evenements de securite en temps reel |
| Tickets | Gestion tickets (liste, detail, reponse, fermeture, assignation) |
| Settings | Configuration (URL API, cle, auto-refresh, logout) |

### Architecture Rust (src-tauri)

Architecture hexagonale avec 10 services exposes via 26 commandes IPC Tauri :

| Service | Fonctionnalite |
|---------|---------------|
| AuthService | OAuth Discord, persistence session (HEED/LMDB), port 19836 callback |
| DashboardService | Stats globales |
| LogsService | Logs de moderation |
| InfractionsService | Consultation infractions |
| RulesService | CRUD regles par serveur |
| TicketsService | Cycle de vie complet des tickets |
| SecurityService | Evenements de securite |
| ModerationService | Actions de moderation + historique |
| RealtimeService | WebSocket avec auto-reconnect (backoff exponentiel) |
| ConfigStore | Stockage LMDB persistent (config Discord, config API) |

### Frontend Vue 3

- **Atomic Design** : atoms (Badge, Button, Input, Select, Toggle, StatusDot), molecules (FilterBar, NavItem, StatCard), organisms (DataTable, NotificationPanel, RuleCard, RuleEditModal, SidebarNav)
- **12 composables** : useAuth, useFetch, useDashboard, useLogs, useInfractions, useRules, useBans, useTickets, useSecurity, useModeration, useRealtime, useNotifications
- **Notifications natives** : alertes desktop pour evenements critiques via WebSocket
- **Mock adapter** : mode fallback quand l'API n'est pas configuree

---

## Middleware API

| Middleware | Description |
|-----------|-------------|
| Auth | Bearer token, mode dev si API_KEY vide, WebSocket via query param |
| Rate Limiting | Token bucket par IP (defaut: 50 req/s, burst 10x), header Retry-After |
| CORS | Origins configurables, methodes GET/POST/PATCH/DELETE/OPTIONS |
| Body Limit | Limite taille requete (defaut: 1 MB) |
| Tracing | Logs structures (method, URI, request_id, status, latency_ms), format JSON optionnel |
| Request ID | Propagation x-request-id |

---

## Deploiement

### Docker Compose (production)

```bash
# Demarrer toute la stack
docker-compose up -d

# Avec monitoring (PgAdmin :5050, Redis Commander :8081)
docker-compose --profile monitoring up -d
```

Services :
- **postgres** (16-alpine) - port 5432, volume `postgres_data`
- **redis** (7-alpine) - port 6379, volume `redis_data`
- **api** - port 3000, depend de postgres + redis
- **automod-bot** - depend de api
- **moderation-bot** - depend de api
- **security-bot** - depend de api
- **ticket-bot** - depend de api

### Variables d'environnement (.env)

```env
# Infrastructure
POSTGRES_PASSWORD=sentinel_secret
REDIS_PASSWORD=sentinel_redis

# API
API_KEY=your_api_key_here
DATABASE_URL=postgres://sentinel:sentinel_secret@postgres:5432/discord_sentinel
REDIS_URL=redis://:sentinel_redis@redis:6379
HOST=0.0.0.0
PORT=3000
RUST_LOG=info

# API (optionnel)
RATE_LIMIT_PER_SEC=50           # Requetes par seconde par IP (defaut: 50)
MAX_BODY_SIZE=1048576            # Taille max body en bytes (defaut: 1MB)
SHUTDOWN_TIMEOUT=30              # Timeout arret gracieux en secondes
ALLOWED_ORIGINS=                 # CORS origins (vide ou * = permissif)
LOG_FORMAT=                      # "json" pour logs JSON (defaut: human)

# Bots (un token par bot)
AUTOMOD_DISCORD_TOKEN=...
MODERATION_DISCORD_TOKEN=...
SECURITY_DISCORD_TOKEN=...
TICKET_DISCORD_TOKEN=...
STATS_DISCORD_TOKEN=...

# Security Bot (optionnel)
RAID_JOIN_THRESHOLD=10           # Joins pour declencher alerte raid
RAID_JOIN_WINDOW_SECS=10         # Fenetre de temps en secondes
MIN_ACCOUNT_AGE_SECS=86400       # Age minimum compte (24h)

# Monitoring (optionnel)
PGADMIN_EMAIL=admin@sentinel.local
PGADMIN_PASSWORD=admin
```

### Developpement local

**Tout lancer d'un coup** (API + bots + desktop) :

```bash
bash dev.sh
```

Le script `dev.sh` :
- Charge le `.env` automatiquement
- Verifie les prerequis (cargo, node)
- Lance l'API en premier, puis les bots et l'app desktop en parallele
- Redirige les logs dans `.logs/` (un fichier par service)
- `Ctrl+C` arrete tout proprement

**Lancer individuellement** :

```bash
# API
cd services/api
cargo run                      # RUST_LOG=debug pour verbose

# Automod bot
cd bots/automod-bot
cargo run

# Moderation bot
cd bots/moderation-bot
cargo run

# Security bot
cd bots/security-bot
cargo run

# Stats bot
cd bots/stats-bot
cargo run

# Ticket bot
cd bots/ticket-bot
cargo run

# Desktop
cd apps/desktop
npm install
npm run tauri dev              # App complete avec hot reload

# Build production desktop
npm run tauri build            # Executable natif
```

---

## Etat d'avancement

### Implemente

**API Backend** :
- Architecture hexagonale complete (domain/ports/adapters)
- 7 use cases : analyze, rules, infractions, tickets, security, moderation, stats
- WebSocket temps reel (broadcaster d'evenements)
- Middleware : auth, rate limiting, CORS, body limit, tracing structure
- 7 migrations PostgreSQL, cache Redis avec invalidation
- Healthcheck (PostgreSQL + Redis)
- Arret gracieux avec drain de connexions

**Bots Discord (5)** :
- Automod bot : detection locale (spam/insultes/liens) + appel API + fallback
- Moderation bot : /warn /mute /ban /unmute /unban /history avec DM et logging
- Security bot : anti-raid (detection joins massifs) + comptes suspects + alertes
- Stats bot : tracking messages/vocal + /stats user, server, top + leaderboard
- Ticket bot : /ticket create, close, assign avec threads prives

**Desktop App** :
- 11 pages d'administration completes
- OAuth Discord avec callback local
- WebSocket temps reel avec auto-reconnect
- Notifications natives desktop
- Stockage local LMDB (config persistante)
- Mock adapter pour mode hors-ligne
- 26 commandes Tauri IPC

**Infrastructure** :
- Docker Compose (9 services + monitoring optionnel)
- Script dev.sh pour developpement local
- Multi-stage Docker builds

### Non implemente

- Worker service (traitement async, queue Redis)
- Gateway service (WebSocket/realtime dedie)
- Stats bot non inclus dans docker-compose.yml
- Infrastructure Kubernetes
- CI/CD (GitHub Actions)
- Tests d'integration end-to-end

### Roadmap

- **Phase 1 (MVP)** : en cours - API, 5 bots, DB, desktop app
- **Phase 2** : queue system Redis, worker async, CI/CD, tests e2e
- **Phase 3** : IA/ML detection, anti-raid avance, analytics, dashboard complet

---

## Bonnes pratiques du projet

- **Jamais de logique metier dans les bots** : les bots sont des interfaces legeres
- **Toujours passer par l'API** : centralisation des decisions
- **Architecture hexagonale** : separation stricte domain/ports/adapters
- **Gestion d'erreurs** : `thiserror` pour les erreurs domain, conversion auto vers HTTP status (400, 404, 409, 422, 504, 500)
- **Cache** : Redis pour les regles (TTL 5min) et stats overview (TTL 60s), invalidation manuelle a la modification
- **Fallback** : si API injoignable, le bot prend une decision locale de securite
- **Rate limiting** : token bucket par IP avec burst configurable
- **Observabilite** : logs structures avec request_id, format JSON optionnel
- **WebSocket** : broadcast d'evenements pour mise a jour temps reel (desktop app + futurs clients)
