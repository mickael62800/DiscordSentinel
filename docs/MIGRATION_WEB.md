# Migration Desktop (Tauri) vers Web

## Vue d'ensemble

L'application bureau DiscordSentinel est construite avec **Tauri 2 + Vue 3 + TypeScript**. Le frontend est deja du web standard (Vue SPA) — seule la couche de communication avec le backend utilise `invoke()` Tauri au lieu d'appels HTTP.

**Effort estime : 3-5 jours developpeur.**

---

## Architecture actuelle

```
apps/desktop/
  src/                    # Frontend Vue 3 (reutilisable tel quel)
    components/           # 62 composants Vue (pages, molecules, atoms)
    composables/          # 33 composables (logique metier)
    router/               # Vue Router (21 routes)
    types/                # TypeScript interfaces
    assets/               # CSS, images
  src-tauri/              # Backend Rust Tauri (a supprimer pour le web)
    src/application/      # Services qui appellent l'API REST
    src/presentation/     # 84 commandes Tauri (bridge invoke → HTTP)
```

### Point cle : abstraction existante

Tous les composables passent par **`useFetch()`** (`composables/useFetch.ts`) qui encapsule `invoke()`. En modifiant uniquement ce fichier, 80% des appels basculent automatiquement.

---

## Ce qui change

### 1. `useFetch.ts` — Le coeur de la migration

**Avant (Tauri) :**
```typescript
import { invoke } from "@tauri-apps/api/core";

export function useFetch<T>(command: string, initialValue: T, params?: Record<string, unknown>) {
  // ...
  const result = await invoke<T>(command, params);
}
```

**Apres (Web) :**
```typescript
const API_BASE = import.meta.env.VITE_API_URL || "http://localhost:3000";

export function useFetch<T>(command: string, initialValue: T, params?: Record<string, unknown>) {
  // ...
  const { url, method, body } = mapCommandToHttp(command, params);
  const resp = await fetch(`${API_BASE}${url}`, { method, body, headers });
  const result = await resp.json() as T;
}
```

Il faut creer un **mapping des 84 commandes Tauri vers les endpoints API REST**. Ce mapping est dans la section 6.

---

### 2. `useAuth.ts` — Authentification

**Avant :** OAuth Discord via Tauri (serveur local Rust) + stockage `@tauri-apps/plugin-store`.

**Apres :** OAuth Discord via redirect web standard + stockage `localStorage`.

```typescript
// Avant
import { load } from "@tauri-apps/plugin-store";
const loggedUser = await invoke<DiscordUser>("discord_login");
const store = await load("auth.json");
await store.set("discord_user", loggedUser);

// Apres
window.location.href = `${API_BASE}/auth/discord?redirect=${window.location.origin}/callback`;
// Callback stocke le token JWT
localStorage.setItem("auth_token", token);
localStorage.setItem("discord_user", JSON.stringify(user));
```

**Action requise :**
- [ ] Ajouter un endpoint `/auth/discord` a l'API (OAuth redirect)
- [ ] Ajouter un endpoint `/auth/callback` (echange code → token)
- [ ] Ajouter un endpoint `/auth/me` (retourne l'utilisateur connecte)
- [ ] Remplacer `useAuth.ts` avec `localStorage` au lieu du Tauri Store
- [ ] Ajouter un JWT middleware a l'API pour les requetes web

---

### 3. `useRealtime.ts` — WebSocket

**Avant :** Tauri listen() sur des events internes forwarded par le backend Rust.

**Apres :** WebSocket natif vers l'endpoint `/ws` de l'API (qui existe deja).

```typescript
// Avant
import { listen } from "@tauri-apps/api/event";
await invoke("ws_connect");
await listen("ws:connected", callback);

// Apres
const ws = new WebSocket(`${WS_URL}/ws`);
ws.onmessage = (event) => {
  const data = JSON.parse(event.data);
  handleEvent(data.event, data.data);
};
```

**Action requise :**
- [ ] Remplacer `useRealtime.ts` (WebSocket natif, ~30 lignes)
- [ ] Adapter `useNotifications.ts` (remplacer `listen()` par `ws.onmessage`)

---

### 4. `useNotifications.ts` — Notifications

**Avant :** `@tauri-apps/plugin-notification` pour les notifs natives desktop.

**Apres :** API Web Notifications.

```typescript
// Avant
import { sendNotification } from "@tauri-apps/plugin-notification";
sendNotification({ title: "Sentinel", body: "Raid detecte" });

// Apres
if (Notification.permission === "granted") {
  new Notification("Sentinel", { body: "Raid detecte" });
} else if (Notification.permission !== "denied") {
  const perm = await Notification.requestPermission();
  if (perm === "granted") new Notification("Sentinel", { body: "Raid detecte" });
}
```

**Action requise :**
- [ ] Remplacer `sendNativeNotification()` dans `useNotifications.ts` (~10 lignes)

---

### 5. File picker (AI Training)

**Avant :** `@tauri-apps/plugin-dialog` pour selectionner un fichier CSV.

**Apres :** `<input type="file">` HTML standard.

```typescript
// Avant
import { open } from "@tauri-apps/plugin-dialog";
const path = await open({ filters: [{ name: "CSV", extensions: ["csv"] }] });
await invoke("ai_upload_dataset", { modelType, filePath: path });

// Apres
const input = document.createElement("input");
input.type = "file";
input.accept = ".csv";
input.onchange = async () => {
  const file = input.files[0];
  const formData = new FormData();
  formData.append("file", file);
  await fetch(`${API_BASE}/api/ai/upload`, { method: "POST", body: formData });
};
input.click();
```

**Action requise :**
- [ ] Modifier `AiTrainingPage.vue` — remplacer `open()` par `<input type="file">`
- [ ] Ajouter un endpoint upload multipart a l'API (si pas encore existant)

---

### 6. Mapping des commandes Tauri → API REST

Chaque `invoke("command_name", { params })` doit etre mappe vers un endpoint HTTP.

#### Lectures (GET)

| Commande Tauri | Endpoint API |
|----------------|-------------|
| `get_guilds` | `GET /api/guilds` |
| `get_guild_overview` | `GET /api/stats/{guild_id}/overview` |
| `get_dashboard_stats` | `GET /api/stats` |
| `get_bot_definitions` | `GET /api/bots/definitions` |
| `get_bot_config` | `GET /api/bots/config/{guild_id}/{bot_name}` |
| `get_all_bot_config` | `GET /api/bots/config/{guild_id}` |
| `get_infractions` | `GET /api/infractions/{guild_id}` |
| `get_rules` | `GET /api/rules/{guild_id}` |
| `get_moderation_history` | `GET /api/moderation/history/{guild_id}/{user_id}` |
| `get_bans` | `GET /api/moderation/bans` |
| `get_tickets` | `GET /api/tickets` |
| `get_ticket_detail` | `GET /api/tickets/{id}` |
| `get_security_events` | `GET /api/security/events` |
| `get_voice_channels` | `GET /api/voice-channels/{guild_id}` |
| `get_conduct_config` | `GET /api/conduct/config/{guild_id}` |
| `get_conduct_leaderboard` | `GET /api/conduct/{guild_id}/leaderboard` |
| `get_conduct_points` | `GET /api/conduct/{guild_id}/{user_id}` |
| `get_conduct_log` | `GET /api/conduct/{guild_id}/{user_id}/log` |
| `get_audit_logs` | `GET /api/audit-logs` |
| `get_watched_users` | `GET /api/watched-users` |
| `get_user_dossier` | `GET /api/watched-users/{guild_id}/{user_id}` |
| `get_levels_config` | `GET /api/levels/config/{guild_id}` |
| `get_levels_leaderboard` | `GET /api/levels/{guild_id}/leaderboard` |
| `get_level_rewards` | `GET /api/levels/rewards/{guild_id}` |
| `get_role_panels` | `GET /api/role-panels/{guild_id}` |
| `get_discord_roles` | `GET /api/discord-roles/{guild_id}` |
| `get_ia_config` | `GET /api/ia-config/{guild_id}` |
| `get_full_analytics` | `GET /api/analytics` |
| `get_logs` | `GET /api/logs` |
| `get_guild_members` | `GET /api/guilds/{guild_id}/members` |
| `get_member_summary` | `GET /api/members/{guild_id}/{user_id}/summary` |
| `get_bot_heartbeat` | `GET /api/bots/heartbeat` |
| `get_cache_stats` | `GET /api/cache/stats` |
| `get_models_status` | `GET /api/models/status` |
| `get_coude_combats` | `GET /api/coude/{guild_id}/combats` |
| `get_coude_players` | `GET /api/coude/{guild_id}/players` |

#### Ecritures (POST/PUT/PATCH/DELETE)

| Commande Tauri | Endpoint API | Methode |
|----------------|-------------|---------|
| `set_bot_config` | `/api/bots/config` | POST |
| `delete_bot_config` | `/api/bots/config` | DELETE |
| `delete_infraction` | `/api/infractions/delete/{id}` | DELETE |
| `update_rule` | `/api/rules` | POST |
| `execute_ban` | `/api/moderation/execute-ban` | POST |
| `execute_unban` | `/api/moderation/execute-unban` | POST |
| `reply_ticket` | `/api/tickets/{id}/messages` | POST |
| `close_ticket` | `/api/tickets/{id}/close` | PATCH |
| `assign_ticket` | `/api/tickets/{id}/assign` | PATCH |
| `save_conduct_config` | `/api/conduct/config` | POST |
| `adjust_conduct_points` | `/api/conduct/{guild_id}/{user_id}/add` | POST |
| `add_watched_user` | `/api/watched-users` | POST |
| `remove_watched_user` | `/api/watched-users/{guild_id}/{user_id}` | DELETE |
| `save_levels_config` | `/api/levels/config` | POST |
| `set_level_reward` | `/api/levels/rewards` | POST |
| `delete_level_reward` | `/api/levels/rewards/{guild_id}/{level}` | DELETE |
| `save_ia_config` | `/api/ia-config/{guild_id}` | PUT |
| `sync_discord_roles` | `/api/discord-roles/{guild_id}/sync` | POST |
| `create_discord_role` | `/api/discord-roles/{guild_id}/create` | POST |
| `edit_discord_role` | `/api/discord-roles/{guild_id}/{role_id}` | PATCH |
| `delete_discord_role` | `/api/discord-roles/{guild_id}/{role_id}` | DELETE |
| `delete_logs_by_category` | `/api/logs/{category}` | DELETE |
| `cancel_coude_combat` | `/api/coude/combats/{id}` | DELETE |
| `adjust_coude_coins` | `/api/coude/players/{guild_id}/{user_id}/coins` | PATCH |
| `reload_models` | `/api/models/reload` | POST |

#### Commandes speciales (pas de mapping direct)

| Commande Tauri | Action web |
|----------------|-----------|
| `discord_login` | Redirect OAuth → `/auth/discord` |
| `get_current_user` | `GET /auth/me` |
| `logout` | Supprimer localStorage + redirect |
| `has_discord_config` | Verifier si token existe en localStorage |
| `save_discord_config` | Stocker en localStorage |
| `save_api_config` | Stocker `API_URL` en localStorage |
| `ws_connect` | `new WebSocket(url)` |
| `ws_disconnect` | `ws.close()` |
| `ws_status` | `ws.readyState` |
| `save_bot_token` | `POST /api/bots/tokens` (a creer si necessaire) |
| `delete_bot_token` | `DELETE /api/bots/tokens/{bot}` (a creer si necessaire) |
| `ai_upload_dataset` | `POST /api/ai/upload` (multipart) |
| `ai_start_training` | `POST /api/ai/train` |
| `ai_stop_training` | `POST /api/ai/stop` |
| `ai_training_status` | `GET /api/ai/status` |

---

## 7. Structure du projet web

```
apps/web/                         # Nouveau dossier
  public/
  src/
    components/                   # Copie de desktop/src/components (inchange)
    composables/                  # Copie + modifications (voir ci-dessous)
      useFetch.ts                 # REECRIT (HTTP au lieu d'invoke)
      useAuth.ts                  # REECRIT (OAuth redirect + localStorage)
      useRealtime.ts              # REECRIT (WebSocket natif)
      useNotifications.ts         # MODIFIE (Web Notifications API)
      useGuildFetch.ts            # MODIFIE (utilise useFetch modifie)
      [tous les autres]           # INCHANGES (utilisent useFetch)
    router/                       # Copie (inchange, +1 route /callback)
    types/                        # Copie (inchange)
    services/
      api.ts                      # NOUVEAU — mapping commande → HTTP
      websocket.ts                # NOUVEAU — WebSocket wrapper
    App.vue                       # Copie (inchange)
    main.ts                       # MODIFIE (supprimer Tauri init)
  index.html
  vite.config.ts                  # NOUVEAU (build web standard)
  package.json                    # MODIFIE (supprimer deps Tauri)
  .env                            # VITE_API_URL=http://localhost:3000
```

---

## 8. Fichiers a modifier (exhaustif)

### Fichiers a REECRIRE (4)

| Fichier | Raison |
|---------|--------|
| `composables/useFetch.ts` | `invoke()` → `fetch()` avec mapping HTTP |
| `composables/useAuth.ts` | OAuth redirect + localStorage |
| `composables/useRealtime.ts` | WebSocket natif |
| `vite.config.ts` | Supprimer config Tauri |

### Fichiers a MODIFIER (6)

| Fichier | Modification |
|---------|-------------|
| `composables/useNotifications.ts` | `sendNotification()` → Web Notifications API |
| `composables/useAiTraining.ts` | File picker + upload multipart |
| `components/pages/AiTrainingPage.vue` | `<input type="file">` au lieu de dialog Tauri |
| `components/pages/SetupPage.vue` | Stockage API URL en localStorage |
| `components/pages/SettingsPage.vue` | Idem |
| `main.ts` | Supprimer init Tauri |

### Fichiers INCHANGES (~55)

Tous les autres composants, pages, composables, types et le router. **Aucun changement necessaire.**

---

## 9. API — Endpoints a ajouter

L'API existante couvre deja 95% des besoins. Il manque :

| Endpoint | Usage |
|----------|-------|
| `GET /auth/discord` | Initie le flow OAuth Discord (redirect) |
| `GET /auth/callback` | Recoit le code OAuth, retourne un JWT |
| `GET /auth/me` | Retourne l'utilisateur connecte (depuis le JWT) |
| `POST /api/ai/upload` | Upload de dataset CSV (multipart) |
| `POST /api/bots/tokens` | Sauvegarder un token de bot (optionnel) |
| `DELETE /api/bots/tokens/{bot}` | Supprimer un token de bot (optionnel) |

Les endpoints bots/tokens sont optionnels — ils servent uniquement au setup depuis le dashboard. En web, les tokens peuvent etre configures via `.env` cote serveur.

---

## 10. CORS

L'API a deja un middleware CORS (`ALLOWED_ORIGINS`). Ajouter l'URL du frontend web :

```env
ALLOWED_ORIGINS=http://localhost:1420,http://localhost:5173,https://dashboard.votredomaine.com
```

---

## 11. Securite web supplementaire

| Mesure | Raison |
|--------|--------|
| JWT avec expiration | Remplace le token en memoire Tauri |
| CSRF protection | Requis pour les appels POST/PUT/DELETE |
| Rate limiting par IP | L'API a deja un rate limiter, verifier qu'il couvre le web |
| `HttpOnly` cookies pour le JWT | Alternative a localStorage (plus securise) |
| CSP headers | Content Security Policy pour le frontend |

---

## 12. Checklist de migration

### Phase 1 : Setup (0.5 jour)
- [ ] Creer `apps/web/` avec Vite + Vue 3
- [ ] Copier `src/components`, `src/router`, `src/types`, `src/assets`
- [ ] Copier tous les composables
- [ ] Supprimer les dependencies Tauri du `package.json`
- [ ] Configurer `.env` avec `VITE_API_URL`

### Phase 2 : Bridge HTTP (1-2 jours)
- [ ] Creer `src/services/api.ts` avec le mapping commande → HTTP
- [ ] Reecrire `useFetch.ts` pour utiliser `fetch()`
- [ ] Reecrire `useAuth.ts` (localStorage + redirect OAuth)
- [ ] Reecrire `useRealtime.ts` (WebSocket natif)
- [ ] Modifier `useNotifications.ts` (Web Notifications API)
- [ ] Modifier `useGuildFetch.ts` si necessaire

### Phase 3 : Pages specifiques (0.5 jour)
- [ ] Modifier `AiTrainingPage.vue` (file input)
- [ ] Modifier `SetupPage.vue` / `SettingsPage.vue` (localStorage config)
- [ ] Ajouter route `/callback` pour OAuth

### Phase 4 : API backend (1 jour)
- [ ] Ajouter endpoints `/auth/discord`, `/auth/callback`, `/auth/me`
- [ ] Ajouter middleware JWT
- [ ] Ajouter endpoint upload AI dataset (multipart)
- [ ] Configurer CORS pour le domaine web

### Phase 5 : Tests + Deploy (0.5 jour)
- [ ] Tester toutes les pages
- [ ] Build production (`vite build`)
- [ ] Deployer (Nginx, Vercel, Cloudflare Pages, etc.)

---

## 13. Compatibilite desktop

La version desktop Tauri peut **coexister** avec la version web. Les deux partagent :
- Le meme backend API
- Les memes types TypeScript
- Les memes composants Vue

La seule difference est la couche de transport : `invoke()` vs `fetch()`.

Pour maintenir les deux, extraire les composants partages dans un package npm :
```
packages/
  ui/                  # Composants Vue partages
  types/               # Types TypeScript partages
apps/
  desktop/             # Tauri (inchange)
  web/                 # Nouveau frontend web
```
