# TODO — Sécurité réseau interne (Docker)

Document de suivi des améliorations possibles pour durcir l'isolation et
l'authentification entre services Docker.
Audit : 2026-05-01.

## Contexte

Le périmètre externe est bien sécurisé (firewall, ports 127.0.0.1, nginx
proxy uniquement sur 80/443). Le risque résiduel concerne le **mouvement
latéral** : si un service Rust est compromis (zero-day dans une dépendance,
RCE), à quoi peut-il accéder ?

État actuel : tous les services partagent le même réseau Docker, la même
`API_KEY`, et certains workers ont aussi le `DISCORD_TOKEN`. Un worker
exploité = accès quasi-complet à l'écosystème.

**Risque actuel** : modéré (zero-day requis pour exploiter), pas critique.
Ces améliorations sont de la défense en profondeur.

---

## 🟡 Priorité moyenne

### 1. Segmentation réseau Docker

**Symptôme** : `infra/docker/docker-compose.yml` — un seul network `sentinel`
pour tout. Un worker compromis peut joindre directement postgres/redis/api.

**Fix proposé** :
```yaml
networks:
  data:        # postgres, redis, pgbouncer (services data-only)
    internal: true
  app:         # api, gateway, workers, bot (services applicatifs)
  public:      # web (nginx), seul service exposé
```

Affectations :
- `postgres`, `redis`, `pgbouncer` → `data` (internal: true = pas d'accès internet)
- `api` → `data` + `app` (pivot autorisé)
- `gateway` → `app` (a besoin de Redis via `app`... à reconsidérer)
- Workers/bot → `app` uniquement (pas d'accès direct à `data`)
- `web` → `app` + `public`

**Impact** : un worker exploité ne peut plus parler à postgres/redis directement.
Il doit passer par l'API → laquelle a déjà des gates RBAC + middleware auth.

**Effort** : ~30 min (refactor compose), ~30 min de tests.

**Risque** : si mal configuré, des services peuvent perdre accès à Redis/DB.
À valider en staging avant prod.

---

### 2. mTLS sur gRPC inter-services

**Symptôme** : `services/api/src/adapters/inbound/grpc/server.rs:159` — gRPC
écoute sur `0.0.0.0:50051` en plain HTTP/2. Workers/bots se connectent via
`http://api:50051`. Le Bearer token transite en clair sur le réseau Docker.

Si un attaquant sniff le réseau Docker (via accès root sur l'host ou
compromission d'un autre conteneur), il peut récupérer la `API_KEY`.

**Fix proposé** :
- Générer un cert auto-signé partagé entre API et clients (script `init-grpc-tls.sh`)
- Côté API (`tonic`) : `Server::tls(...)` avec le cert
- Côté clients (workers/bot) : `Channel::tls(...)` avec le CA
- Monter les certs en read-only dans les conteneurs concernés

**Impact** : impossible de sniffer la `API_KEY` même avec un accès réseau.

**Effort** : ~1h.

**Risque** : low. Les certs sont auto-générés, pas de dépendance externe.

---

## 🔴 Priorité haute (gros chantier)

### 3. Tokens API par service

**Symptôme** : tous les services partagent la même `API_KEY` (variable env
unique). Un service compromis peut appeler **n'importe quel** endpoint API
en se faisant passer pour un autre service.

Localisations :
- `infra/docker/docker-compose.yml:40, 200, 249, 364, 437, 478` (env)
- `bots/shared/src/config.rs:7,20` (lecture côté clients)
- `services/api/src/adapters/inbound/http/middleware/auth.rs` (validation côté API)

**Fix proposé** :
1. Générer un token par service au boot (ex: `API_TOKEN_ANALYTICS_WORKER`,
   `API_TOKEN_SENTINEL_BOT`, etc.) — ou un token signé HMAC avec un secret unique
2. Dans l'API, table `service_tokens (service_name, token_hash, scopes JSONB)`
3. Middleware auth lit le token, identifie le service, vérifie les scopes
4. Chaque endpoint a un scope requis (ex: `wallet:write`, `audit:read`)
5. Migration progressive : accepter ancienne `API_KEY` ET nouveaux tokens
   pendant 1 release, puis retirer l'ancienne

**Impact** : un worker analytics exploité ne peut plus écrire dans `wallet`,
seulement lire les stats. Principe de moindre privilège.

**Effort** : ~3-4h (migration BDD + middleware + génération + rotation des
tokens dans les services Rust + tests).

**Risque** : moyen. Refactor de tous les clients API. Bug = service ne peut
plus parler à l'API. À tester soigneusement.

---

## 🟢 Priorité basse

### 4. Centraliser l'accès Discord via l'API

**Symptôme** : `discord-audit-sync-worker` et `coude-worker` ont le
`SENTINEL_DISCORD_TOKEN` directement. Si ces workers sont compromis →
contrôle complet du bot Discord.

**Fix proposé** : ces workers ne devraient parler à Discord qu'à travers
l'API (qui détient le token et applique audit logs). Refactor architectural,
gros effort.

**Effort** : ~6-8h selon le nombre d'appels Discord directs dans ces workers.

---

### 5. Redis ACL par service (Redis 6+)

**Symptôme** : tous les services partagent `REDIS_PASSWORD`. Un worker
compromis peut effacer toutes les streams.

**Fix proposé** : créer des users Redis par service avec permissions
granulaires (`+xadd ~sentinel:events:*`, `-flushall`, etc.).

**Effort** : ~1h.

---

## Notes

### Décisions explicites de NE PAS faire
- Pas de Vault/SOPS pour l'instant — overkill pour ce projet, `.env`
  + secrets git-ignored suffit.
- Pas de service mesh (Linkerd/Istio) — trop lourd, mTLS basique suffit.
- Pas d'IDS/IPS interne (Falco) — utile mais hors scope.

### Ce qui est déjà bien
- Tous les conteneurs run en non-root (uid 1000)
- `no-new-privileges:true` sur l'API
- `security_opt` strict
- Socket Docker uniquement sur l'API (gated superadmin)
- Comparaison API_KEY constant-time (pas de timing attack)
- Postgres / Redis / API : ports bind 127.0.0.1
- Discord OAuth CSRF protégé (state UUID + Redis)

---

## Plan d'action recommandé

| Ordre | Action | Effort | Priorité |
|---|---|---|---|
| 1 | Segmentation réseau Docker (#1) | 1h | 🟡 |
| 2 | mTLS gRPC (#2) | 1h | 🟡 |
| 3 | Tokens API par service (#3) | 3-4h | 🔴 |
| 4 | Redis ACL (#5) | 1h | 🟢 |
| 5 | Discord centralisé (#4) | 6-8h | 🟢 |

**Total quick wins** (1+2) : ~2h, gros gain pour effort modéré.
