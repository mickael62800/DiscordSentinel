# Audit sécurité — apps/web (à corriger plus tard)

Rapport d'audit du panneau d'administration Vue.js (`apps/web/`).
Classement par sévérité. Chaque point = fichier:ligne + problème + fix proposé.

---

## CRITIQUES

### 1. `client_secret` Discord stocké côté navigateur
**Fichiers** : `src/api/config.ts` (setters/getters Discord config), `src/components/pages/SetupPage.vue:137-145`, `src/stores/authStore.ts:42-45`

Le formulaire Setup demande à l'utilisateur de saisir le `client_secret` Discord, qui est ensuite persisté en `localStorage`. Un `client_secret` est par définition un secret **serveur** — il ne doit **jamais** atteindre le navigateur. Un simple XSS = vol total et possibilité d'usurper l'application Discord.

**Fix** :
- Déplacer `DISCORD_CLIENT_ID` / `DISCORD_CLIENT_SECRET` en variables d'environnement de l'API (`services/api/.env`).
- Exposer deux routes backend :
  - `GET /auth/discord/authorize` → renvoie l'URL Discord construite côté serveur.
  - `GET /auth/discord/callback?code=...` → le backend échange `code` ↔ token auprès de Discord, récupère le user, crée une session.
- Supprimer toute saisie/stockage de `client_secret` côté front.

---

### 2. API key (Bearer) stockée en localStorage
**Fichier** : `src/api/config.ts` (fonctions d'accès à `apiKey`), utilisé dans `src/api/http.ts`

L'API key est lue/écrite en `localStorage`. XSS = extraction immédiate → accès complet à l'API Axum.

**Fix** :
- Passer à un modèle **session cookie HTTP-only** : après login OAuth réussi, le backend pose un cookie `Set-Cookie: session=...; HttpOnly; Secure; SameSite=Strict`.
- Retirer `X-API-Key` du front. L'API key reste un secret backend utilisé pour les appels inter-services, pas pour le navigateur.
- Ajouter `credentials: 'include'` sur les `fetch` + configurer `Access-Control-Allow-Credentials: true` côté Axum.

---

### 3. Tokens de bots Discord stockés en localStorage
**Fichiers** : `src/api/config.ts:44-61`, `src/components/pages/ComponentConfigPage.vue:39-44`

Les tokens des bots Discord (automod, moderation, ticket, etc.) sont écrits en clair dans `localStorage` via le panneau de config. XSS = compromission directe de **tous** les bots gérés.

**Fix** :
- Les tokens de bots ne doivent exister que dans le `.env` backend (déjà le cas pour le serveur).
- Le panneau de config doit envoyer les tokens au backend via une route dédiée (`POST /config/bot-token`) qui les chiffre au repos (LMDB avec clé dérivée, ou variables d'env).
- Le front ne redescend **jamais** un token déjà stocké — il affiche juste "configuré" / "non configuré".

---

### 4. Aucune validation du schéma d'URL API
**Fichiers** : `src/api/http.ts:7`, `src/components/pages/SetupPage.vue:34`, `src/services/configService.ts:12-13`

La page Setup accepte n'importe quelle chaîne comme URL d'API. Un utilisateur (ou un attaquant via XSS) peut saisir `javascript://…`, `file://…`, `data://…`, ou pointer vers un serveur contrôlé pour du credential harvesting.

**Fix** : dans `SetupPage.vue`, valider avant save :
```ts
function validateApiUrl(url: string): boolean {
  try {
    const u = new URL(url);
    return u.protocol === "http:" || u.protocol === "https:";
  } catch {
    return false;
  }
}
```
Même validation dans `configService.saveApiConfig` en garde-fou.

---

## MOYENNES

### 5. Pas de Content-Security-Policy
**Fichier** : `apps/web/index.html`

Aucune `<meta http-equiv="Content-Security-Policy">`. En cas d'XSS découvert plus tard, aucune mitigation.

**Fix** : ajouter dans `<head>` :
```html
<meta http-equiv="Content-Security-Policy"
  content="default-src 'self';
           script-src 'self';
           style-src 'self' 'unsafe-inline';
           img-src 'self' data: https://cdn.discordapp.com;
           connect-src 'self' http://localhost:3000 ws://localhost:3001;">
```
À ajuster selon les origines réelles en prod (HTTPS uniquement).

---

### 6. Guards du router contournables via DevTools
**Fichier** : `src/main.ts:14-21`, `src/router/index.ts`

Le `beforeEach` vérifie `user.value` qui vient de `localStorage`. Un attaquant avec accès à DevTools peut injecter un faux user et naviguer dans l'UI.

**Fix** :
- Acceptable **si** l'API valide systématiquement l'auth côté serveur (c'est le cas avec `X-API-Key`/session).
- Une fois la session cookie en place (cf. point 2), vérifier via un `GET /auth/me` au chargement de l'app : si 401, forcer `/login`. Ne jamais se fier uniquement au localStorage.

---

### 7. URL WebSocket dérivée sans validation
**Fichier** : `src/services/realtimeService.ts:14-18`

L'URL WebSocket est construite à partir de l'URL API stockée (qui est user-input). Si le hostname est malveillant, on peut se connecter à `ws://attacker:3001/ws?token=...` et **leaker le token dans la query string**.

**Fix** :
- Valider que le hostname du WS correspond bien à celui de l'API (`new URL(apiUrl).hostname`).
- Cf. aussi point 4 (validation de l'URL API à la source).

---

### 8. Token dans query string loggé / exposé
**Fichier** : `src/services/realtimeService.ts:38`

`emit("ws:connected", { connected: true, url: wsUrl })` propage l'URL complète, laquelle contient le token en query string. N'importe quel listener (ou extension navigateur, ou log externe) peut récupérer le token.

**Fix** :
- Envoyer le token via **Subprotocol** WebSocket (`new WebSocket(url, ['bearer', token])`) ou via le premier message après `open` — jamais en query string.
- Ou a minima, nettoyer l'URL avant log : `wsUrl.split('?')[0]`.

---

## FAIBLES / INFO

### 9. Mixed content potentiel en prod
**Fichier** : `src/api/http.ts:7` et :53 (défauts `http://localhost:3000`, `http://localhost:8000`)

Les défauts sont HTTP. En prod, si la web app est servie en HTTPS, les appels HTTP seront bloqués (mixed content).

**Fix** : ajouter un check en prod : si `location.protocol === 'https:'` et que l'URL API est `http://`, throw avec un message clair.

---

### 10. Headers sensibles envoyés sans CSRF
**Fichier** : `src/api/http.ts:10-16`

`X-Discord-Token` est envoyé tel quel. Une fois passé en cookie session (point 2), ajouter une protection CSRF (double-submit token ou `SameSite=Strict` suffit pour la plupart des cas).

---

## RAS (déjà OK)

- **XSS via `v-html`** : aucune occurrence dans les `.vue`.
- **`innerHTML` / `document.write`** : aucune occurrence.
- **Open redirects** : `router.push()` utilise des routes statiques, pas de user-input.
- **CORS credentials** : pas de `credentials: 'include'` hors contexte (à ajouter quand session cookie sera en place).
- **Dépendances** : Vue 3.5, Vite 6, Pinia 3 — versions récentes, pas de CVE connue.

---

## Ordre d'attaque recommandé

1. **Quick wins** (heures) : points **4, 5, 7, 8, 9** — validation d'URL, CSP, nettoyage des logs WS. Zéro impact backend.
2. **Refonte auth** (jours) : points **1, 2, 6** ensemble — backend gère `client_secret`, flux OAuth complet, session cookie HTTP-only, suppression du stockage localStorage sensible. Touche backend + web + potentiellement desktop (à voir si on garde le flux Tauri localhost:19836 en parallèle).
3. **Gestion des tokens bots** (jours) : point **3** — route backend dédiée + chiffrement au repos.
4. **Hardening** (heures) : point **10** une fois la session cookie en place.
