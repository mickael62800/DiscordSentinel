# DiscordSentinel

Plateforme de moderation distribuee pour serveurs Discord. Architecture microservices : bots legers (interfaces), API centrale (intelligence), workers (traitement async), app desktop (administration).

---

## Architecture globale

```
Discord Messages
       |
       v
+-------------------+
|   Automod Bot     |  <-- Detection locale rapide (spam, insultes, liens)
|   (Serenity/Rust) |
+--------+----------+
         |
    POST /analyze (si flags detectes)
         |
         v
+-------------------+
|   API Backend     |  <-- Scoring, decision, persistance
|   (Axum/Rust)     |
+--------+----------+
    |         |
    v         v
 PostgreSQL  Redis       <-- Persistence + cache regles (TTL 5min)
    16        7
         |
         v
  { action, reason, duration }
         |
         v
+-------------------+
|   Automod Bot     |  <-- Execute l'action (warn/delete/mute/ban)
+-------------------+

+-------------------+
|   Desktop App     |  <-- Admin: regles, infractions, tickets, logs
|   (Tauri + Vue 3) |
+-------------------+
```

**Philosophie** : Bots = interfaces (legers, pas de logique metier) | API = cerveau (decisions) | Workers = puissance (async) | App = controle (admin)

---

## Stack technique

| Composant | Technologie | Details |
|-----------|------------|---------|
| API Backend | Rust, Axum 0.8, Tokio | Hexagonal architecture (ports & adapters) |
| Base de donnees | PostgreSQL 16 | Rules, infractions, tickets |
| Cache | Redis 7 | Cache regles TTL 5min, futur: queues |
| Bots Discord | Rust, Serenity 0.12 | Automod + Ticket fonctionnels |
| Desktop App Frontend | Vue 3, TypeScript, Vite, Pinia | Atomic design (atoms/molecules/organisms) |
| Desktop App Backend | Tauri 2.x, Rust | Hexagonal architecture, HEED/LMDB local |
| Containerisation | Docker (Alpine), Docker Compose | Multi-stage builds |
| Migrations SQL | sqlx 0.8 | 3 migrations (rules, infractions, tickets) |

**Dependances Rust cles** : serde, reqwest 0.12, sqlx 0.8, chrono, uuid, thiserror, tracing, async-trait, regex

---

## Structure du projet

```
DiscordSentinel/
|
|-- apps/
|   +-- desktop/                    # App admin Tauri + Vue 3
|       |-- src/                    # Frontend Vue 3 + TypeScript
|       |   |-- components/         # Atomic design (atoms, molecules, organisms)
|       |   |-- router/             # Vue Router
|       |   |-- composables/        # Vue composables
|       |   +-- styles/             # CSS global
|       |-- src-tauri/              # Backend Tauri (Rust)
|       |   +-- src/
|       |       |-- application/    # Services (auth, dashboard, infractions, rules, tickets, logs, notifications)
|       |       |-- domain/         # Logique metier & ports
|       |       |-- infrastructure/ # Adapters (API client, config store, mock)
|       |       +-- presentation/   # Tauri commands (IPC)
|       +-- package.json
|
|-- services/
|   |-- api/                        # API centrale (Axum)
|   |   |-- src/
|   |   |   |-- main.rs             # Bootstrap, DI, demarrage serveur
|   |   |   |-- config.rs           # Config env (DATABASE_URL, REDIS_URL, API_KEY, PORT)
|   |   |   |-- domain/
|   |   |   |   |-- entities/       # Rule, Infraction, Ticket, MessageAnalysis
|   |   |   |   |-- value_objects/  # Action, DetectionFlags, FlagType (Spam/Insult/Link)
|   |   |   |   +-- services/      # ScoringService (calcul score + seuils)
|   |   |   |-- ports/
|   |   |   |   |-- inbound/       # Traits UseCase (AnalyzeMessage, ManageRules, etc.)
|   |   |   |   +-- outbound/      # Traits Repository + Cache
|   |   |   |-- application/       # Implementations des use cases
|   |   |   |   |-- analyze_message_service.rs
|   |   |   |   |-- manage_rules_service.rs
|   |   |   |   |-- manage_infractions_service.rs
|   |   |   |   +-- manage_tickets_service.rs
|   |   |   +-- adapters/
|   |   |       |-- inbound/http/  # Handlers Axum, DTOs, middleware auth, router
|   |   |       +-- outbound/      # PostgreSQL repos, Redis cache
|   |   |-- migrations/            # SQL (rules, infractions, tickets + ticket_messages)
|   |   |-- Dockerfile
|   |   +-- Cargo.toml
|   |-- worker/                     # [STUB] Futur traitement async
|   +-- gateway/                    # [STUB] Futur WebSocket/realtime
|
|-- bots/
|   |-- automod-bot/                # Bot auto-moderation (FONCTIONNEL)
|   |   +-- src/
|   |       |-- main.rs             # Init client Serenity
|   |       |-- handler.rs          # EventHandler (message, ready)
|   |       |-- api_client.rs       # Client HTTP vers API /analyze
|   |       |-- config.rs           # Env: DISCORD_TOKEN, API_URL, API_KEY
|   |       +-- detectors/          # Detection locale rapide
|   |           |-- mod.rs          # Orchestrateur
|   |           |-- spam.rs         # Majuscules, repetitions
|   |           |-- insult.rs       # Regex patterns FR + EN
|   |           +-- link.rs         # URLs, discord.gg
|   |-- ticket-bot/                 # Bot tickets (FONCTIONNEL)
|   |   +-- src/
|   |       |-- main.rs
|   |       |-- handler.rs
|   |       |-- api_client.rs
|   |       |-- config.rs
|   |       +-- commands/ticket.rs  # Slash commands
|   |-- moderation-bot/             # [STUB] Vide
|   |-- security-bot/               # [STUB] Vide
|   +-- voice-bot/                  # [STUB] Vide
|
|-- packages/                       # [STUBS] Libs partagees (vides)
|   |-- config/
|   |-- core/
|   |-- types/
|   +-- utils/
|
|-- infra/                          # [STUBS] Infrastructure (vides)
|   |-- docker/
|   |-- k8s/
|   +-- scripts/
|
|-- docs/                           # Documentation technique
|   |-- api.md                      # Architecture API + endpoints
|   |-- automod-bot.md              # Design automod bot
|   |-- ticket-bot.md               # Design ticket bot
|   +-- communication-bot-api.md    # Protocole bot <-> API
|
|-- docker-compose.yml              # Orchestration complete
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

Contrainte unique : `(guild_id, flag_type)`

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

Index : `guild_id`, `(guild_id, user_id)`

### Table `tickets` + `ticket_messages` - Systeme de tickets

- `tickets` : id, title, status (open/closed), priority, author_id, assigned_to, timestamps
- `ticket_messages` : id, ticket_id (FK cascade), author_id, content, timestamps

---

## Endpoints API

### Authentification

Toutes les routes (sauf `/health`) necessitent : `Authorization: Bearer <API_KEY>`
Si `API_KEY` est vide dans la config, l'auth est desactivee (mode dev).

### Routes Bot

| Methode | Route | Description | Body |
|---------|-------|-------------|------|
| POST | `/analyze` | Analyse un message | `{ guild_id, channel_id, user_id, username, message_id, content, flags: { spam, insult, link } }` |

**Reponse** : `{ action: "none"|"warn"|"delete"|"mute"|"ban", reason: "...", duration: null|seconds }`

### Routes Admin (Rules)

| Methode | Route | Description |
|---------|-------|-------------|
| GET | `/rules/{guild_id}` | Liste les regles du serveur |
| POST | `/rules/{guild_id}` | Creer/modifier une regle |
| DELETE | `/rules/{guild_id}/{rule_id}` | Supprimer une regle |

### Routes Admin (Infractions)

| Methode | Route | Description |
|---------|-------|-------------|
| GET | `/infractions/{guild_id}` | Liste les infractions du serveur |

### Routes Admin (Tickets)

| Methode | Route | Description |
|---------|-------|-------------|
| GET | `/api/tickets` | Lister les tickets |
| POST | `/api/tickets` | Creer un ticket |
| GET | `/api/tickets/{id}` | Detail d'un ticket |
| POST | `/api/tickets/{id}/messages` | Repondre a un ticket |
| PATCH | `/api/tickets/{id}/close` | Fermer un ticket |
| PATCH | `/api/tickets/{id}/assign` | Assigner un ticket |

### Route publique

| Methode | Route | Description |
|---------|-------|-------------|
| GET | `/health` | Healthcheck (pas d'auth) |

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

## Detection locale (Automod Bot)

Avant d'appeler l'API, le bot effectue une detection rapide locale :

- **Spam** : detection majuscules excessives, repetition de caracteres
- **Insults** : dictionnaire regex francais + anglais
- **Links** : patterns URL, detection invitations discord.gg

Si aucun flag n'est detecte, le message est ignore (pas d'appel API).

**Fallback** : si l'API est injoignable, le bot supprime le message localement quand des flags sont detectes.

---

## Desktop App (Tauri)

Architecture hexagonale cote Rust (src-tauri) avec les services suivants exposes via IPC Tauri :

| Service | Fonctionnalite |
|---------|---------------|
| AuthService | OAuth Discord, persistence session (HEED/LMDB) |
| DashboardService | Stats globales, bots en ligne, messages analyses |
| InfractionsService | Consultation infractions avec filtres |
| RulesService | CRUD regles par serveur |
| TicketsService | Cycle de vie complet des tickets |
| LogsService | Logs de moderation |
| NotificationsService | Alertes temps reel |

Frontend Vue 3 : architecture Atomic Design, Pinia pour le state, Vue Router pour la navigation.

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
- **worker** - pas de port externe, depend de postgres + redis
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

# Bots (un token par bot)
AUTOMOD_DISCORD_TOKEN=...
MODERATION_DISCORD_TOKEN=...
SECURITY_DISCORD_TOKEN=...
TICKET_DISCORD_TOKEN=...
VOICE_DISCORD_TOKEN=...

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
- API backend complete avec architecture hexagonale
- Automod bot fonctionnel (detection + appel API + execution actions)
- Ticket bot fonctionnel (slash commands)
- PostgreSQL avec 3 migrations (rules, infractions, tickets)
- Redis caching des regles
- Desktop app shell Tauri + Vue 3
- Docker multi-stage builds
- Systeme de scoring configurable par serveur

### Stubs (non implemente)
- Worker service (traitement async)
- Gateway service (WebSocket/realtime)
- Moderation bot, Security bot, Voice bot (dossiers vides)
- Packages partages (core, config, types, utils)
- Infrastructure k8s

### Roadmap
- **Phase 1 (MVP)** : en cours - API, automod bot, ticket bot, DB, desktop app
- **Phase 2** : queue system Redis, worker async, split bots
- **Phase 3** : IA/ML detection, anti-raid, analytics avances, dashboard complet

---

## Bonnes pratiques du projet

- **Jamais de logique metier dans les bots** : les bots sont des interfaces legeres
- **Toujours passer par l'API** : centralisation des decisions
- **Architecture hexagonale** : separation stricte domain/ports/adapters
- **Gestion d'erreurs** : `thiserror` pour les erreurs domain, conversion auto vers HTTP status
- **Cache** : Redis pour les regles avec TTL 5min, invalidation manuelle a la modification
- **Fallback** : si API injoignable, le bot prend une decision locale de securite
