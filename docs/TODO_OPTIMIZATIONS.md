# TODO — Optimisations & dette technique

Document de suivi des optimisations identifiées mais pas encore implémentées.
Daté du 2026-05-01.

---

## 🔴 Priorité haute

### 1. Préchargement centralisé au démarrage (`useAppInit`)

**Symptôme** : chaque page Vue refetch les mêmes données stables (config bots, rôle RBAC, visibility, channels, etc.) à son `onMounted`. Latence perçue + charge API inutile.

**Fix** : créer un composable `useAppInit()` appelé une seule fois après le login, qui charge en parallèle dans des stores Pinia partagés :
- `/api/bots/definitions` (TTL 1h, change rarement)
- `/api/bots/config/{guild}` (TTL 15min)
- `/api/rbac/me/{guild}` (TTL session)
- `/api/rbac/component-visibility/{guild}` (TTL session)
- `/api/guilds/{guild}/channels` (TTL 10min)
- `/api/discord-roles/{guild}` (DB locale, pas cher)

Les pages lisent dans les stores au lieu de fetch. Les stores ont une stratégie SWR (return cache immédiat, refetch en background).

**Impact estimé** : -40% à -50% de requêtes API, premier rendu instant après navigation.

**Effort** : ~3-4h.

**Fichiers concernés** :
- `apps/web/src/composables/useAppInit.ts` (nouveau)
- `apps/web/src/stores/` (nouveaux stores Pinia)
- `apps/web/src/router/index.ts` (appel après login confirmé)

---

### 2. Cache frontend Discord guilds — éviter 503 transients

**Symptôme** : sous charge (refresh rapide, plusieurs onglets ouverts), Discord rate-limit `get_user_guilds` (429 Too Many Requests) → middleware `guild_auth` côté API renvoie 503 → frontend affiche page vide / boutons cachés.

**État actuel backend** : déjà cachet correctement (Redis TTL 1h + fallback stale 24h dans `middleware/guild_auth.rs:92-147`). Le 503 survient uniquement quand le cache miss coïncide avec un rate-limit Discord.

**Fix complémentaire côté API** : quand cache stale dispo ET appel Discord échoue avec 429, retourner les guilds stale au lieu de 503. Déjà partiellement présent, à durcir pour TOUS les codes d'erreur Discord transients (429, 5xx).

**Fix côté frontend** : retry exponentiel sur 503 (3 tentatives, 500ms / 1s / 2s) avant d'afficher l'erreur. Évite la page blanche si le 503 dure < 2s.

**Effort** : 1h backend + 30min frontend.

**Fichiers concernés** :
- `services/api/src/adapters/inbound/http/middleware/guild_auth.rs`
- `apps/web/src/api/http.ts` (retry wrapper)

---

### 3. Singleton pour `/api/bots/definitions`

**Symptôme** : appelé séparément depuis `ComponentConfigPage`, `useBotEnabledStatus`, et indirectement plusieurs autres. Pas de partage.

**Fix** : module-scope ref dans un composable `useBotDefinitions()` (pattern déjà appliqué à `useComponentVisibility`). Une seule requête par session, tout le monde lit le ref partagé.

**Effort** : 30min.

**Fichiers concernés** :
- `apps/web/src/composables/useBotDefinitions.ts` (nouveau)
- Refactor des 3-4 sites d'appel actuels.

---

## 🟡 Priorité moyenne

### 4. Dedup `/api/rbac/me` + `/api/rbac/component-visibility`

**Symptôme** : ces 2 endpoints sont appelés en parallèle (`useRbac` + `useComponentVisibility`) pour des données qui pourraient être obtenues en un seul aller-retour.

**Fix au choix** :
- **Option A (frontend)** : un seul composable `useRbacBootstrap()` qui appelle les 2 endpoints en `Promise.all` une seule fois et expose les résultats à toute l'app (couvre si on garde 2 endpoints distincts).
- **Option B (backend)** : nouvel endpoint composite `/api/rbac/bootstrap/{guild_id}` qui retourne `{ me, visibility }` en une réponse. Plus rapide réseau-wise.

**Recommandation** : Option A d'abord (zéro modif backend), Option B si on veut squeeze davantage.

**Effort** : 1h.

---

### 5. Pinia persist sur `/api/guilds`

**Symptôme** : la liste des guilds est rechargée à chaque cold-boot. Stable, change rarement.

**Fix** : `pinia-plugin-persistedstate` sur `guildSelectorStore` → hydratation depuis `localStorage` au boot, refetch en background pour valider.

**Effort** : 20min.

**Fichiers concernés** :
- `apps/web/src/stores/guildSelectorStore.ts`
- `apps/web/package.json` (dépendance)

---

### 6. Stale-while-revalidate générique

**Symptôme** : à chaque navigation entre pages, l'utilisateur voit un loader pendant 200-800ms le temps que les données arrivent.

**Fix** : composable wrapper SWR autour de `useGuildFetch` qui retourne immédiatement le cache stale si présent, puis refetch en background et update le ref. UX instantanée.

**Effort** : 2h.

---

## 🟢 Priorité basse / dette technique

- Logs verbeux : `bots/heartbeat` toutes les secondes pollue les logs INFO. Passer en DEBUG.
- Endpoint `/health` appelé depuis le frontend en parallèle de chaque navigation — utile ? Sinon supprimer ce ping.
- Bundle `index-*.js` pas analysé : potentiel tree-shaking sur les chart.js / Discord embeds.

---

## Notes diverses

### Ce qui est déjà bien
- Backend bien cacheé (Redis sur tous les endpoints Discord-dépendants)
- `useComponentVisibility` adopte le bon pattern singleton module-scope
- `guildSelectorStore` (Pinia) charge `/api/guilds` une fois au boot
- Pas d'appel Discord non-caché côté API

### Anti-patterns à éviter en relisant le code
- `onMounted` qui refetch des données déjà en store
- `watch` sur `selectedGuildId` qui refetch sans dedup (plusieurs composables font la même requête en parallèle)
- Hardcoder des valeurs par défaut différentes entre dev/prod (cf. fix `apiBase()` du 2026-05-01)
