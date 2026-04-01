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
|  Ticket Bot    |  | Progression Bot|  |   Desktop App       |
|  /ticket       |  |  /stats        |  |   Tauri + Vue 3     |
|  create/close  |  |  user/server   |  |   Admin complete    |
+----------------+  +----------------+  +---------------------+
                              |
+----------------+  +----------------+  +----------------+
|  Voice Bot     |  |  Audit Bot     |  | Community Bot  |
|  Salons dyn.   |  |  Logs audit    |  |  Role panels   |
|  Vote kick     |  |  Tracking      |  |  Auto-roles    |
+----------------+  +----------------+  +----------------+
```

**Philosophie** : Bots = interfaces (legers, pas de logique metier) | API = cerveau (decisions + IA) | Gateway = temps reel | App = controle (admin)

---

## Stack technique

| Composant            | Technologie                              | Details                                                    |
| -------------------- | ---------------------------------------- | ---------------------------------------------------------- |
| API Backend          | Rust, Axum 0.8, Tokio                    | Architecture hexagonale, 84+ endpoints, 25 handlers, 17 use cases, helpers.rs, 52 migrations |
| Gateway WebSocket    | Rust, Axum 0.8, Redis pub/sub            | Service dedie temps reel, auto-reconnect                   |
| Workers (3 + common) | Rust, Tokio, Redis, sqlx                 | 3 workers specialises + crate partagee sentinel-worker-common |
| Base de donnees      | PostgreSQL 16                            | 37 migrations, 22+ tables                                  |
| Cache                | Redis 7                                  | Cache regles TTL 5min, stats TTL 60s, pub/sub events       |
| Inference IA         | ONNX Runtime 2.0, ndarray, tokenizers    | Vision (NSFW/illicite) + Text (sentiments)                 |
| Automod Bot          | Rust, Serenity 0.12, DashMap             | Detection spam/insultes/liens/phishing/emoji/mentions/fichiers/unicode + leet speak + mode nuit + slowmode adaptatif |
| Moderation Bot       | Rust, Serenity 0.12, DashMap             | /warn /mute /ban /history /note /call /context /appeal /export /massmute /massban + mode apprenti + autocomplete templates |
| Security Bot         | Rust, Serenity 0.12, DashMap             | Anti-raid, quarantaine, captcha (bouton/math), slowmode auto, lockdown, alt detection |
| Progression Bot        | Rust, Serenity 0.12, DashMap             | /stats /level + XP cooldown + streaks + multiplicateurs + badges + tracking |
| Ticket Bot           | Rust, Serenity 0.12, DashMap             | /ticket create, close, assign + SLA tracking, satisfaction survey, templates, FAQ, escalade auto, transcript MD/HTML |
| Image Bot            | Rust, Serenity 0.12, DashMap             | Detection images NSFW/illicites via API + hash cache + seuils salon + queue retry |
| Voice Bot            | Rust, Serenity 0.12                      | Salons dynamiques, vote kick, co-admins, whitelist/ban, invite links, AFK auto-move, themes, stage mode |
| Audit Bot            | Rust, Serenity 0.12, DashMap             | Tracking audit logs, cache messages, anomaly detection, permission diffs, rapport hebdo |
| Community Bot          | Rust, Serenity 0.12, DashMap             | Role panels + auto-roles + exclusifs + prerequis + temp roles + parrainage + sync Discord |
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
|   |       |-- adaptive_slowmode.rs     # Slowmode adaptatif (DashMap, avec tests)
|   |       +-- detectors/              # spam, insult, link, phishing, unicode (avec 214 tests)
|   |-- moderation-bot/                 # Bot moderation manuelle avancee
|   |   +-- src/
|   |       |-- commands/               # warn, mute, ban, history, notes, call, context, appeal, export, mass (25 tests)
|   |       +-- reason_templates.rs     # Templates raisons + autocomplete (avec tests)
|   |-- security-bot/                   # Bot securite serveur
|   |   +-- src/
|   |       +-- security/               # Modules securite (tous avec tests)
|   |           |-- raid_detector.rs    # Anti-raid (DashMap, thread-safe)
|   |           |-- raid_analyzer.rs    # Analyse patterns raid (Levenshtein, avatars, creation cluster)
|   |           |-- account_checker.rs  # Verification age compte
|   |           |-- quarantine.rs       # Gestion quarantaine
|   |           |-- captcha.rs          # Captcha bouton + math (4 choix)
|   |           |-- slowmode.rs         # Slowmode auto pendant raid
|   |           |-- lockdown.rs         # Lockdown auto (deny SEND_MESSAGES @everyone)
|   |           +-- alt_detector.rs     # Detection alt accounts (Levenshtein + creation proximity)
|   |-- progression-bot/                 # Bot progression (stats, XP, niveaux, streaks, badges)
|   |   +-- src/
|   |       |-- tracker.rs              # Cache local (RwLock + HashMap, avec tests)
|   |       |-- xp_cooldown.rs          # Anti-farm XP (DashMap, avec tests)
|   |       |-- streaks.rs              # Streaks jours consecutifs (DashMap, avec tests)
|   |       |-- multipliers.rs          # Multiplicateurs XP salon/role (avec tests)
|   |       |-- badges.rs               # 8 badges debloquables (avec tests)
|   |       +-- commands/               # stats.rs, level.rs
|   |-- ticket-bot/                     # Bot tickets support avance
|   |   +-- src/
|   |       |-- commands/               # ticket.rs (1600+ lignes, 59 tests)
|   |       |-- sla.rs                  # SLA tracker (DashMap, first_response, breach detection)
|   |       |-- satisfaction.rs         # Sondage satisfaction 1-5 etoiles
|   |       |-- templates.rs            # Templates reponses rapides (parse config)
|   |       |-- faq.rs                  # FAQ suggestions avant creation ticket
|   |       +-- transcript.rs           # Generation transcript Markdown/HTML (avec tests)
|   |-- image-bot/                      # Bot detection images IA
|   |   +-- src/
|   |       |-- main.rs, handler.rs, api_client.rs, config.rs
|   |-- voice-bot/                      # Bot salons vocaux dynamiques
|   |   +-- src/
|   |       |-- handlers/               # message.rs, voice.rs
|   |       |-- interactions/           # access_control, channel_management, co_admin, queue, setup, transfer, vote_kick
|   |       |-- state/                  # cooldown_tracker, flood_tracker, pending_channels, vote_tracker, afk_tracker (tous avec tests)
|   |       |-- tasks/                  # afk_sweep.rs (tache de fond AFK auto-move)
|   |       +-- embeds.rs              # Logging embed Discord
|   |-- audit-bot/                      # Bot audit logs avances
|   |   +-- src/
|   |       |-- main.rs, handler.rs, api_client.rs, config.rs, audit_event.rs
|   |       |-- message_cache.rs        # Cache LRU messages (DashMap, avec tests)
|   |       |-- anomaly.rs              # Detection anomalies temps reel (avec tests)
|   |       |-- permission_diff.rs      # Diff permissions lisible (avec tests)
|   |       |-- weekly_report.rs        # Stats hebdo + format embed (avec tests)
|   |       +-- handlers/               # channel, guild, invite, member, message, role, thread, voice
|   +-- community-bot/                   # Bot communaute (roles, parrainage, onboarding)
|       +-- src/
|           |-- commands/               # roles_panel.rs, sponsor.rs
|           |-- exclusive_groups.rs     # Groupes mutuellement exclusifs (avec tests)
|           |-- prerequisites.rs        # Prerequis conditionnels (avec tests)
|           |-- temp_roles.rs           # Roles temporaires avec expiration (avec tests)
|           +-- sponsorship.rs          # Systeme de parrainage (avec tests)
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

## Schema base de donnees (PostgreSQL — 52 migrations)

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
| `voice_channel_invite_links` | Liens d'invitation vocaux | voice_channel_id (FK), code (UNIQUE), max_uses, current_uses, expires_at, revoked   |
| `voice_channel_themes`       | Themes de salons vocaux  | guild_id, name, emoji, channel_name_template, member_limit, visibility, bitrate     |
| `strike_config`    | Config escalade par serveur        | guild_id, window_secs, thresholds (JSONB), enabled                                 |
| `user_strikes`     | Strikes individuels                | guild_id, user_id, reason, source, infraction_id, expires_at                       |
| `user_notes`       | Notes moderateur sur utilisateurs  | guild_id, user_id, author_id, author_name, content, category                       |
| `sanction_reminders` | Rappels sanctions temporaires    | guild_id, moderator_id, target_id, action_type, remind_at, expires_at, status      |

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

## Endpoints API (84+)

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

### Voice Channels (19 endpoints)

Gestion complete des salons vocaux dynamiques : CRUD, transfert, co-admins, whitelist, bans, liens d'invitation, themes.

| Methode | Route                                            | Description                          |
| ------- | ------------------------------------------------ | ------------------------------------ |
| GET     | `/api/voice-channels/by-channel/{id}/invites`    | Lister les liens d'invitation actifs |
| POST    | `/api/voice-channels/by-channel/{id}/invites`    | Creer un lien d'invitation           |
| DELETE  | `/api/voice-channels/by-channel/{id}/invites/{link_id}` | Revoquer un lien             |
| POST    | `/api/voice-channels/invites/{code}/use`         | Utiliser un code d'invitation        |
| GET     | `/api/voice-channels/themes/{guild_id}`          | Lister les themes du serveur         |
| POST    | `/api/voice-channels/themes/{guild_id}`          | Creer un theme                       |
| PATCH   | `/api/voice-channels/themes/{guild_id}/{id}`     | Modifier un theme                    |
| DELETE  | `/api/voice-channels/themes/{guild_id}/{id}`     | Supprimer un theme                   |

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
| GET     | `/api/stats/{guild_id}/voice-stats`    | Stats vocales par salon (temp/perm, 30j, top 20) |

### Strikes (escalade progressive)

| Methode | Route                              | Description                                              |
| ------- | ---------------------------------- | -------------------------------------------------------- |
| GET     | `/api/strikes/config/{guild_id}`   | Config escalade du serveur                               |
| PUT     | `/api/strikes/config/{guild_id}`   | Modifier la config escalade                              |
| GET     | `/api/strikes/{guild_id}/{user_id}`| Strikes actifs d'un utilisateur                          |
| POST    | `/api/strikes`                     | Ajouter un strike (retourne escalation si seuil atteint) |
| DELETE  | `/api/strikes/{guild_id}/{user_id}`| Reset les strikes d'un utilisateur                       |

### Notes utilisateur

| Methode | Route                              | Description               |
| ------- | ---------------------------------- | ------------------------- |
| POST    | `/api/notes`                       | Ajouter une note          |
| GET     | `/api/notes/{guild_id}/{user_id}`  | Notes d'un utilisateur    |
| DELETE  | `/api/notes/{id}`                  | Supprimer une note        |

### Rappels sanctions temporaires

| Methode | Route                     | Description                |
| ------- | ------------------------- | -------------------------- |
| POST    | `/api/reminders`          | Creer un rappel            |
| GET     | `/api/reminders/pending`  | Rappels en attente d'envoi |
| GET     | `/api/reminders/{guild_id}` | Rappels d'un serveur     |

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
- **Insultes** : dictionnaire regex francais + anglais + normalisation leet speak (`c0nnard`, `f*ck`, `@$$hole`) + mots personnalises
- **Liens** : URLs http/https, invitations discord.gg (configurable)
- **Phishing** : detection liens suspects + typosquatting Discord/Steam + scam patterns
- **Emoji spam** : detection emojis Unicode + custom Discord excessifs (seuil configurable)
- **Mentions excessives** : detection `<@id>`, `@everyone`, `@here` (seuil configurable)
- **Fichiers suspects** : detection pieces jointes dangereuses (.exe, .bat, .ps1, .dll, etc.) + extensions custom
- **Unicode abuse** : detection zalgo text (combining characters excessifs), caracteres invisibles (zero-width), homoglyphes (melange latin/cyrillique)
- **Mode nuit** : seuils de detection divises par 2 pendant les heures configurees (ex: 22h-8h UTC)
- **Slowmode adaptatif** : activation automatique du slowmode quand l'activite d'un salon depasse un seuil configurable

Si flags detectes -> appel `POST /analyze` -> scoring (regles + IA) -> execution action.
**Fallback** : si API injoignable, suppression locale du message.

### Moderation Bot — Moderation manuelle avancee

| Commande                           | Description                    |
| ---------------------------------- | ------------------------------ |
| `/warn <user> <gravity> <reason>`  | Avertissement + DM + escalation strikes |
| `/mute <user> <reason> [duration]` | Timeout Discord (max 28 jours) |
| `/unmute <user>`                   | Retrait timeout                |
| `/ban <user> <reason> [duration]`  | Bannissement (DM avant ban)    |
| `/unban <user_id>`                 | Debannissement                 |
| `/history <user>`                  | Historique moderation          |
| `/note <user> <content> [cat]`     | Note moderateur (general/warning/positive/context) |
| `/call <user> [reason]`            | Convocation dans un salon prive temporaire |
| `/context <message_id> [count]`    | Afficher les messages autour d'un message |
| `/appeal`                          | Contester une sanction (cree un ticket automatiquement) |
| `/export <user> [format]`          | Exporter l'historique en CSV ou JSON |
| `/massmute <users> <reason> [dur]` | Mute plusieurs utilisateurs en masse |
| `/massban <users> <reason>`        | Bannir plusieurs utilisateurs en masse |

Features avancees :
- **Mode apprenti** : les moderateurs avec le role apprenti proposent des actions au lieu de les executer — boutons Approuver/Rejeter pour les seniors
- **Templates de raisons** : raisons predefinies avec autocomplete Discord (configurable par serveur)
- **Appel de sanction** : bouton "Contester" dans les DMs de sanction + commande `/appeal` → creation automatique de ticket

### Security Bot — Securite serveur

- **Anti-raid** : detection joins massifs (configurable), activation verification, alerte
- **Analyse pattern raid** : detection noms similaires (Levenshtein), avatars par defaut, dates de creation clusterisees, score composite 0-100
- **Comptes suspects** : flag comptes < 24h (configurable)
- **Quarantaine** : role restrictif assigne aux comptes suspects/raid, retrait apres captcha
- **Captcha** : verification par bouton en DM ou captcha math (4 choix), kick automatique si timeout (defaut 5min)
- **Slowmode auto** : activation slowmode sur tous les salons texte pendant un raid, revert automatique
- **Lockdown auto** : desactivation `SEND_MESSAGES` pour @everyone pendant un raid, restauration des permissions apres expiration
- **Detection alt accounts** : comparaison des nouveaux membres avec les bans recents (distance Levenshtein + proximite date de creation), quarantaine automatique si suspicion

### Image Bot — Detection images IA avancee

- Intercepte tous les attachments images (jpg, png, gif, webp, bmp) + embeds
- Telecharge, encode base64, envoie a `POST /analyze/image`
- Detection magic bytes pour le content type
- **Hash cache** : evite d'analyser deux fois la meme image (TTL configurable)
- **Seuils par salon** : tolerance configurable par channel (ex: #art plus tolerant)
- **Screenshot detection** : flag `is_screenshot` dans la requete pour OCR cote API
- **GIF anime** : flag `is_animated` pour adaptation du traitement API
- **File d'attente** : queue avec retry (3 tentatives, 10s entre chaque) au lieu de suppression preventive (opt-in)
- **Fallback** : suppression preventive si API down et queue non activee

### Progression Bot — Statistiques, XP et progression

| Commande               | Description                                      |
| ---------------------- | ------------------------------------------------ |
| `/stats user [target]` | Stats utilisateur (messages, vocal, infractions) |
| `/stats server`        | Stats globales                                   |
| `/stats top [limit]`   | Classement (1-25)                                |
| `/level [user]`        | Niveau, XP, streak et badges                     |
| `/level top [limit]`   | Classement niveaux                               |

Features avancees :
- **XP Cooldown** : anti-farm, 1 seul gain XP par message toutes les 60s (configurable)
- **Streaks** : bonus XP pour jours consecutifs d'activite (+10% par semaine, max 1.5x)
- **Multiplicateurs XP** : par salon et par role (configurables par serveur)
- **Badges** : 8 badges debloquables (Bavard, Orateur, Vocal, DJ, Etoile, Legende, En feu, Diamant)

### Ticket Bot — Tickets support avance

| Commande                                       | Description        |
| ---------------------------------------------- | ------------------ |
| `/ticket create <title> <category> [priority]` | Thread prive       |
| `/ticket close`                                | Fermer et archiver |
| `/ticket assign <moderator>`                   | Assigner           |

Features avancees :
- **SLA Tracking** : mesure du temps de premiere reponse staff et temps de resolution par ticket
- **Satisfaction survey** : sondage 1-5 etoiles envoye en DM apres fermeture du ticket
- **Templates de reponses** : reponses rapides predefinies configurables par serveur (format `label|contenu`)
- **Escalade automatique** : augmentation de priorite si pas de reponse staff dans un delai configurable
- **Transcript Markdown/HTML** : generation de fichiers transcript formates avec CSS, XSS-safe
- **FAQ avant creation** : affichage de FAQ configurables avant la creation du ticket, bouton "Creer quand meme"

### Voice Bot — Salons vocaux dynamiques

Gestion complete : creation automatique, permissions, co-admins, vote kick, whitelist/ban, file d'attente, anti-flood, AFK auto-move, liens d'invitation par code, themes de salon.

- **Mode stage** : mode presentation ou seul le proprietaire (et les speakers designes) peut parler. Les autres membres ecoutent. Le proprietaire peut donner/retirer la parole via un bouton "Donner parole" (user select). Simulation par permissions Discord (deny SPEAK a @everyone). Activable par bouton dans le panneau de controle ou par defaut via un theme.
- **Themes de salon** : templates pre-configures par serveur (Gaming, Musique, Travail, etc.). Chaque theme definit : nom (template {user}), emoji, limite membres, visibilite, verrouille, queue, bitrate, slowmode, stage. A la creation, si plusieurs themes existent, un menu de selection est envoye en DM. Configuration CRUD depuis l'app desktop.
- **Liens d'invitation** : le proprietaire genere un code 8 caracteres (bouton "Lien" dans le panneau) avec duree configurable (15min/30min/1h/24h). N'importe qui utilise le code via `!join <code>` pour etre automatiquement whiteliste et autorise a rejoindre, meme si le salon est cache/verrouille. Gestion depuis l'app desktop (creation, liste, revocation, copie).
- **AFK auto-move** : detecte les utilisateurs mute+sourd dans les salons temporaires et les deplace vers un canal AFK apres un timeout configurable. Tache de fond toutes les 60s. Respecte l'immunite du proprietaire (configurable). Config per-guild : `afk_enabled`, `afk_channel_id`, `afk_timeout_minutes`, `afk_move_owner`.

### Audit Bot — Logs d'audit avances

Tracking complet des actions Discord et envoi a l'API :

- **Cache messages** : stockage LRU (10K/guild) pour afficher le contenu des messages supprimes
- **Anomaly detection** : detection pics d'activite suspects (mass bans, mass deletes, mass role changes) sur fenetre glissante configurable
- **Permission diffs** : calcul et affichage lisible des changements de permissions sur les roles (+ MANAGE_MESSAGES, - BAN_MEMBERS)
- **Historique pseudos** : envoi des changements de pseudo au backend (`POST /api/name-history`)
- **Rapport hebdomadaire** : embed recapitulatif automatique chaque lundi (joins, bans, deletes, edits, role changes, voice events, anomalies)
- **Events trackes** : messages (delete/edit/bulk), membres (join/leave/ban/unban/nickname/avatar/roles/timeout), channels, roles, voice, guild, threads, invites

### Community Bot — Roles et communaute

Gestion de panels de roles avec boutons + auto-roles + features avancees :

- **Roles exclusifs** : groupes mutuellement exclusifs (ex: prendre "Equipe Rouge" retire "Equipe Bleue")
- **Prerequis** : conditions pour obtenir un role (avoir un autre role, anciennete minimum)
- **Roles temporaires** : expiration automatique configurable par role
- **Parrainage** (`/parrain @membre`) : systeme de parrainage avec limites (1 parrain/filleul, max 3 actifs)
- **Booster detection** : attribution auto d'un role aux server boosters

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
- **automod-bot, moderation-bot, security-bot, ticket-bot, image-bot, voice-bot, progression-bot, audit-bot, community-bot**

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
PROGRESSION_DISCORD_TOKEN=...
AUDIT_DISCORD_TOKEN=...
COMMUNITY_DISCORD_TOKEN=...
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
| Automod Bot — All modules   | 214   | Spam, emoji spam, mentions, insult, link, phishing, unicode (zalgo/invisible/homoglyphs), adaptive slowmode |
| Security Bot — All modules  | 64    | Raid detector, raid analyzer (Levenshtein), alt detector, lockdown, captcha (math), quarantine, slowmode |
| Progression Bot — Tracker   | ~5    | Cache local                                         |
| Voice Bot — State           | ~22   | Cooldown, flood, pending, vote, AFK tracker         |
| API — Moderation HTTP       | 15    | Endpoints log_action, history, bans (integration)   |
| API — Moderation Service    | 17    | log_action, get_history, list_bans, delete_bans     |
| API — Strikes Service       | 8     | add_strike, escalation, reset, config               |
| API — Strikes HTTP          | 8     | Endpoints config, strikes, add, reset (integration) |
| API — Notes Service         | 10    | add_note, categories, get, delete                   |
| API — Notes HTTP            | 7     | Endpoints add, get, delete (integration)            |
| API — Reminders Service     | 6     | create, mark_sent, cancel, list                     |
| API — Reminders HTTP        | 6     | Endpoints create, pending, list (integration)       |
| API — Tickets HTTP          | 17    | CRUD tickets, status, channels (integration)        |
| API — Voice Channels HTTP   | 23    | CRUD channels, invite links, themes (integration)   |
| Audit Bot — All modules     | 29    | Message cache, anomaly detection, permission diffs, weekly report |
| Ticket Bot — All modules    | 59    | SLA tracker, satisfaction, templates, FAQ, transcript MD/HTML, ticket commands |
| Moderation Bot — All modules| 25    | Export CSV/JSON, reason templates, mass user parsing |
| Progression Bot — All modules | 42  | XP cooldown, streaks, multipliers, badges, tracker |
| Community Bot — All modules | 40    | Exclusive groups, prerequisites, temp roles, sponsorship |
| Image Bot — All modules     | 18    | Hash cache, channel thresholds, analysis queue |

**Total : ~650+ tests** sur 70+ fichiers avec `#[cfg(test)]` et tests d'integration

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

- [x] API Backend — Architecture hexagonale, 84+ endpoints, 25 handlers, helpers.rs, 17 use cases, 52 migrations
- [x] Automod Bot — Detection spam/insultes/liens/phishing/emoji/mentions/fichiers/unicode + leet speak + mode nuit + slowmode adaptatif + appel API + fallback
- [x] Moderation Bot — /warn /mute /ban /unmute /unban /history /note /call /context /appeal /export /massmute /massban + mode apprenti + templates raisons autocomplete
- [x] Security Bot — Anti-raid + analyse patterns (Levenshtein, avatars, creation cluster) + comptes suspects + captcha math + lockdown auto + alt detection + alertes
- [x] Progression Bot — Tracking messages/vocal + /stats + XP/levels + cooldown anti-farm + streaks + multiplicateurs salon/role + badges
- [x] Ticket Bot — /ticket create, close, assign + SLA tracking, satisfaction survey, templates reponses, FAQ, escalade auto, transcript MD/HTML
- [x] Image Bot — Detection images NSFW/illicites via API + hash cache + seuils salon + queue retry + screenshot/GIF detection
- [x] Voice Bot — Salons dynamiques, vote kick, co-admins, whitelist/ban, AFK auto-move, invite links, themes, stage mode
- [x] Audit Bot — Tracking audit logs + cache messages + anomaly detection + permission diffs + historique pseudos + rapport hebdomadaire
- [x] Community Bot — Panels de roles + auto-roles + exclusifs + prerequis + temp roles + parrainage + booster detection
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
- [x] Tests unitaires — 650+ tests (API, gateway, bots)
- [x] Multi-stage Docker builds — Images Alpine optimisees
- [x] Workers specialises — 3 workers dedies (moderation, analytics, monitoring) avec heartbeat
- [x] Anti-raid avance — Quarantaine, captcha DM (bouton/math), slowmode auto, lockdown auto, kick timeout, analyse patterns raid, detection alt accounts
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
- [x] Voice Invite Links — Liens d'invitation par code (8 chars, expiration configurable), bot + API + desktop
- [x] Voice Stats par salon — Statistiques vocales aggregees (temp/permanent), endpoint API + section desktop
- [x] Voice Themes — Templates de salon pre-configures (nom, emoji, limite, visibilite, bitrate, etc.), menu selection en DM, CRUD API + desktop
- [x] Voice Stage Mode — Mode presentation (deny SPEAK @everyone, grant speakers), bouton toggle + donner parole, integre aux themes
- [x] Systeme de strikes — Escalade progressive (3 strikes = mute, 5 = ban), config per-guild, fenetre glissante, 5 endpoints API, integration moderation handler
- [x] Notes utilisateur — /note <user> <text> avec categories (general/warning/positive/context), integration UserDossier, 3 endpoints API
- [x] Rappels sanctions temporaires — Auto-creation de rappel 1h avant expiration mute/ban temp, job worker send_reminders (30s), 3 endpoints API
- [x] Renommage stats-bot → progression-bot — Migration 038, tous les fichiers mis a jour
- [x] Renommage roles-bot → community-bot — Migration 039, tous les fichiers mis a jour

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
