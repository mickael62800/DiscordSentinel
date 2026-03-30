# DiscordSentinel

Plateforme de moderation distribuee pour serveurs Discord. Architecture microservices : bots legers (interfaces), API centrale (intelligence), gateway WebSocket (temps reel), app desktop (administration), inference IA (ONNX).

---

## Architecture globale

```
Discord Messages / Events / Images
       |
       v
+----------------+  +----------------+  +----------------+  +----------------+
|  Automod Bot   |  | Moderation Bot |  | Security Bot   |  |  Image Bot     |
|  Spam/insultes |  | /warn /mute    |  | Anti-raid      |  | NSFW/illicite  |
|  + appel API   |  | /ban /history  |  | Comptes suspects|  | + appel API   |
+-------+--------+  +-------+--------+  +-------+--------+  +-------+--------+
        |                    |                    |                    |
        +--------------------+--------------------+--------------------+
                             |
                      POST /analyze, /analyze/image, /api/...
                             |
                             v
                  +---------------------+
                  |   API Backend       |  <-- Scoring, IA inference, decisions, persistance
                  |   (Axum / Rust)     |
                  +--------+------------+
                      |    |    |    |
                      v    v    v    v
                   PgSQL Redis ONNX  Redis pub/sub
                    16    7   Runtime     |
                                         v
                              +---------------------+
                              |   Gateway WebSocket  |
                              |   (temps reel dedie) |
                              +--------+-------------+
                                       |
        +------------------------------+------------------------------+
        |                              |                              |
        v                              v                              v
+----------------+  +----------------+  +---------------------+
|  Ticket Bot    |  |  Stats Bot     |  |   Desktop App       |
|  /ticket       |  |  /stats        |  |   Tauri + Vue 3     |
|  create/close  |  |  user/server   |  |   Admin complete    |
+----------------+  +----------------+  +---------------------+
                              |
+----------------+  +----------------+  +----------------+
|  Voice Bot     |  |  Audit Bot     |  |  Roles Bot     |
|  Salons dyn.   |  |  Logs audit    |  |  Role panels   |
|  Vote kick     |  |  Tracking      |  |  Auto-roles    |
+----------------+  +----------------+  +----------------+
```

**Philosophie** : Bots = interfaces (legers, pas de logique metier) | API = cerveau (decisions + IA) | Gateway = temps reel | App = controle (admin)

---

## Stack technique

| Composant            | Technologie                              | Details                                                    |
| -------------------- | ---------------------------------------- | ---------------------------------------------------------- |
| API Backend          | Rust, Axum 0.8, Tokio                    | Architecture hexagonale, 62+ endpoints, 22 handlers, 14 use cases, helpers.rs |
| Gateway WebSocket    | Rust, Axum 0.8, Redis pub/sub            | Service dedie temps reel, auto-reconnect                   |
| Workers (3 + common) | Rust, Tokio, Redis, sqlx                 | 3 workers specialises + crate partagee sentinel-worker-common |
| Base de donnees      | PostgreSQL 16                            | 29 migrations, 20+ tables                                  |
| Cache                | Redis 7                                  | Cache regles TTL 5min, stats TTL 60s, pub/sub events       |
| Inference IA         | ONNX Runtime 2.0, ndarray, tokenizers    | Vision (NSFW/illicite) + Text (sentiments)                 |
| Automod Bot          | Rust, Serenity 0.12                      | Detection spam/insultes/liens/phishing + appel API         |
| Moderation Bot       | Rust, Serenity 0.12                      | /warn /mute /ban /unmute /unban /history                   |
| Security Bot         | Rust, Serenity 0.12, DashMap             | Anti-raid, quarantaine, captcha DM, slowmode auto          |
| Stats Bot (→ Progression Bot) | Rust, Serenity 0.12               | /stats user, server, top + tracking temps reel + XP/levels |
| Ticket Bot           | Rust, Serenity 0.12                      | /ticket create, close, assign                              |
| Image Bot            | Rust, Serenity 0.12                      | Detection images NSFW/illicites via API                    |
| Voice Bot            | Rust, Serenity 0.12                      | Salons dynamiques, vote kick, co-admins, whitelist/ban     |
| Audit Bot            | Rust, Serenity 0.12                      | Tracking audit logs Discord                                |
| Roles Bot (→ Community Bot) | Rust, Serenity 0.12                 | Role panels + auto-roles + sync Discord roles              |
| Desktop App Frontend | Vue 3, TypeScript, Vite, Pinia, Chart.js | 27 pages, 24 composants UI, 29 composables                 |
| Desktop App Backend  | Tauri 2.x, Rust                          | Architecture hexagonale, HEED/LMDB local, WebSocket        |
| Entrainement IA      | Python, PyTorch, Transformers, ONNX      | 2 modeles : vision + text sentiment                        |
| Containerisation     | Docker (Alpine), Docker Compose          | Multi-stage builds, 15 services                            |

**Dependances Rust cles** : serde, reqwest 0.12, sqlx 0.8, chrono, uuid, thiserror, tracing, async-trait, regex, tower-http (CORS, rate limiting, tracing), dashmap, futures-util, ort (ONNX Runtime), tokenizers, image, base64, ndarray, sentinel-worker-common (crate partagee workers), sentinel-shared (crate partagee bots)

---

## Structure du projet

```
DiscordSentinel/
|
|-- apps/
|   +-- desktop/                        # App admin Tauri + Vue 3
|       |-- src/                        # Frontend Vue 3 + TypeScript
|       |   |-- components/             # Atomic design (6 atoms, 4 molecules, 8 organisms, 1 template, 25 pages)
|       |   |-- router/                 # Vue Router (25 routes)
|       |   |-- composables/            # 25 composables Vue
|       |   |-- types/                  # TypeScript interfaces
|       |   +-- styles/                 # CSS global
|       |-- src-tauri/                  # Backend Tauri (Rust)
|       |   +-- src/
|       |       |-- application/        # 19 services
|       |       |-- domain/             # Entites & ports
|       |       |-- infrastructure/     # Adapters (API client, config store LMDB, mock)
|       |       +-- presentation/       # Tauri commands (IPC)
|       +-- package.json
|
|-- services/
|   |-- api/                            # API centrale (Axum)
|   |   |-- src/
|   |   |   |-- main.rs                 # Bootstrap, DI, demarrage serveur
|   |   |   |-- config.rs              # Config env
|   |   |   |-- domain/
|   |   |   |   |-- entities/           # 21 entites (Rule, Infraction, Ticket, Analytics, etc.)
|   |   |   |   |-- value_objects/      # Action, DetectionFlags, FlagType (10 variants)
|   |   |   |   |-- services/           # ScoringService, InferenceService, TextTokenizer
|   |   |   |   +-- errors.rs           # Erreurs domaine -> HTTP
|   |   |   |-- ports/
|   |   |   |   |-- inbound/            # 14 traits UseCase
|   |   |   |   +-- outbound/           # 19 traits Repository + Cache
|   |   |   |-- application/            # 14 implementations use cases
|   |   |   +-- adapters/
|   |   |       |-- inbound/
|   |   |       |   |-- http/           # 22 handlers, helpers.rs, 19 DTOs, middleware (auth, rate_limit), router
|   |   |       |   +-- ws/             # EventBroadcaster (Redis pub/sub)
|   |   |       +-- outbound/           # 18 PostgreSQL repos, Redis cache
|   |   |-- migrations/                 # 29 SQL migrations
|   |   |-- Dockerfile
|   |   +-- Cargo.toml
|   |
|   |-- gateway/                        # Gateway WebSocket dedie
|   |   |-- src/
|   |   |   |-- main.rs                 # Bootstrap, CORS, graceful shutdown
|   |   |   |-- config.rs              # HOST, PORT, REDIS_URL, API_KEY, MAX_CONNECTIONS
|   |   |   |-- broadcaster.rs         # Broadcast local + limite connexions
|   |   |   |-- handler.rs             # WebSocket handler (auth, ping/pong)
|   |   |   |-- health.rs              # GET /health (status + connected_clients)
|   |   |   +-- redis_subscriber.rs    # Redis pub/sub listener, auto-reconnect
|   |   |-- Dockerfile
|   |   +-- Cargo.toml
|   |
|   +-- worker/                         # Worker async legacy (queue Redis)
|       +-- Cargo.toml
|
|-- services/workers/                    # 3 workers specialises + crate partagee
|   |-- worker-common/                 # Crate partagee (shutdown, heartbeat, scheduler, pg_pool)
|   |   +-- src/lib.rs
|   |-- moderation-worker/              # Conduite, bans, sync ban proposals
|   |   +-- src/ (main, config, scheduler, jobs/)
|   |-- analytics-worker/              # Snapshots quotidiens + horaires
|   |   +-- src/ (main, config, scheduler, jobs/)
|   +-- monitoring-worker/             # Monitoring systeme
|       +-- src/ (main, config, monitor)
|
|-- bots/
|   |-- automod-bot/                    # Bot auto-moderation
|   |   +-- src/
|   |       |-- main.rs, handler.rs, api_client.rs, config.rs
|   |       +-- detectors/              # spam, insult, link, phishing (avec tests)
|   |-- moderation-bot/                 # Bot moderation manuelle
|   |   +-- src/
|   |       +-- commands/               # warn, mute, ban, history
|   |-- security-bot/                   # Bot securite serveur
|   |   +-- src/
|   |       |-- raid_detector.rs        # Anti-raid (DashMap, thread-safe, avec tests)
|   |       +-- account_checker.rs      # Verification age compte
|   |-- stats-bot/                      # Bot statistiques + XP
|   |   +-- src/
|   |       |-- tracker.rs              # Cache local (RwLock + HashMap, avec tests)
|   |       +-- commands/               # stats.rs, level.rs
|   |-- ticket-bot/                     # Bot tickets support
|   |   +-- src/
|   |       +-- commands/               # ticket.rs
|   |-- image-bot/                      # Bot detection images IA
|   |   +-- src/
|   |       |-- main.rs, handler.rs, api_client.rs, config.rs
|   |-- voice-bot/                      # Bot salons vocaux dynamiques
|   |   +-- src/
|   |       |-- handlers/               # message.rs, voice.rs
|   |       |-- interactions/           # access_control, channel_management, co_admin, queue, setup, transfer, vote_kick
|   |       |-- state/                  # cooldown_tracker, flood_tracker, pending_channels, vote_tracker (tous avec tests)
|   |       +-- utils/                  # embeds.rs
|   |-- audit-bot/                      # Bot audit logs
|   |   +-- src/
|   |       +-- main.rs, handler.rs, api_client.rs, config.rs
|   +-- roles-bot/                      # Bot role panels + auto-roles
|       +-- src/
|           +-- commands/               # roles_panel.rs
|
|-- ai/                                 # Entrainement IA
|   |-- requirements.txt                # Deps Python (torch, transformers, onnx)
|   |-- .gitignore                      # Exclut datasets, checkpoints, exports
|   |-- training/
|   |   |-- vision/                     # Modele detection images
|   |   |   |-- configs/train_config.yaml   # EfficientNetV2-S, 3 classes (safe/nsfw/illicit)
|   |   |   |-- scripts/               # dataset.py, train.py, export_onnx.py
|   |   |   |-- datasets/              # safe/ nsfw/ illicit/ (images)
|   |   |   |-- checkpoints/           # Meilleur modele .pt
|   |   |   +-- exports/               # vision_sentinel.onnx
|   |   +-- text/                       # Modele detection sentiments
|   |       |-- configs/train_config.yaml   # DistilBERT multilingual, 5 classes
|   |       |-- scripts/               # dataset.py, train.py, export_onnx.py
|   |       |-- datasets/              # neutral/ toxic/ (txt/jsonl)
|   |       |-- checkpoints/           # Meilleur modele
|   |       +-- exports/               # text_sentinel.onnx + tokenizer.json
|   +-- shared/                         # Utils partagees
|
|-- docs/                               # Documentation technique
|   |-- api.md, automod-bot.md, ticket-bot.md, desktop-app.md
|   +-- communication-bot-api.md, communication-app-api.md, ...
|
|-- docker-compose.yml                  # Orchestration complete (15 services)
|-- dev.sh                              # Script dev local
|-- .env.example                        # Template variables d'environnement
+-- README.md
```

---

## Schema base de donnees (PostgreSQL — 29 migrations)

### Tables principales

| Table                | Description                      | Colonnes cles                                                                      |
| -------------------- | -------------------------------- | ---------------------------------------------------------------------------------- |
| `rules`              | Regles de moderation par serveur | guild_id, flag_type (10 types), weight, thresholds (warn/delete/mute/ban), enabled |
| `infractions`        | Violations enregistrees          | guild_id, user_id, content, flags (JSONB), score, action, reason                   |
| `tickets`            | Systeme de tickets               | title, status, priority, author_id, assigned_to, category                          |
| `ticket_messages`    | Messages des tickets             | ticket_id (FK), author_name, author_role, content                                  |
| `security_events`    | Evenements de securite           | event_type (raid/suspicious), severity, user_ids (JSONB)                           |
| `moderation_actions` | Historique moderation manuelle   | moderator_id, target_id, action_type, gravity, duration                            |
| `user_stats`         | Stats utilisateurs               | message_count, voice_seconds                                                       |
| `voice_channels`     | Salons vocaux dynamiques         | owner_id, channel_type, is_locked, user_limit, co-admins                           |
| `conduct_points`     | Points de conduite               | points, penalties, regen                                                           |
| `levels`             | Configuration XP/niveaux         | xp_per_message, xp_per_voice_minute, level_up_channel                              |
| `user_levels`        | Niveaux utilisateurs             | xp, level                                                                          |
| `level_rewards`      | Recompenses par niveau           | level, role_id                                                                     |
| `guilds`             | Referentiel serveurs             | guild_id, name, icon, member_count                                                 |
| `bot_definitions`    | Definitions des bots             | bot_name, config_schema (JSON)                                                     |
| `bot_guild_config`   | Config per-guild par bot         | guild_id, bot_name, config_key, config_value                                       |
| `logs`               | Logs d'activite                  | level, bot, server, message                                                        |
| `audit_logs`         | Logs d'audit                     | guild_id, action, actor_id, target_id, details (JSONB)                             |
| `daily_activity`     | Snapshots quotidiens             | messages, voice_minutes, active_members, infractions                               |
| `hourly_activity`    | Activite par heure (heatmaps)    | guild_id, day, hour, messages, infractions                                         |
| `role_panels`        | Panels de roles                  | guild_id, channel_id, message_id, title, roles (JSONB)                             |
| `discord_roles`      | Roles Discord synchronises       | guild_id, id, name, color, position, permissions, managed, member_count             |

### Flag types supportes (10)

| Type         | Source      | Poids defaut | Description                    |
| ------------ | ----------- | ------------ | ------------------------------ |
| `spam`       | Bot automod | 3.0          | Majuscules, repetitions, flood |
| `insult`     | Bot automod | 5.0          | Dictionnaire regex FR/EN       |
| `link`       | Bot automod | 1.0          | URLs http/https, discord.gg    |
| `phishing`   | Bot automod | 7.0          | Liens suspects                 |
| `nsfw`       | IA Vision   | 8.0          | Images NSFW                    |
| `illicit`    | IA Vision   | 9.0          | Produits illicites             |
| `anger`      | IA Text     | 3.0          | Colere                         |
| `rage`       | IA Text     | 6.0          | Rage / haine                   |
| `threat`     | IA Text     | 8.0          | Menaces                        |
| `harassment` | IA Text     | 7.0          | Harcelement                    |

---

## Endpoints API (62+)

### Authentification

Toutes les routes (sauf `/health`) necessitent : `Authorization: Bearer <API_KEY>`
Si `API_KEY` est vide dans la config, l'auth est desactivee (mode dev).

### Analyse (bots)

| Methode | Route            | Description                                             |
| ------- | ---------------- | ------------------------------------------------------- |
| POST    | `/analyze`       | Analyse un message (scoring regles + inference IA text) |
| POST    | `/analyze/image` | Analyse une image (inference IA vision ONNX)            |

### Rules

| Methode | Route                         | Description                 |
| ------- | ----------------------------- | --------------------------- |
| GET     | `/rules/{guild_id}`           | Liste les regles du serveur |
| POST    | `/rules`                      | Creer/modifier une regle    |
| DELETE  | `/rules/{guild_id}/{rule_id}` | Supprimer une regle         |

### Infractions

| Methode | Route                     | Description                                                   |
| ------- | ------------------------- | ------------------------------------------------------------- |
| GET     | `/infractions/{guild_id}` | Liste les infractions (query: user_id, action, limit, offset) |

### Tickets

| Methode  | Route                        | Description          |
| -------- | ---------------------------- | -------------------- |
| GET/POST | `/api/tickets`               | Lister / creer       |
| GET      | `/api/tickets/{id}`          | Detail avec messages |
| POST     | `/api/tickets/{id}/messages` | Repondre             |
| PATCH    | `/api/tickets/{id}/close`    | Fermer               |
| PATCH    | `/api/tickets/{id}/assign`   | Assigner             |

### Security

| Methode | Route                  | Description              |
| ------- | ---------------------- | ------------------------ |
| POST    | `/api/security/events` | Reporter un evenement    |
| GET     | `/api/security/events` | Lister (query: guild_id) |

### Moderation

| Methode | Route                                          | Description       |
| ------- | ---------------------------------------------- | ----------------- |
| POST    | `/api/moderation/actions`                      | Logger une action |
| GET     | `/api/moderation/history/{guild_id}/{user_id}` | Historique        |

### Voice Channels (11 endpoints)

Gestion complete des salons vocaux dynamiques : CRUD, transfert, co-admins, whitelist, bans.

### Conduct (points de conduite)

Config, consultation, leaderboard, ajout/deduction points, historique.

### Levels / XP

Config, ajout XP, niveaux utilisateur, leaderboard, recompenses par niveau.

### Role Panels

Panels de roles avec selection, auto-roles a l'arrivee.

### Discord Roles

| Methode | Route                                | Description                                        |
| ------- | ------------------------------------ | -------------------------------------------------- |
| GET     | `/api/discord-roles/{guild_id}`      | Liste les roles Discord du serveur (synchronises)  |
| POST    | `/api/discord-roles/{guild_id}/sync` | Synchroniser les roles (appele par le community-bot) |

### Analytics (6 endpoints)

| Methode | Route                             | Description                                |
| ------- | --------------------------------- | ------------------------------------------ |
| GET     | `/api/analytics`                  | Toutes les analytics en une requete        |
| GET     | `/api/analytics/heatmap`          | Activite par heure x jour de la semaine    |
| GET     | `/api/analytics/actions`          | Distribution warn/delete/mute/ban (avec %) |
| GET     | `/api/analytics/top-infractors`   | Classement des plus sanctionnes            |
| GET     | `/api/analytics/moderation-trend` | Evolution quotidienne par type             |
| GET     | `/api/analytics/peak-hours`       | Heures les plus actives (moyennes)         |

Query params : `guild_id` (optionnel), `days` (1-90, defaut 30), `limit` (1-50, defaut 10)

### Stats

| Methode | Route                                  | Description                |
| ------- | -------------------------------------- | -------------------------- |
| POST    | `/api/stats/messages`                  | Enregistrer messages       |
| POST    | `/api/stats/voice`                     | Enregistrer temps vocal    |
| GET     | `/api/stats/{guild_id}/user/{user_id}` | Stats utilisateur          |
| GET     | `/api/stats/{guild_id}/overview`       | Vue d'ensemble (cache 60s) |
| GET     | `/api/stats/{guild_id}/leaderboard`    | Classement (max 50)        |

### Dashboard / Admin

Stats globales, logs, guilds, bot heartbeat, charts activite.

### Healthcheck

| Methode | Route     | Description                   |
| ------- | --------- | ----------------------------- |
| GET     | `/health` | Status PostgreSQL, Redis, API |

---

## Bots Discord (9 bots)

### Automod Bot — Auto-moderation

Detection locale rapide avant appel API :

- **Spam** : majuscules excessives, repetition caracteres/mots, flood (configurable par guild)
- **Insultes** : dictionnaire regex francais + anglais
- **Liens** : URLs http/https, invitations discord.gg
- **Phishing** : detection liens suspects

Si flags detectes -> appel `POST /analyze` -> scoring (regles + IA) -> execution action.
**Fallback** : si API injoignable, suppression locale du message.

### Moderation Bot — Moderation manuelle

| Commande                           | Description                    |
| ---------------------------------- | ------------------------------ |
| `/warn <user> <gravity> <reason>`  | Avertissement + DM             |
| `/mute <user> <reason> [duration]` | Timeout Discord (max 28 jours) |
| `/unmute <user>`                   | Retrait timeout                |
| `/ban <user> <reason> [duration]`  | Bannissement (DM avant ban)    |
| `/unban <user_id>`                 | Debannissement                 |
| `/history <user>`                  | Historique moderation          |

### Security Bot — Securite serveur

- **Anti-raid** : detection joins massifs (configurable), activation verification, alerte
- **Comptes suspects** : flag comptes < 24h (configurable)
- **Quarantaine** : role restrictif assigne aux comptes suspects/raid, retrait apres captcha
- **Captcha** : verification par bouton en DM, kick automatique si timeout (defaut 5min)
- **Slowmode auto** : activation slowmode sur tous les salons texte pendant un raid, revert automatique

### Image Bot — Detection images IA

- Intercepte tous les attachments images (jpg, png, gif, webp, bmp) + embeds
- Telecharge, encode base64, envoie a `POST /analyze/image`
- Detection magic bytes pour le content type
- **Fallback** : suppression preventive si API down

### Stats Bot — Statistiques + XP

| Commande               | Description                                      |
| ---------------------- | ------------------------------------------------ |
| `/stats user [target]` | Stats utilisateur (messages, vocal, infractions) |
| `/stats server`        | Stats globales                                   |
| `/stats top [limit]`   | Classement (1-25)                                |
| `/level [user]`        | Niveau et XP                                     |

Tracking automatique messages + vocal + XP.

### Ticket Bot — Tickets support

| Commande                                       | Description        |
| ---------------------------------------------- | ------------------ |
| `/ticket create <title> <category> [priority]` | Thread prive       |
| `/ticket close`                                | Fermer et archiver |
| `/ticket assign <moderator>`                   | Assigner           |

### Voice Bot — Salons vocaux dynamiques

Gestion complete : creation automatique, permissions, co-admins, vote kick, whitelist/ban, file d'attente, anti-flood.

### Audit Bot — Logs d'audit

Tracking des actions Discord (bans, kicks, modifications roles, etc.) et envoi a l'API.

### Roles Bot — Panels de roles

Gestion de panels de roles avec boutons de selection + auto-roles a l'arrivee de nouveaux membres.

---

## Systeme de scoring

Le scoring determine l'action a executer. Combine scoring par regles + inference IA.

**Scoring regles (bot)** : `score_bot = somme(poids des flags actifs)`

**Scoring IA text** : `score_ia = somme(poids * confiance)` pour chaque sentiment detecte (seuil: 50%)

**Score combine** : `score_total = score_bot + score_ia`

**Seuils par defaut** :

- `>= 2.0` -> warn | `>= 4.0` -> delete | `>= 6.0` -> mute (10 min) | `>= 9.0` -> ban

Tous les seuils et poids sont configurables par serveur via les regles.

---

## Inference IA (ONNX)

### Modele Vision — Detection images

| Propriete     | Valeur                                     |
| ------------- | ------------------------------------------ |
| Architecture  | EfficientNetV2-S                           |
| Classes       | safe, nsfw, illicit                        |
| Input         | Image 224x224 normalisee ImageNet          |
| Format        | ONNX (opset 17)                            |
| Preprocessing | Resize + normalisation (mean/std ImageNet) |

### Modele Text — Detection sentiments

| Propriete    | Valeur                                   |
| ------------ | ---------------------------------------- |
| Architecture | DistilBERT multilingual                  |
| Classes      | neutral, anger, rage, threat, harassment |
| Input        | Tokens (max 256) + attention mask        |
| Format       | ONNX (opset 17)                          |
| Tokenizer    | HuggingFace tokenizers (Rust)            |

Les modeles sont charges au demarrage de l'API. Si absents, l'API fonctionne en mode degrade (scoring regles uniquement).

### Config IA per-guild

Les seuils de confiance IA sont configurables par serveur via la table `ia_config` et l'UI desktop.

| Propriete         | Defaut | Description                                            |
| ----------------- | ------ | ------------------------------------------------------ |
| `text_enabled`    | true   | Active/desactive l'inference text (sentiments)         |
| `text_threshold`  | 0.5    | Seuil de confiance minimum pour les sentiments IA      |
| `vision_enabled`  | true   | Active/desactive l'inference vision (NSFW/illicite)    |
| `vision_threshold`| 0.5    | Seuil de confiance minimum pour les classifications    |

**Endpoints** : `GET /api/ia-config/{guild_id}`, `PUT /api/ia-config/{guild_id}`

**Desktop** : Page dediee avec sliders pour ajuster les seuils en temps reel (route `/ia-config`).

---

## Gateway WebSocket

Service dedie au temps reel, separe de l'API.

**Architecture** : API publie sur Redis (`PUBLISH sentinel:events`) -> Gateway ecoute (`SUBSCRIBE`) -> broadcast aux clients WebSocket.

| Propriete         | Valeur                                     |
| ----------------- | ------------------------------------------ |
| Port              | 3001                                       |
| Auth              | `?token=<api_key>`                         |
| Max connexions    | Configurable (defaut: 1000)                |
| Reconnexion Redis | Automatique (backoff 2s)                   |
| Healthcheck       | `GET /health` (status + connected_clients) |

**Events broadcasts** : `infraction_new`, `ticket_new`, `ticket_message`, `ticket_closed`, `ticket_assigned`, `security_event`, `moderation_action`

---

## Desktop App (Tauri)

### Pages (27 ecrans)

| Page           | Fonctionnalite                                                   |
| -------------- | ---------------------------------------------------------------- |
| Setup          | Configuration initiale (URL API + credentials Discord OAuth)     |
| Login          | Connexion Discord OAuth                                          |
| Dashboard      | Stats globales + graphiques Chart.js                             |
| Logs           | Logs d'activite avec filtres (niveau, bot)                       |
| Logs API       | Logs specifiques a l'API                                         |
| Logs Bots      | Logs specifiques aux bots Discord                                |
| Logs WebSocket | Logs specifiques au gateway WebSocket                            |
| Logs Workers   | Logs specifiques aux workers                                     |
| Infractions    | Table des infractions avec details                               |
| Rules          | Gestion des regles (toggle, edition seuils/poids)                |
| Bans           | Liste des bans avec recherche et filtres                         |
| Moderation     | Application d'actions + consultation historique                  |
| Security       | Monitoring evenements de securite en temps reel                  |
| Tickets        | Gestion tickets (liste, detail, reponse, fermeture, assignation) |
| Voice Channels | Monitoring salons vocaux dynamiques                              |
| Conduct        | Points de conduite + leaderboard                                 |
| Levels         | Systeme XP/niveaux + recompenses                                 |
| Role Panels    | Gestion panels de roles + auto-roles                             |
| Watched Users  | Surveillance utilisateurs avec dossiers                          |
| Audit          | Logs d'audit                                                     |
| Settings       | Configuration (URL API, cle, auto-refresh, logout)               |
| Bot Config     | Configuration par bot et par serveur                             |
| Worker Config  | Configuration et monitoring des workers                          |
| Analytics      | Heatmap, trends moderation, top infracteurs, peak hours, distribution |
| IA Config      | Seuils de confiance IA par serveur (sliders text + vision)       |
| AI Training    | Interface d'entrainement IA (datasets, progression, graphiques)  |
| Discord Roles  | Visualisation des roles Discord du serveur (sync par le bot)     |

### Frontend Vue 3

- **Atomic Design** : 9 atoms, 5 molecules, 10 organisms, 1 template, 27 pages
- **29 composables** : useAuth, useDashboard, useRules, useInfractions, useModeration, useTickets, useVoiceChannels, useBans, useConduct, useLevels, useRolePanels, useSecurity, useWatchedUsers, useAuditLogs, useLogs, useDashboardCharts, useRealtime, useNotifications, useAiTraining, useAnalytics, useFetch, useFormatDate, useGuildSelector, useIaConfig, usePagination, useGuildFetch, useSearch, useConfirm, useDiscordRoles
- **Notifications natives** desktop via WebSocket
- **Graphiques** Chart.js (trends, distributions)

### Backend Tauri (Rust)

- 19 services applicatifs
- Architecture hexagonale (domain/ports/adapters)
- Stockage local LMDB (config persistante)
- WebSocket temps reel avec auto-reconnect
- Mock adapter pour mode hors-ligne

---

## Workers (3 services specialises)

Architecture distribuee avec 3 workers dedies, chacun avec son propre scheduler et heartbeat.

### Moderation Worker

Gere les taches liees a la moderation et aux sanctions.

| Tache                | Intervalle | Description                                    |
| -------------------- | ---------- | ---------------------------------------------- |
| `conduct_regen`      | 1h         | Regeneration points de conduite (weekly/monthly)|
| `cleanup_bans`       | 60s        | Nettoyage bans vocaux expires                  |
| `sync_ban_proposals` | periodique | Synchronisation propositions de ban             |

### Analytics Worker

Gere les snapshots et statistiques d'activite.

| Tache             | Intervalle | Description                                    |
| ----------------- | ---------- | ---------------------------------------------- |
| `daily_snapshot`  | 5min       | Snapshots activite quotidienne par guild        |
| `hourly_snapshot` | periodique | Snapshots activite par heure (heatmaps)        |

### Monitoring Worker

Surveillance systeme et sante des services.

| Composant | Description                       |
| --------- | --------------------------------- |
| `monitor` | Monitoring sante et metriques     |

### Configuration workers

| Variable                  | Defaut          | Description                    |
| ------------------------- | --------------- | ------------------------------ |
| `DATABASE_URL`            | requis          | URL PostgreSQL                 |
| `REDIS_URL`               | requis          | URL Redis                      |
| `CONDUCT_REGEN_INTERVAL`  | `3600`          | Intervalle regen conduite (s)  |
| `BAN_CLEANUP_INTERVAL`    | `60`            | Intervalle cleanup bans (s)    |
| `DAILY_SNAPSHOT_INTERVAL` | `300`           | Intervalle snapshots (s)       |
| `SHUTDOWN_TIMEOUT`        | `10`            | Timeout arret gracieux (s)     |

---

## Middleware API

| Middleware           | Description                                                           |
| -------------------- | --------------------------------------------------------------------- |
| Auth                 | Bearer token, mode dev si API_KEY vide                                |
| Rate Limiting (HTTP) | Token bucket par IP (defaut: 50 req/s, burst 10x), header Retry-After |
| Rate Limiting (IA)   | Semaphore + token bucket pour inference ONNX (defaut: 4 concurrent, 20/s) |
| CORS                 | Origins configurables                                                 |
| Body Limit           | Defaut: 1 MB                                                          |
| Tracing              | Logs structures (method, URI, request_id, status, latency_ms)         |
| Request ID           | Propagation x-request-id                                              |

### Variables rate limiting inference

| Variable                   | Defaut | Description                                  |
| -------------------------- | ------ | -------------------------------------------- |
| `INFERENCE_MAX_CONCURRENT` | `4`    | Nombre max d'inferences ONNX simultanées     |
| `INFERENCE_MAX_PER_SEC`    | `20`   | Nombre max d'inferences par seconde (0=off)  |

Retourne HTTP 429 si le rate limit est depasse.

---

## Deploiement

### Docker Compose (15 services)

```bash
# Demarrer toute la stack
docker-compose up -d

# Avec monitoring (PgAdmin :5050, Redis Commander :8081)
docker-compose --profile monitoring up -d
```

Services :

- **postgres** (16-alpine) — port 5432
- **redis** (7-alpine) — port 6379
- **api** — port 3000
- **gateway** — port 3001
- **moderation-worker** — conduite, bans, sync ban proposals
- **analytics-worker** — snapshots quotidiens + horaires
- **monitoring-worker** — surveillance systeme
- **automod-bot, moderation-bot, security-bot, ticket-bot, image-bot, voice-bot, stats-bot, audit-bot, roles-bot**

### Variables d'environnement (.env)

```env
# Infrastructure
POSTGRES_PASSWORD=sentinel_secret
REDIS_PASSWORD=sentinel_redis

# API
API_KEY=your_api_key_here

# IA / Inference ONNX (optionnel)
VISION_MODEL_PATH=./models/vision_sentinel.onnx
TEXT_MODEL_PATH=./models/text_sentinel.onnx
TEXT_TOKENIZER_PATH=./models/tokenizer.json
TEXT_MAX_LENGTH=256

# Bots (un token par bot)
AUTOMOD_DISCORD_TOKEN=...
MODERATION_DISCORD_TOKEN=...
SECURITY_DISCORD_TOKEN=...
TICKET_DISCORD_TOKEN=...
IMAGE_DISCORD_TOKEN=...
VOICE_DISCORD_TOKEN=...
STATS_DISCORD_TOKEN=...
AUDIT_DISCORD_TOKEN=...
ROLES_DISCORD_TOKEN=...
```

### Developpement local

```bash
bash dev.sh          # Lance API + bots + desktop en parallele
# ou individuellement :
cd services/api && cargo run
cd services/gateway && cargo run
cd bots/automod-bot && cargo run
cd apps/desktop && npm run tauri dev
```

---

## Tests

| Module                      | Tests | Couverture                                          |
| --------------------------- | ----- | --------------------------------------------------- |
| API — ScoringService        | 21    | Tous les flags, poids, seuils, regles custom        |
| API — InferenceService      | 10    | Softmax, mode degrade, classify sans modele         |
| API — TextTokenizer         | 4     | Mode degrade, tokenize sans tokenizer               |
| API — AnalyzeImageService   | 4     | Preprocessing image, normalisation                  |
| API — AnalyzeMessageService | 6     | Thresholds, inference text, construction            |
| API — Value Objects         | 12    | Action, FlagType, DetectionFlags (serde, roundtrip) |
| API — Level                 | 3     | XP calculs                                          |
| Gateway — Broadcaster       | 6     | Subscribe, unsubscribe, max connections, broadcast  |
| Automod Bot — Detectors     | ~20   | Spam, insult, link, phishing                        |
| Security Bot — Raid         | ~5    | Detection joins massifs                             |
| Stats Bot — Tracker         | ~5    | Cache local                                         |
| Voice Bot — State           | ~15   | Cooldown, flood, pending, vote                      |

**Total : ~110+ tests unitaires** sur 20 fichiers avec `#[cfg(test)]`

---

## Bonnes pratiques du projet

- **Jamais de logique metier dans les bots** : les bots sont des interfaces legeres
- **Toujours passer par l'API** : centralisation des decisions
- **Architecture hexagonale** : separation stricte domain/ports/adapters
- **Gateway dedie** : WebSocket temps reel decouple de l'API REST
- **Inference IA gracieuse** : si modeles absents, fallback sur scoring regles
- **Gestion d'erreurs** : `thiserror` pour les erreurs domain, conversion auto vers HTTP
- **Cache** : Redis pour regles (TTL 5min) et stats (TTL 60s), invalidation a la modification
- **Fallback** : si API injoignable, le bot prend une decision locale de securite
- **Rate limiting** : token bucket par IP avec burst configurable
- **Observabilite** : logs structures avec request_id, format JSON optionnel

---

## Suivi des features

### Termine

- [x] API Backend — Architecture hexagonale, 62+ endpoints, 22 handlers, helpers.rs, 14 use cases, 29 migrations
- [x] Automod Bot — Detection spam/insultes/liens/phishing + appel API + fallback
- [x] Moderation Bot — /warn /mute /ban /unmute /unban /history avec DM et logging
- [x] Security Bot — Anti-raid + comptes suspects + alertes
- [x] Stats Bot — Tracking messages/vocal + /stats + XP/levels
- [x] Ticket Bot — /ticket create, close, assign avec threads prives
- [x] Image Bot — Detection images NSFW/illicites via API + fallback
- [x] Voice Bot — Salons dynamiques, vote kick, co-admins, whitelist/ban
- [x] Audit Bot — Tracking audit logs Discord
- [x] Roles Bot — Panels de roles + auto-roles
- [x] Desktop App — 27 pages, OAuth Discord, WebSocket temps reel, notifications natives
- [x] Gateway WebSocket — Service dedie, Redis pub/sub, auto-reconnect, limite connexions
- [x] Inference IA ONNX — Vision (NSFW/illicite) + Text (sentiments) integres dans l'API
- [x] Tokenizer Rust — HuggingFace tokenizers pour inference text
- [x] Scoring combine — Regles bot + inference IA text avec ponderation par confiance
- [x] Analytics complet — Heatmap, distribution actions, top infracteurs, trends, peak hours
- [x] Systeme de conduite — Points, penalties, regeneration, leaderboard
- [x] Systeme XP/Levels — XP par message/vocal, niveaux, recompenses roles
- [x] Watched Users — Surveillance avec dossiers complets
- [x] Bot Config — Configuration per-guild par bot depuis l'app desktop
- [x] Docker Compose — 15 services orchestres
- [x] Tests unitaires — 110+ tests (API, gateway, bots)
- [x] Multi-stage Docker builds — Images Alpine optimisees
- [x] Workers specialises — 3 workers dedies (moderation, analytics, monitoring) avec heartbeat
- [x] Anti-raid avance — Quarantaine (role restrictif), captcha DM (bouton), slowmode auto, kick timeout
- [x] Config seuils IA per-guild — Table ia_config, endpoints API, page desktop avec sliders, seuils dynamiques
- [x] Rate limiting inference — Semaphore (4 concurrent) + token bucket (20/s), HTTP 429, configurable
- [x] Page analytics desktop — Heatmap, trends moderation, top infracteurs, peak hours, distribution actions
- [x] Logs segmentes — Pages dediees par source (API, Bots, WebSocket, Workers)
- [x] AI Training UI — Interface desktop pour entrainement IA (datasets, progression, graphiques)
- [x] Worker Config UI — Page de configuration et monitoring des workers
- [x] Guild Selector — Selection de serveur globale dans l'app desktop
- [x] Refactoring DRY desktop — useGuildFetch, useSearch, useConfirm, LogsTemplate, FormField, LoadingState, EmptyState, ConfirmDialog, CSS variables
- [x] Refactoring API — helpers.rs (map_to_dtos, normalize_limit, ok_response), DiscordApiService, route nesting
- [x] Refactoring Gateway — Race condition fixee, exponential backoff Redis, graceful shutdown timeout, config env
- [x] Refactoring Workers — Crate sentinel-worker-common, 3 workers refactores, fix bug analytics interval
- [x] Ameliorations ML — max_length 128, early stopping vision, test split, augmentation enrichie, class weights, AMP, LR finder, shared metrics
- [x] Embeds Discord uniformes — bots/shared/src/embeds.rs, 8 bots migres vers embeds riches
- [x] Centralisation variants — actionLabel, typeLabel, eventVariant, eventLabel, eventIcon dans variants.ts
- [x] Discord Roles — Sync bot→API→desktop, page DiscordRolesPage, migration 029

### En cours

- [ ] Collecter les datasets IA — Images (safe/nsfw/illicit) et textes (neutral/toxic)
- [ ] Entrainer les modeles IA — `python train.py` + `python export_onnx.py`

### A faire

- [ ] CI/CD — GitHub Actions (lint, test, build, deploy)
- [ ] Tests e2e — Integration end-to-end (API + bots + DB)
- [ ] Infrastructure Kubernetes — Helm charts, HPA, monitoring
- [ ] Monitoring avance — Prometheus, Grafana, alerting
- [ ] Backup automatique — Snapshots PostgreSQL + export config

> Roadmap detaillee des features futures : [docs/ROADMAPV2.md](docs/ROADMAPV2.md)
> Regles metier des bots + 87 features planifiees + modifications API/desktop : [docs/bots-business-logic.md](docs/bots-business-logic.md)
