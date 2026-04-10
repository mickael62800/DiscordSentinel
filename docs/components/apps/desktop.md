# desktop — Application Tauri (Rust + Vue 3)

**Rôle** : Interface graphique d'administration DiscordSentinel. Connexion OAuth2 Discord, affichage des dashboards, gestion des règles/modération/tickets, entraînement des modèles IA, WebSocket temps-réel.

## Architecture

**Tauri 2** (Rust backend + Vue 3 frontend). Pattern en layers côté Rust : **services (application) + ports (domain) + adapters (infrastructure)**. ConfigStore chiffré en AES-256-GCM dans un LMDB local. WebSocket client via `tokio-tungstenite`.

## Structure du code

```
apps/desktop/
├── src-tauri/                      (Rust backend Tauri)
│   ├── Cargo.toml
│   ├── src/
│   │   ├── main.rs                 (entrypoint → run())
│   │   ├── lib.rs                  (setup services, DI, Tauri commands)
│   │   │
│   │   ├── application/            (Services — 21 services)
│   │   │   ├── auth_service.rs     # OAuth2 Discord, propage X-Discord-Token
│   │   │   ├── dashboard_service.rs
│   │   │   ├── tickets_service.rs
│   │   │   ├── moderation_service.rs
│   │   │   ├── security_service.rs
│   │   │   ├── coude_service.rs
│   │   │   ├── ai_training_service.rs
│   │   │   ├── realtime_service.rs  # WebSocket gateway client
│   │   │   └── ... (21 au total)
│   │   │
│   │   ├── infrastructure/
│   │   │   ├── api_adapter.rs      # HTTP client wrapper, X-Discord-Token (Phase 2 B)
│   │   │   └── config_store.rs     # LMDB encrypt AES-256-GCM (bot tokens, API config)
│   │   │
│   │   ├── domain/
│   │   │   ├── entities.rs         # DTOs mappés depuis l'API
│   │   │   └── ports.rs            # trait AppAdapter
│   │   │
│   │   └── presentation/
│   │       ├── commands.rs         # Tauri commands exposées au frontend
│   │       └── macros.rs
│   │
│   └── tauri.conf.json             (config Tauri : bundler, permissions, etc.)
│
├── src/                            (Frontend Vue 3)
│   ├── main.ts                     (createApp)
│   ├── App.vue
│   ├── components/
│   │   ├── pages/                  (29 pages)
│   │   │   ├── DashboardPage.vue
│   │   │   ├── TicketsPage.vue
│   │   │   ├── ModerationPage.vue
│   │   │   ├── RulesPage.vue
│   │   │   ├── SecurityPage.vue
│   │   │   ├── InfractionsPage.vue
│   │   │   ├── VoiceChannelsPage.vue
│   │   │   ├── ConductPage.vue
│   │   │   ├── LevelsPage.vue
│   │   │   ├── CoudePage.vue
│   │   │   ├── AuditPage.vue
│   │   │   ├── AnalyticsPage.vue
│   │   │   ├── WatchedUsersPage.vue
│   │   │   ├── IaTrainingPage.vue  # connecte à ai-api Python
│   │   │   ├── ComponentConfigPage.vue
│   │   │   ├── SettingsPage.vue
│   │   │   ├── SetupPage.vue
│   │   │   ├── LoginPage.vue
│   │   │   └── ... (29 au total)
│   │   │
│   │   ├── molecules/              (12 composants réutilisables)
│   │   │   ├── StatCard.vue
│   │   │   ├── FilterBar.vue
│   │   │   ├── UserDossierPanel.vue
│   │   │   ├── BotTokenManager.vue
│   │   │   ├── AuditEventDetail.vue
│   │   │   └── ...
│   │   │
│   │   ├── atoms/                  (boutons, inputs, toggles)
│   │   └── layouts/                (MainLayout, etc.)
│   │
│   ├── stores/                     (Pinia stores — state management)
│   ├── utils/
│   │   └── api.ts                  (wrapper Tauri invoke())
│   └── types/
│
├── package.json
└── vite.config.ts
```

## Pages principales

| Page | Rôle |
|---|---|
| **Dashboard** | Accueil — stats, top infracteurs, tendances |
| **Tickets** | Créer / clôturer / assigner tickets support |
| **Moderation** | Ban, kick, warn |
| **Rules** | CRUD règles modération |
| **Security** | Événements détection malveillance |
| **Infractions** | Historique infractions par user |
| **Voice Channels** | Gestion salons vocaux personnels |
| **Conduct** | Points conduite, leaderboard |
| **Levels** | XP, récompenses |
| **Coude** | Gestion jeu Coup de Coude |
| **Audit** | Logs d'audit avec filtres |
| **Analytics** | Charts modération/activité |
| **Watched Users** | Dossiers users surveillés |
| **AI Training** | Fine-tuning modèles ONNX via `ai-api` |
| **Component Config** | Activer/désactiver bots par guild |
| **Settings / Setup** | Config API + Discord OAuth2 |
| **Login** | OAuth2 Discord (port local 19836) |

## Flux d'auth (Phase 2 B)

```
User clique "Login" (LoginPage.vue)
  ↓
AuthService.start_oauth_flow() ouvre https://discord.com/oauth2/authorize
  ↓
Discord redirige vers http://localhost:19836/?code=xxx
  ↓
AuthService.exchange_code() → POST oauth2/token → access_token
  ↓
AuthService.fetch_user() → GET /users/@me → DiscordUser
  ↓
ApiAdapter.set_discord_token(token) ← Phase 2 B
  ↓
Toutes les requêtes suivantes envoient header X-Discord-Token
  ↓
Backend (api) middleware guild_auth_middleware filtre par guild autorisée
```

Au logout : `AuthService::logout()` appelle `ApiAdapter::clear_discord_token()`.

## Dépendances externes

- **Tauri 2** (framework desktop)
- **Vue 3** + **Pinia** (frontend + state)
- **Reqwest** (HTTP client Rust)
- **Tokio** (runtime async)
- **Tauri plugins** : opener, store, notification, dialog
- **LMDB / heed** (local encrypted config store)
- **AES-256-GCM** (chiffrement bot tokens)
- **tokio-tungstenite** (WebSocket client vers `gateway`)

## Variables d'env (Rust side)

- `API_BASE_URL` — URL API backend (défaut `http://localhost:3000`)
- `API_KEY` — bearer token (stocké en LMDB après configuration initiale)
- `GATEWAY_URL` — URL WebSocket gateway (défaut `ws://localhost:3001/ws`)

Le reste de la config (URL API, clé API, Discord client ID/secret, bot tokens) est **persisté en LMDB chiffré** — la saisie initiale se fait via la page `SetupPage.vue`.

## Observabilité

- **Rust side** — logs console/fichier via `tracing-subscriber`
- **Frontend** — `console.log` errors, Pinia devtools
- **AuthService** — trace OAuth flow step par step
- **ConfigStore** — erreurs de persistance loggées
