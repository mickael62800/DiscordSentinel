# TODO — Audit sécurité

Document de suivi des findings sécurité identifiés mais pas encore corrigés.
Audit réalisé le 2026-05-01.

---

## 🔴 Critique — à fixer en priorité

### 1. Timing attack sur la comparaison Bearer API key

**Sévérité** : 🔴 Critique
**Effort** : ~15 min

**Description**
La comparaison de la clé API utilise `==` (comparaison byte-par-byte qui short-circuit dès qu'un caractère diffère). Un attaquant peut mesurer la latence pour deviner la clé caractère par caractère via timing attack.

**Localisation**
`services/api/src/adapters/inbound/http/middleware/auth.rs:28`

**Fix recommandé**
Ajouter le crate `subtle = "2"` à `services/api/Cargo.toml` et remplacer le `==` par `subtle::ConstantTimeEq`.

```rust
use subtle::ConstantTimeEq;
if provided.as_bytes().ct_eq(expected.as_bytes()).into() {
    // OK
}
```

---

## 🟡 Moyenne — à fixer dans la foulée

### 2. Token Discord en localStorage

**Sévérité** : 🟡 Moyenne
**Effort** : ~30 min

**Description**
Le token Discord OAuth est stocké dans `localStorage`, donc persistant entre sessions et accessible à tout JS chargé sur le domaine. En cas de XSS futur (même mineur), le token est exfiltrable.

**Localisation**
`apps/web/src/api/config.ts:67-70`

**Fix recommandé**
Migrer vers `sessionStorage` (perd le token au close du navigateur, mais limite l'exfiltration) OU stocker en mémoire dans un store Pinia (`pinia-plugin-persistedstate` désactivé pour ce store) avec re-login si tab reload — plus strict.

Compromis recommandé : `sessionStorage`. La clé API du Setup peut rester en `localStorage` (moins sensible que le token Discord d'un user authentifié).

---

### 3. CSP frontend trop permissif

**Sévérité** : 🟡 Moyenne
**Effort** : ~3 h (gros chantier Vite)

**Description**
La CSP nginx du frontend autorise `'unsafe-inline'` pour scripts ET styles, ce qui annule une grande partie de la protection CSP contre les XSS. Vite injecte des scripts/styles inline au build, donc on est obligé d'autoriser pour l'instant.

**Localisation**
`apps/web/nginx.conf:74`

**Fix recommandé**
Configurer Vite pour utiliser des nonces ou hashes CSP (`vite-plugin-csp` ou équivalent). Générer un nonce par requête nginx et l'injecter dans la directive CSP + sur tous les `<script>`/`<style>`. Demande de la rigueur car chaque plugin Vite doit respecter la CSP.

**Alternative low-effort** : ne pas fixer pour l'instant. La CSP actuelle a quand même de la valeur (`default-src 'self'`, restrictions img/connect/font correctes), `'unsafe-inline'` est juste l'élément faible.

---

### 4. Docker socket monté en RW sans audit log applicatif

**Sévérité** : 🟡 Moyenne
**Effort** : ~45 min

**Description**
`/var/run/docker.sock` est monté en RW dans le conteneur API (nécessaire pour `/api/docker/*`). Les actions destructives (start/stop/restart/prune) sont bien gated par `require_superadmin`, mais il n'y a pas d'audit log de qui a lancé quoi et quand. Si la clé superadmin fuit, aucune trace.

**Localisation**
`infra/docker/docker-compose.yml:221` (le mount)
`services/api/src/adapters/inbound/http/handlers/system/docker.rs` (les handlers à instrumenter)

**Fix recommandé**
Ajouter un appel `tracing::info!` ou un insert dans `audit_logs` dans chaque handler Docker destructif (`start_container`, `stop_container`, `restart_container`, `remove_container`, `remove_image`, `remove_volume`, `prune_*`) avec :
- `actor.discord_user_id` (extrait du `RoleContext`)
- `action` (ex: `"docker.container.stop"`)
- `target` (id du conteneur / image / volume)
- `timestamp`

Idéalement réutiliser la table `audit_logs` existante.

---

## ✅ Findings positifs (pas d'action nécessaire)

Pour mémoire — ce qui est déjà solide :

- **Discord OAuth2 CSRF** : state UUID, Redis TTL 10min, consommation atomique au callback (`handlers/system/oauth.rs:82-183`)
- **RBAC fail-closed** : `RoleContext` injecté par middleware, défaut Viewer (principle of least privilege)
- **Endpoints destructifs gated** : tous les DELETE/POST sensibles passent par `require_role` / `require_superadmin`
- **Pas de logs de secrets** : aucun `tracing::info!` ne loggue `token`/`password`/`secret`/`api_key`
- **Messages d'erreur neutres** : 500 → `"Erreur interne"`, pas de stack trace ou SQL exposé
- **`.env` dans `.gitignore`** ✓
- **CSP API stricte** : `default-src 'none'; frame-ancestors 'none'; base-uri 'none'; form-action 'none'`
- **Headers complets** : HSTS 1 an, X-Frame-Options DENY, Referrer-Policy strict-origin, Permissions-Policy ✓
- **Rate limiter par IP** : token bucket + cleanup périodique 60s + cap 50k IPs (pas d'OOM via flood IPs uniques)
- **Endpoints IA durcis** : rate 5 req/s strict
- **Pagination bornée** : `.min(50)`, `.min(200)` partout, pas de DoS via `limit=999999`
- **Path params validés** : `validate_discord_id` + regex snowflake 17-20 digits
- **Body size limité** : `RequestBodyLimitLayer` configurable via env
- **Pas de SQLi** : tous les `format!()` dans les queries n'utilisent QUE des constantes hardcoded (`MEMBER_RESET_TABLES`, `COUDE_PURGE_TABLES`)
- **Pas de XSS** : aucun `v-html`, `innerHTML`, `eval`. Avatars passent par `safeImageUrl()` qui whitelist Discord CDN
- **Pas de command injection** : aucun `std::process::Command` exposé à de l'input user
- **Dépendances à jour** : axum 0.8, tokio latest, sqlx, redis, ort 2.0-rc12. `deny.toml` liste les advisories acceptés
- **Docker non-root** : `nginx:1.27-alpine` final stage, pas d'`USER=root` dans le Dockerfile
- **Ports bind 127.0.0.1** : api/gateway/pgadmin/redis-commander seulement accessibles localhost (fix appliqué le 2026-05-01)
- **`rel="noopener noreferrer"`** sur tous les `target="_blank"`
- **Pas de Service Worker** exposant l'API key
- **Pas de `postMessage`** ou `window.opener` à risque

---

## Plan d'action recommandé

| Ordre | Action | Effort | Priorité |
|---|---|---|---|
| 1 | Fix #1 (timing attack API key) | 15 min | 🔴 |
| 2 | Fix #2 (token Discord → sessionStorage) | 30 min | 🟡 |
| 3 | Fix #4 (audit log Docker actions) | 45 min | 🟡 |
| 4 | Fix #3 (CSP nonces) — gros chantier | 3 h | 🟡 |

**Total essentiel (1+2+4)** : ~1h30
**Total complet (1+2+3+4)** : ~4h30

La #3 peut être reportée car la CSP actuelle a déjà de la valeur, et le bénéfice marginal (passer de "bonne" à "excellente") demande un gros refactor.
