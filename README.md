# DiscordSentinel

Plateforme de modération et d'animation distribuée pour serveurs Discord. Architecture microservices : **un bot Discord unifié** (interface Serenity), **API centrale** (intelligence + IA), **gateway WebSocket** (temps réel), **app web** (administration), **un worker unifié `sentinel-worker`** (scheduler regroupant 17 domaines périodiques), **inférence ONNX** embarquée.

---

## Architecture globale

```
Discord Messages / Events / Images
       │
       ▼
┌─────────────────────────────────────────────────────────────┐
│      Bot Discord unifié (Serenity 0.12)  — sentinel-bot      │
│   22 modules : ai_dataset · announcements · audit · automod ·│
│   blackjack · bump · cleanup · command_channel · community · │
│   confessions · games · moderation · progression ·           │
│   rotation · security · slot · tamagotchi · tickets · voice ·│
│   welcome · wheel                                            │
└────────────┬─────────────────────────────────┬──────────────┘
             │ HTTP (BaseApiClient keep-alive)  │ Redis streams
             ▼                                  ▼
┌─────────────────────────┐         ┌──────────────────────────┐
│  API backend (Axum 0.8) │◄────────┤  Gateway WebSocket       │
│  sentinel-api           │         │  sentinel-gateway        │
│  - 285 migrations       │         │  (relay Redis → clients) │
│  - ONNX inference       │         └─────────┬────────────────┘
│  - ~150 handlers HTTP   │                   │
│  - Hexagonal            │                   │
│  - guild_auth multi-    │                   │
│    tenant (OAuth2)      │                   │
│  - OAuth Discord        │                   │
└──────┬──────────────┬───┘                   │
       │              │                       │
       │ PostgreSQL   │ Redis                 │
       ▼              ▼                       ▼
┌───────────┐  ┌───────────┐        ┌────────────────────────┐
│ Postgres  │  │  Redis    │        │ web (Vue 3)   │
│(PgBouncer)│  │ (cache +  │        │ OAuth2 Discord + WS    │
│ 285 migs  │  │  streams) │        └────────────────────────┘
└─────┬─────┘  └─────┬─────┘
      │              │
      └──────┬───────┘
             ▼
┌─────────────────────────────────────────┐
│ sentinel-worker (Tokio) — worker unifié  │
│ Meta-scheduler, 17 domaines périodiques: │
│ ai · analytics · announcements ·         │
│ appeal_sla · audit_cache · automod ·     │
│ cache · cleanup · discord_audit_sync ·   │
│ export · game_portal · guild_backup ·    │
│ moderation · monitoring · security ·     │
│ temp_roles · tickets                     │
└─────────────────────────────────────────┘
```

**Philosophie** : Bot = interface légère (multi-module dans un seul process) · API = cerveau (décisions + IA + persistance) · Gateway = temps réel découplé · Workers = jobs DB-bound périodiques · Web = admin remote.

---

## Stack technique

| Composant | Technologie | Détails |
|---|---|---|
| API backend | Rust / Axum 0.8 / Tokio / sqlx 0.8 | Hexagonal, ~150 handlers, 285 migrations, ONNX inference, OAuth Discord |
| Gateway WebSocket | Rust / Axum 0.8 / Redis | Service dédié temps réel, auto-reconnect exponential backoff |
| Bot Discord unifié | Rust / Serenity 0.12 | Process unique, 22 modules chargés dynamiquement selon config per-guild (helpers communs dans `src/shared/`) |
| Worker unifié | Rust / Tokio / sqlx / lib `worker-common` | 1 binaire `sentinel-worker` (meta-scheduler, 17 domaines périodiques), heartbeat + métriques Prometheus |
| gRPC | `tonic` 0.13 + `prost` 0.13 | Crate `sentinel-proto` (CRUD voice/IA/modération en gRPC) |
| PostgreSQL | Postgres 16 + **PgBouncer** | 285 migrations, partitionnement RANGE mensuel, vues matérialisées |
| Cache / Bus | Redis 7 | `maxmemory=2gb allkeys-lru`, **Redis Streams** (`sentinel:events`, consumer groups durables), cache `user_guilds` multi-tenant |
| Inférence IA | ONNX Runtime 2.0 (`ort` 2.0-rc.12) / ndarray / tokenizers | Vision (NSFW/illicite) + Text (sentiments multilingues) |
| Web dashboard | Vue 3 + TS + Vite + Pinia + Chart.js | `web` — servi par Nginx (Dockerfile + nginx.conf) |
| Observabilité | Prometheus + Grafana + tokio-metrics | Middleware Axum metrics, dashboards provisionnés |
| Containerisation | Docker Alpine multi-stage + Compose | Infra + API + gateway + bot + workers + web + monitoring |

**Dépendances workspace** : `tokio`, `serde`, `reqwest 0.12` (rustls, pool tuné), `sqlx 0.8`, `chrono`, `uuid`, `tracing`, `async-trait`, `tower-http` (CORS, compression zstd/gzip, rate limit, request-id), `ort` (ONNX), `tokenizers`, `ndarray`, `redis 0.27` (streams), `tonic`/`prost` (gRPC), `metrics-exporter-prometheus`, `tokio-metrics`, `tikv-jemallocator` (Linux/macOS). Profil release : `lto = "thin"`, `codegen-units = 16`, `strip = true`.

---

## Structure du projet

```
DiscordSentinel/
├── web/                # Vue 3 web dashboard (Pinia, vue-router, Chart.js, Nginx)
│
├── sentinel-bot/                # Bot Discord unifié (single process, 22 modules)
│   └── src/
│       ├── modules/             # ai_dataset · announcements · audit · automod · blackjack ·
│       │                        # bump · cleanup · command_channel · community · confessions ·
│       │                        # games · moderation · progression · rotation ·
│       │                        # security · slot · tamagotchi · tickets · voice ·
│       │                        # welcome · wheel
│       └── shared/              # Helpers communs (api_client, event_bus, cache_settings, embeds, grpc_client, ...)
│
├── sentinel-api/                # API backend (Axum 0.8) — hexagonal
│   ├── src/
│   │   ├── adapters/inbound/http/   # handlers, 21 fichiers de routes, middlewares (auth, guild_auth, rate_limit, api_logger, metrics)
│   │   ├── adapters/inbound/grpc/   # services gRPC (voice, ai, moderation)
│   │   ├── adapters/outbound/       # repositories postgres, redis_cache
│   │   ├── application/             # use case services
│   │   ├── domain/                  # entities, value_objects, services (ONNX, Discord API)
│   │   └── ports/                   # traits inbound/outbound
│   └── migrations/                  # 285 fichiers (001 → 281, plusieurs séries)
│
├── sentinel-gateway/            # WebSocket relay (Redis → clients)
├── sentinel-proto/              # Définitions gRPC (`tonic` + `prost`)
├── sentinel-worker/             # Meta-scheduler unifié — 17 domaines périodiques
│
├── sentinel-ml/                 # Configs d'entraînement (YAML) + dossiers d'exports ONNX (montés par Docker)
├── infrastructure/              # docker/ (compose + Dockerfiles), prometheus.yml, grafana, scripts/ (build-all, dev, health-check, ...)
│                                # (partagé par les stacks Sentinel et Nexus — ex-sentinel-infrastructure/)
│
├── nexus-core/                  # Nexus (jeux) — cœur hexagonal pur (scaffold)
├── nexus-api/                   # Nexus — API HTTP axum (scaffold)
├── nexus-bot/                   # Nexus — bot Discord serenity (scaffold)
├── nexus-worker/                # Nexus — worker tokio (scaffold)
├── nexus-gateway/               # Nexus — gateway (scaffold)
├── nexus-proto/                 # Nexus — définitions gRPC (stub)
│
├── Cargo.toml                   # Workspace Rust (11 crates)
└── README.md                    # ← ce fichier
```

> Note : `web/` et `infrastructure/` (ex-`sentinel-web/` / `sentinel-infrastructure/`)
> sont désormais partagés par les deux stacks (Sentinel et Nexus).

## Nexus (plateforme jeux, en construction)

Nouvelle plateforme de jeux, architecture calquée sur Sentinel (hexagonale) :

- `nexus-core` — lib pure : domain / application / ports, zéro dépendance infra
- `nexus-api` — API HTTP (axum), port `NEXUS_API_PORT` (défaut 3100)
- `nexus-bot` — bot Discord (serenity) ; ne se connecte pas sans `NEXUS_DISCORD_TOKEN`
- `nexus-worker` — jobs de fond (tokio)
- `nexus-gateway` — point d'entrée réseau
- `nexus-proto` — protos gRPC (stub, modèle `sentinel-proto`)

---

## Modules, fonctionnalités & commandes

Le bot expose **22 modules** activables/configurables par serveur (table `bot_guild_config`, schéma de formulaire dans `bot_definitions.config_schema`, éditables depuis le dashboard web). Chaque commande slash est filtrée par module activé **et** par permission Discord. Référence détaillée : [`docs/COMMANDES_ADMIN.md`](docs/COMMANDES_ADMIN.md) (staff) et [`docs/COMMANDES_UTILISATEURS.md`](docs/COMMANDES_UTILISATEURS.md) (membres).

### 🤖 automod — Modération automatique + vote des modérateurs

Analyse chaque message (texte + images) : détecteurs locaux (spam, insulte, lien, phishing, caps, flood, emoji, mentions, unicode, fichiers suspects) **+** IA ONNX (texte multilingue, vision NSFW/illicite). Chaque flag a un **poids** ; la somme donne un **score** comparé aux **seuils** (warn / delete / mute / ban) configurés sur la page **Règles de modération** (`/api/rules`).

**Selon le score et la config**, le message :
- est traité **automatiquement** (warn / delete / mute appliqués ; le ban n'est jamais automatique → simple signalement), **ou**
- déclenche une **carte de review/vote** dans le salon de review (`log_channel_id`).

**Système de vote** (`vote_enabled`) : la carte affiche le contexte et des boutons **Warn / Delete / Mute / Ban / Ignorer**. Les modérateurs votent ; à l'échéance (`vote_deadline_hours`) le `sentinel-worker` (domaine `automod`) dépouille (quorum + tie-break) ; un **administrateur finalise** via un bouton dédié (seule voie d'un ban réel).

- **Regroupement par utilisateur** (`vote_aggregate_enabled`) : tant qu'une carte est ouverte pour un membre, les nouveaux signalements **s'agrègent** dans la même carte (liste d'incidents, score cumulé, deadline prolongée) au lieu de spammer des cartes.
- **Salon de discussion** (`discussion_channel_enabled`) : un bouton « Ouvrir une discussion » crée un **salon textuel privé** (membre concerné + rôle modo) sous une catégorie configurable (`discussion_category_id`), avec un message de contexte épinglé, pour échanger avant décision.

| Commande | Permission | Rôle |
|---|---|---|
| `/automod status` / `/automod test` | Gérer le serveur | État des caches/trackers ; tester l'analyse d'un message |

### ⚖️ moderation — Modération manuelle (22 commandes)

Sanctions (`/warn`, `/unwarn`, `/mute`, `/unmute`, `/ban`, `/unban`, `/massmute`, `/massban`), dossiers (`/history`, `/note`, `/context`, `/evidence`, `/expirations`, `/compare`, `/call`), outils (`/appeal`, `/review`, `/template`, `/transcript`, `/export`, `/modstats`).

- **`/card`** — crée manuellement une carte de vote (identique à l'automod, contexte **avant + après** le message) quand une détection est passée au travers. Cible via **lien Discord** (cross-salon) ou ID. Postée dans le salon de review automod, flux de vote/finalisation complet.
- **`/context`** — affiche les messages autour d'un message pour comprendre une situation.

### 🔐 security — Anti-raid / alt accounts

Détection de raids (pics de joins), comptes récents/alt, captcha, quarantaine, lockdown, slowmode adaptatif. `/security status`, `/security history`. *(Seuils & toggles configurés au démarrage via variables d'env — voir section Déploiement.)*

### 🎉 welcome — Accueil & onboarding

Messages de bienvenue / départ / retour (rich embeds), validation du règlement (bouton → rôle), anniversaires d'arrivée, et **deux compteurs de salon renommés** :
- **Compteur de membres** (`counter_*`) → ex. `Membres : 1234`
- **Compteur de présence vocale** (`voice_counter_*`) → ex. `En Vocal : 5`, mis à jour à chaque connexion/déconnexion vocale.

*(Pas de commande : tout se configure via le dashboard web `/api/welcome`.)*

### 🔊 voice — Salons vocaux temporaires

Création à la volée (salon « créateur »), panneau de contrôle (renommer, lock, limite, visibilité, kick, ban, co-admins, transfert, file d'attente, vote-kick), thèmes réutilisables, cleanup auto des salons vides, liens d'invitation.

- **Whitelist persistante par propriétaire** (« liste d'amis ») : réappliquée à chaque création de salon.
- **Preset de paramètres par propriétaire** : le bouton **« Sauvegarder params »** mémorise l'état courant (nom, limite, visibilité, verrou, file) ; à la création suivante, le salon reprend ces réglages **et** la whitelist. Mode **caché-sauf-whitelist** : `@everyone` ne voit pas le salon, seuls les membres whitelistés y accèdent.

### 🐾 tamagotchi — Compagnon virtuel

Jeu d'animation (coins partagés). Panneau public → **salon privé par joueur** : choix d'espèce à la naissance, puis **carte** du compagnon (rendu PNG) avec boutons de soin (Nourrir / Jouer / Dormir / Câliner), entraînement de stats, boutique, visite, combat PvP asynchrone (ELO).

- **Cycle de vie** : le `sentinel-worker` (domaine `tamagotchi`) applique périodiquement la décroissance des jauges (faim / bonheur / énergie) et les transitions **maladie → mort → guérison**, selon des seuils configurables par serveur. Pagination par curseur (aucune limite sur le nombre de compagnons).
- **Rafraîchissement automatique de la carte** dans Discord : tâche par serveur, **fréquence réglable sur le web** (`card_refresh_interval_minutes`). La carte est ré-éditée sans intervention du joueur.
- **Notifications** : DM au propriétaire + édition immédiate de la carte quand le compagnon tombe malade ou meurt (event Redis Stream consommé par le bot).

| Commande | Permission | Rôle |
|---|---|---|
| `/tama-setup` | Admin | Déploie le panneau public « Ouvrir mon compagnon » |

### 🎫 tickets — Support

`/ticket close` (membre), `/ticket-admin panel|invite` (staff). SLA, fermeture sur inactivité, transcripts.

### 🔎 audit — Journal d'audit Discord

`/audit search`, `/audit stats`. Ingestion des audit-logs (domaine `discord_audit_sync` du worker), surveillance d'utilisateurs, rapports hebdomadaires.

### 📈 progression — Niveaux & XP

XP par activité (messages, vocal, ancienneté), rôles de niveau. `/level user|top`, `/stats`, `/progression-resync` (admin).

### 🧹 cleanup — Nettoyage

`/purge last|user|contains` (Gérer les messages), `/cleanup logs|infractions|audit` (admin).

### 👥 community — Rôles & parrainage

`/roles-panel` (panels de rôles auto-assignables, staff), `/parrain` (parrainage de nouveaux membres).

### 🔁 rotation — Administrateur tournant

Chaque période, un modérateur (rôle configuré) devient administrateur à tour de rôle, après acceptation en MP + validation de l'owner. Le précédent admin redevient modérateur. État persisté via `/api/rotation`.

### 🤫 confessions — Confessions anonymes

`/confess` (membre, anonyme), `/confess-admin deploy-panel|delete|reveal` (admin).

### 📣 announcements — Annonces planifiées

Annonces programmées (publiées par le worker, consommées via Redis Stream par le bot).

### ⬆️ bump — Récompenses Disboard

Détecte un `/bump` Disboard réussi, récompense l'auteur en coins (montant gradué selon le nombre de bumps de la semaine, calculé côté API) et rappelle quand un nouveau bump est possible après le cooldown.

### 💬 command_channel — Salons « commandes uniquement »

Supprime en silence tout message texte classique dans les salons configurés (les commandes slash, étant des interactions, ne sont pas affectées). L'owner et les bots ne sont jamais supprimés.

### 🧠 ai_dataset — Collecte de dataset IA

Module indépendant (désactivé par défaut) : quand actif, envoie chaque message non-bot à `POST /api/ai-dataset/collect` pour alimenter la table `ai_dataset_messages`. La page web « Dataset IA » permet l'étiquetage manuel et l'export CSV.

### 🎮 Jeux & casino

- **games** — `/game list|join|leave`, `/game-admin` (catalogue de jeux + panels d'inscription).
- **blackjack** — `/blackjack` (solo) + tables multijoueur via `/blackjack-setup`.
- **slot** — `/slot-setup` (machine à sous, jackpot progressif).
- **wheel** — `/wheel-setup` (roue du destin).

---

## Base de données

**PostgreSQL 16** derrière **PgBouncer** (transaction pooling). **285 migrations** versionnées dans `sentinel-api/migrations/` (numérotation 001 → 281, plusieurs séries de plages).

### Optimisations structurelles

- **Vues matérialisées** (`mv_wallet_leaderboard`, `mv_level_leaderboard`) refreshées toutes les 5 min par le domaine `cache` du worker. Gain 100–1000× sur les endpoints leaderboard.
- **Partitionnement RANGE mensuel** sur 4 tables hot : `infractions`, `audit_logs`, `user_activity_log`, `logs`. Génération automatique M+1/M+2 par le domaine `cache`.
- **Enums Postgres** : `moderation_gravity`, `voice_channel_kind` (wrappers Rust `#[derive(sqlx::Type)]`). `discord_roles.permissions` en `BIGINT`.
- **Index GIN** sur `infractions.flags`, `security_events.user_ids`, `bot_definitions.config_schema`. Partials soft-delete sur `voice_channels` + `tickets`. Discord IDs typés `VARCHAR(20)`.
- **Table `user_cache`** : source de vérité des usernames Discord, alimentée par agrégation périodique depuis 4 tables hot.
- **Table `ai_jobs`** : file d'attente asynchrone pour l'inférence IA (consommée par le domaine `ai`).

### Tables principales (extrait)

| Table | Description |
|---|---|
| `rules` / `infractions` | Règles de modération + infractions (flags JSONB + GIN) |
| `tickets` / `ticket_messages` | Tickets support + SLA |
| `moderation_actions` | Historique modération manuelle (enum `moderation_gravity`) |
| `security_events` | Détection raid / alt accounts |
| `audit_logs` **(partitionné)** | Logs d'audit Discord (ingestés par `discord_audit_sync`) |
| `user_activity_log` **(partitionné)** | Activité utilisateur pour surveillance |
| `logs` **(partitionné)** | Logs applicatifs bot/workers |
| `user_stats` / `user_levels` / `user_wallets` | Stats, XP, wallets (3 vues matérialisées) |
| `voice_channels` + sub-tables | Salons vocaux temporaires (enum `voice_channel_kind`), whitelists, bans, invites, **presets par propriétaire** |
| `pets` / `pet_events` | Tamagotchi : compagnons (jauges, stats, statut, position de la carte Discord) + journal d'actions |
| `bot_guild_config` / `bot_definitions` | Config per-guild + schéma de config par module bot |
| `sanction_reminders` | Rappels 24 h avant expiration mute/ban |
| `temp_roles` | Rôles temporaires (scan par le domaine `temp_roles`) |
| `ai_jobs` / `ai_dataset_messages` | Queue IA async + dataset collecté |
| `welcome_config` | Config bienvenue + rich embeds |
| `automod_reviews` / `automod_review_votes` | Cartes de review + votes des modérateurs (incidents agrégés, score cumulé) |
| `automod_discussion_channels` | Salons de discussion liés à une review (audit + idempotence) |

### Flag types supportés (10)

| Type | Source | Poids défaut |
|---|---|---|
| `spam` / `insult` / `link` / `phishing` | Module automod | 3.0 / 5.0 / 1.0 / 7.0 |
| `nsfw` / `illicit` | IA Vision ONNX | 8.0 / 9.0 |
| `anger` / `rage` / `threat` / `harassment` | IA Text ONNX | 3.0 / 6.0 / 8.0 / 7.0 |

---

## Endpoints API (résumé)

**Authentification** : `Authorization: Bearer <API_KEY>` obligatoire (sauf `/health` et `/metrics`). Le middleware `guild_auth_middleware` filtre en plus par `X-Discord-Token` si présent (multi-tenant OAuth2). ~150 endpoints répartis sur **21 fichiers de routes**.

| Préfixe | Handler | Description |
|---|---|---|
| `/health`, `/metrics` | health / metrics / system | Publics (Prometheus + healthcheck) |
| `/analyze`, `/analyze/image` | analyze*.rs | Inférence IA **synchrone** (rate limit strict) |
| `/api/ai/jobs` | ai_jobs.rs | Queue **async** (POST = 202 + job_id, GET = status) |
| `/api/analytics/*`, `/api/dashboard/*`, `/api/stats/*` | analytics / dashboard / dashboard_charts / stats | Heatmap, trends, top infractors, KPIs |
| `/api/rules/*` | rules.rs | Règles de modération |
| `/api/infractions/*`, `/api/strikes/*` | infractions / strikes | Infractions + escalade |
| `/api/tickets/*` | tickets.rs | Tickets support |
| `/api/moderation/*`, `/api/purge/*` | moderation / purge | `log_action`, history, bans, purges |
| `/api/security/*` | security.rs | Events de détection |
| `/api/notes/*`, `/api/reminders/*` | notes / reminders | Notes mod + rappels |
| `/api/voice-channels/*` | voice_channels.rs | Salons vocaux dynamiques (presets, whitelists, bans, invites, thèmes) |
| `/api/tamagotchi/*` | tamagotchi/pets.rs | Compagnons : CRUD, soins, combat, tick de cycle de vie, cartes à rafraîchir |
| `/api/levels/*` | levels | XP/niveaux |
| `/api/blackjack/*`, `/api/games/*`, `/api/wallet/*` | blackjack/* / games / wallet | Jeux + porte-monnaie |
| `/api/audit-logs/*`, `/api/watched-users/*`, `/api/user-activity/*` | audit_logs / watched_users / user_activity | Audit + dossiers surveillés |
| `/api/ai-dataset/*` | ai_dataset | Collecte + étiquetage dataset IA |
| `/api/discord-roles/*`, `/api/role-panels/*` | discord_roles / role_panels | Sync rôles + panels réactionnels |
| `/api/members/*`, `/api/guilds/*`, `/api/guild-channels/*` | guild_members / guild_channels | Membres + guilds + salons |
| `/api/welcome/*` | welcome.rs | Config bienvenue + rich embeds |
| `/api/models/*`, `/api/cache/*`, `/api/bots/*` | models_status / cache_stats / bot_config / bot_persistence | Monitoring ONNX + cache + config per-guild |
| `/api/oauth/*`, `/api/rbac/*` | oauth / rbac | OAuth2 Discord + rôles applicatifs |
| `/api/exports/*` | exports.rs | Exports async (délégués au domaine `export`) |

---

## Inférence IA (ONNX)

### Modèle Vision

| Propriété | Valeur |
|---|---|
| Architecture | EfficientNetV2-S |
| Classes | `safe`, `nsfw`, `illicit` |
| Input | 224×224 normalisé ImageNet |
| Format | ONNX (opset 17) |

### Modèle Text

| Propriété | Valeur |
|---|---|
| Architecture | DistilBERT multilingual |
| Classes | `neutral`, `anger`, `rage`, `threat`, `harassment` |
| Input | Tokens (max 256) + attention mask |
| Tokenizer | HuggingFace tokenizers (Rust) |

Les modèles sont chargés au démarrage de l'API. **Mode dégradé** automatique si absents (scoring règles seulement). Les configs d'entraînement vivent dans `sentinel-ml/{text,vision}/configs/`, les exports ONNX sont attendus dans `sentinel-ml/{text,vision}/exports/` (montés en `/models/*` par `docker-compose.yml`). Le pipeline d'entraînement lui-même est externe au repo.

### Config IA per-guild

Centralisée dans `bot_guild_config` (bot_name = `automod-bot`) : détection IA (`text_enabled`, `text_threshold`, `vision_enabled`, `vision_threshold`, `context_dampening`, `context_format`, `context_max_messages`, `context_max_chars`), tension de salon (`channel_tension_*`), et **système de vote** (`vote_enabled`, `vote_deadline_hours`, `vote_quorum`, `vote_mod_role_id`, `vote_admin_role_id`, `vote_tie_action`, `vote_context_before`, `vote_thread_enabled`, `vote_aggregate_enabled`, `discussion_channel_enabled`, `discussion_category_id`). Configurable depuis le dashboard web.

### Mode async

Alternative à `POST /analyze` (synchrone, timeout 5 s côté bot) : **`POST /api/ai/jobs`** retourne `202 Accepted` + `job_id` immédiatement. Le domaine `ai` du worker dépile la file et publie le résultat sur Redis `ai_result:{job_id}` (+ `SET` TTL 600 s).

---

## Bus d'événements (Redis Streams)

Tous les producers (API, workers) publient sur une **stream unique `sentinel:events`** (`XADD MAXLEN ~ 10000`). Format d'entrée : un champ `payload` = `{"event": ..., "data": ...}`.

- **Consumers durables** (bot) : `XREADGROUP` + `XACK` (at-least-once), un consumer group par feature (`tamagotchi-bot-lifecycle`, …), auto-claim `XAUTOCLAIM` des pending > 60 s après un crash.
- **Consumers live-tail** (gateway) : `XREAD $` sans group → relay WebSocket.

**Events** : `infraction_new`, `ticket_*`, `security_event`, `moderation_action`, `voice_channel_updated/closed`, `tamagotchi_pet_status`, `tournament_resolved`, `sanction_expiry_reminder`, `temp_role_expire`, `bot_log`, etc.

---

## Multi-tenant

Middleware **`guild_auth_middleware`** : filtre les requêtes selon l'appartenance Discord du user appelant.

1. Le web fait OAuth2 Discord (scopes `identify` + `guilds`) → `access_token`.
2. L'adaptateur client envoie ce token dans `X-Discord-Token` sur toutes les requêtes.
3. Le middleware extrait le `guild_id` de l'URI, interroge `/users/@me/guilds` (cache Redis 5 min par hash de token), et refuse `403` si le guild n'est pas dans la liste autorisée.
4. Si `X-Discord-Token` est absent (appel bot/worker interne), le middleware est **pass-through** — `auth_middleware` (Bearer API key) reste obligatoire.

---

## Gateway WebSocket

| Propriété | Valeur |
|---|---|
| Port | 3001 |
| Auth | `?token=<api_key>` |
| Max connexions | 1000 (configurable) |
| Reconnexion Redis | Exponential backoff |
| Healthcheck | `GET /health` |

---

## Middleware API (ordre de traversée)

```
Request
  → CORS
  → SetRequestId + TraceLayer
  → BodyLimit (10 MB par défaut)
  → CompressionLayer (zstd + gzip)
  → metrics_middleware (Prometheus)
  → api_logger (→ table logs)
  → [si route protégée]
      → rate_limit (token bucket IP)
      → auth (Bearer API key)
      → guild_auth (X-Discord-Token)
  → Handler
```

**Rate limit inférence ONNX** : semaphore (`INFERENCE_MAX_CONCURRENT=4`) + token bucket (`INFERENCE_MAX_PER_SEC=20`). HTTP 429 si dépassement.

---

## Observabilité

- **Prometheus** — endpoint `/metrics` sur l'API et chaque worker (port 9100). Compteurs `http_requests_total{route,method,status}`, histogrammes `http_request_duration_seconds`, gauges `tokio_busy_ratio`, `tokio_live_tasks_count`, `tokio_global_queue_depth`.
- **Grafana** — dashboards auto-provisionnés dans `infrastructure/grafana/`. Démarrage : `docker compose -f infrastructure/docker/docker-compose.yml --profile monitoring up -d prometheus grafana`. UI sur `http://localhost:3002` (admin/admin).
- **pg_stat_statements** — extension activée. `SELECT * FROM pg_stat_statements ORDER BY total_exec_time DESC`.
- **Tracing structuré** — `tracing-subscriber` JSON en prod, correlation IDs `X-Request-ID` propagés via `tower_http::request_id`.

---

## Déploiement

### Docker Compose

```bash
# Stack complète (infra + API + bot unifié + worker unifié + gateway + web)
docker compose -f infrastructure/docker/docker-compose.yml up -d

# Avec Prometheus + Grafana
docker compose -f infrastructure/docker/docker-compose.yml --profile monitoring up -d
```

**Services infra** : `postgres` (tuning RAM : `shared_buffers=4GB`, `work_mem=64MB`, WAL tuning), `pgbouncer` (transaction pooling), `redis` (`maxmemory=2gb allkeys-lru`).

### Variables d'environnement (.env)

```env
# Infrastructure
POSTGRES_PASSWORD=sentinel_secret
REDIS_PASSWORD=sentinel_redis

# API
API_KEY=your_api_key_here
REQUIRE_API_KEY=true

# IA / Inference ONNX
VISION_MODEL_PATH=/models/vision/vision_sentinel.onnx
TEXT_MODEL_PATH=/models/text/text_sentinel.onnx
TEXT_TOKENIZER_PATH=/models/text/tokenizer.json
TEXT_MAX_LENGTH=256

# Bot Discord unifié (réutilisé par l'API et certains workers pour audit-sync, etc.)
SENTINEL_DISCORD_TOKEN=...

# Module security (seuils & toggles globaux, lus au démarrage du bot)
RAID_JOIN_THRESHOLD=10
MIN_ACCOUNT_AGE_SECS=86400
QUARANTINE_ENABLED=false
CAPTCHA_ENABLED=false
LOCKDOWN_ENABLED=false
ALT_DETECTION_ENABLED=false

# Module voice (fallbacks ; la config par serveur prime via le dashboard)
VOICE_GUILD_ID=...
VOICE_PUBLIC_CREATOR_CHANNEL_ID=...
VOICE_PRIVATE_CREATOR_CHANNEL_ID=...
VOICE_LOG_CHANNEL_ID=...

# Tamagotchi (intervalle du tick de cycle de vie ; le refresh des cartes
# est configuré par serveur via le web : card_refresh_interval_minutes)
TAMAGOTCHI_TICK_INTERVAL_SECS=300

# Économie (fallback pour le solde de départ des wallets)
WALLET_STARTING_COINS=100
```

> **Note** — Plusieurs réglages historiquement portés par des variables d'env sont désormais éditables **par serveur** depuis le dashboard web (`bot_guild_config`) : solde de départ, fréquence de refresh des cartes tamagotchi, salons/catégories voice/tickets, etc. Les variables d'env restent des **fallbacks** ou des réglages **globaux** lus au démarrage (un changement nécessite alors un redémarrage).

### Développement local

```bash
bash infrastructure/scripts/dev.sh              # Lance API + bot + web
bash infrastructure/scripts/build-all.sh        # Build release de tous les crates
bash infrastructure/scripts/health-check.sh     # Vérifie que tous les services répondent
bash infrastructure/scripts/seed-rules.sh       # Seed de règles de dev
bash infrastructure/scripts/start-all.sh        # Démarre la stack complète

# Ou composant par composant :
cd sentinel-api && cargo run
cd sentinel-worker && cargo run
cd sentinel-bot && cargo run
cd web && npm run dev
```

---

## Tests

Tests `cargo test --lib` côté API + unitaires workers/modules bot. Couverture principale :

- **API** : ScoringService, InferenceService, ValueObjects, MV repos, middleware (`guild_auth`), handlers (welcome, infractions postgres, repos outbound).
- **Bot unifié** : détecteurs automod, security, tickets, audit, progression, community, voice.
- **Workers** : helpers `worker-common`.
- **Gateway** : broadcaster.

Stack de tests dédiée via `infrastructure/docker/docker-compose.test.yml`.

---

## Bonnes pratiques du projet

- **Bot = interface légère** — logique métier centralisée dans l'API, jamais dans les modules du bot.
- **Architecture hexagonale** côté API — séparation stricte domain / ports / adapters.
- **Workers = jobs périodiques DB-bound** — via `spawn_periodic` + Redis Streams, pas de Discord gateway direct (sauf le domaine `discord_audit_sync` qui utilise le token bot pour lire les audit-logs).
- **Gateway découplé** — absorbe les bursts WebSocket indépendamment de l'API.
- **Inférence IA gracieuse** — si modèles absents, fallback scoring règles.
- **Multi-tenant** — filtre `guild_auth` avec pass-through pour appels internes.
- **Config par serveur d'abord** — `config_schema` + `bot_guild_config` éditables sur le web ; les variables d'env servent de fallback / réglage global.
- **Observabilité first** — métriques Prometheus + traces JSON + `pg_stat_statements`.
- **Workspace Rust partagé** — deps communes déclarées une seule fois dans le `Cargo.toml` racine, `lto = "thin"` pour garder les builds rapides.
```
