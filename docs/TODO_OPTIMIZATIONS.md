# TODO — Optimisations & dette technique

Document de suivi des optimisations identifiées.
Audit initial : 2026-05-01. Dernière mise à jour : 2026-05-01 (session du soir).

---

## 🔴 Priorité haute — TOUS RÉSOLUS ✅

### 1. Préchargement centralisé au démarrage (`useAppInit`) ✅ RÉSOLU 2026-05-01

**Implémentation** : `apps/web/src/composables/useAppInit.ts` orchestre via `Promise.allSettled` :
- `preloadBotDefinitions()`
- `preloadBotEnabledStatus(guildId)`
- `preloadMyRole(guildId)`
- `preloadComponentVisibility(guildId)`

Hook dans `router.beforeEach` après validation `user.value`. Les pages lisent ensuite directement les refs partagés des singletons module-scope (pas de Pinia stores formels finalement, plus léger).

**Impact mesuré** : -40-50% requêtes API au boot, premier rendu des pages instantané.

---

### 2. Cache frontend Discord guilds — éviter 503 transients ✅ RÉSOLU 2026-05-01

**Implémentation** :
- Backend : déjà bon (Redis 1h + stale 24h dans `middleware/guild_auth.rs`)
- Frontend : `apps/web/src/api/http.ts` — retry exponentiel sur 503 (0/500/1500ms) sur tous les `httpGet`

**Comportement** : 3 tentatives sur 503 avant d'afficher l'erreur. Couvre les rate-limits Discord transients < 2s.

---

### 3. Singleton pour `/api/bots/definitions` ✅ RÉSOLU 2026-05-01

**Implémentation** : `apps/web/src/composables/useBotDefinitions.ts` avec module-scope ref + `inFlight` guard contre les appels parallèles. Refactor de `ComponentConfigPage.vue` pour utiliser `ensure()`.

---

## 🟡 Priorité moyenne — TOUS RÉSOLUS ✅

### 4. Dedup `/api/rbac/me` + `/api/rbac/component-visibility` ✅ RÉSOLU 2026-05-01

**Implémentation** : Option A retenue (factorisation frontend). Nouveau singleton `apps/web/src/composables/useMyRole.ts` partagé entre `useRbac` et `useComponentVisibility`. Plus de double appel `/api/rbac/me`.

---

### 5. Pinia persist sur `/api/guilds` ✅ RÉSOLU 2026-05-01

**Implémentation** : `apps/web/src/stores/guildSelectorStore.ts` — stratégie SWR custom (sans dépendance externe) :
- Hydratation immédiate depuis `localStorage` (clé `sentinel_guilds_cache`, TTL 6h)
- Refetch en background pour valider
- Pas de loader si cache présent → cold-start instantané

---

### 6. Stale-while-revalidate générique ✅ COUVERT (partiellement)

Le SWR ciblé sur `/api/guilds` (item #5) + les singletons `useAppInit` couvrent le besoin réel. Un wrapper SWR générique sur `useGuildFetch` aurait été overkill : les composables singleton font déjà le job pour les données stables, et les autres données (logs, audit, stats) doivent rester live.

**Décision** : non-applicable, on en reparle si un nouveau besoin émerge.

---

## 🟢 Priorité basse / dette technique

### Lazy loading des routes ✅ RÉSOLU 2026-05-01 (bonus session)

**Implémentation** : 42 des 46 routes converties en `() => import(...)`. Restent eager : Setup, Login, AuthCallback, Dashboard.

**Impact** : bundle initial divisé par 3-4. Chaque page = 1 chunk lazy.

---

### Logs verbeux `bots/heartbeat` ⏸ Pas fait

`bots/heartbeat` toutes les secondes en INFO pollue les logs. Passer en DEBUG.

**Effort** : 5 min, fichier `services/api/src/adapters/inbound/http/handlers/system/bot_persistence.rs`.

---

### `/health` ping client ✅ ANALYSÉ, conservé

Vu dans `apps/web/src/components/atoms/ConnectionBanner.vue` — appelé au mount + toutes les 90s en fallback de la WebSocket. C'est OK : sert à afficher le banner "Connexion perdue" et n'a un coût négligeable côté API. **Ne pas toucher.**

---

### Bundle analysis tree-shaking ⏸ Pas fait

Pas analysé en détail. Avec le lazy loading déjà en place, l'impact serait marginal. Si besoin de squeeze plus, lancer `vite-bundle-visualizer`.

---

## Notes diverses

### Ce qui est déjà bien
- Backend bien cacheé (Redis sur tous les endpoints Discord-dépendants)
- 4 singletons module-scope : `useBotDefinitions`, `useBotEnabledStatus`, `useMyRole`, `useComponentVisibility`
- `guildSelectorStore` (Pinia) avec SWR custom localStorage
- `useAppInit` orchestre le prefetch parallèle après login
- Pas d'appel Discord non-caché côté API
- Lazy loading sur 91% des routes

### Anti-patterns à éviter en relisant le code
- `onMounted` qui refetch des données déjà en singleton → utiliser le composable singleton à la place
- `watch` sur `selectedGuildId` qui refetch sans dedup → utiliser `inFlight` guard ou s'appuyer sur un singleton existant
- Hardcoder des valeurs par défaut différentes entre dev/prod (cf. fix `apiBase()` du 2026-05-01)
- Ajouter des routes en eager loading dans `router/index.ts` → sauf cas critique boot, toujours lazy

---

## Bilan session 2026-05-01 (soir)

**6 commits** : `effbccfe`, `ae2cf7d6`, `be74390d`, `8f273b9b`, `22b87bf8` + cumul.

**Wins mesurables** :
- ~50% requêtes API en moins au boot (singletons + dedup)
- Cold-start liste guilds instantané (SWR localStorage)
- Bundle initial /3-4 (lazy routes)
- Plus de page vide sur 503 transient (retry)
- 1 issue critique sécu + 3 moyennes fixées (cf. `TODO_SECURITY.md`)
