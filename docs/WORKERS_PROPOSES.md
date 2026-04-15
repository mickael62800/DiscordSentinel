rf# Workers background — Analyse & propositions

Document d'analyse des workers existants et propositions d'extraction/création pour l'architecture DiscordSentinel.

Un **worker** ici = service Rust autonome qui tourne en background, écoute Redis/DB/cron, effectue des tâches asynchrones sans gateway Discord directe, et communique via l'API ou Redis pub/sub.

---

## État actuel : 6 workers existants

Situés dans `services/workers/` :

| Worker | Responsabilités |
|---|---|
| **moderation-worker** | Regen conduct, cleanup bans temporaires, sync proposals, reminders |
| **analytics-worker** | Snapshots quotidiens et horaires des stats |
| **cleanup-worker** | Purge des anciennes données |
| **cache-worker** | Invalidation/refresh Redis |
| **coude-worker** | Expiration des combats "Coup de Coude" |
| **monitoring-worker** | Écoute des événements Redis |

Architecture cohérente : `worker-common::spawn_periodic` + PostgreSQL + Redis + heartbeat API.

---

## Extractions depuis bots existants

### 🔴 Priorité haute

#### 1. `voice-afk-worker` (depuis `voice-bot`)

- **Extrait** : sweep AFK (60s) itérant tous les voice channels pour déplacer les membres inactifs
- **Pourquoi** : tâche lourde en appels Discord API → risque de rate-limit sur le gateway. L'extraction protège la latence des events voice temps-réel (join/leave/speak)

#### 2. `temp-roles-worker` (depuis `community-bot`)

- **Extrait** : cleanup des rôles temporaires expirés (60s)
- **Pourquoi** : read-heavy DashMap + appels Discord REST. Aucune raison d'occuper le gateway community-bot qui gère déjà messages/reactions

### 🟡 Priorité moyenne

#### 3. `audit-cache-worker` (depuis `audit-bot`)

- **Extrait** : refresh du cache watched-users (60s)
- **Pourquoi** : batch refresh optimisable avec diff DB, permet à audit-bot de rester purement event-driven

#### 4. `blackjack-cleanup-worker` (depuis `blackjack-bot`)

- **Extrait** : fermeture des tables AFK
- **Pourquoi** : léger mais 100% découplable ; permet à blackjack-bot de ne gérer que l'interaction joueur

---

## Nouveaux workers à créer

### 🔴 Haute valeur

#### 5. `sanction-expiry-worker`

- **Rôle** : poll les mutes/bans temporaires, DM le modérateur 24h avant expiration, poste "sanction expirée" dans le salon de logs à l'échéance
- **Pourquoi** : aujourd'hui les timeouts expirent silencieusement. Tâche purement périodique, parfaite pour un worker

#### 6. `appeal-sla-worker`

- **Rôle** : surveille les tickets d'appel ouverts, escalade ceux > 48h sans réponse, notifie les seniors, ferme automatiquement les appels abandonnés > 14j
- **Pourquoi** : aucun SLA n'est actuellement appliqué sur les appels. Tâche scheduled idéale

#### 7. `discord-audit-sync-worker`

- **Rôle** : pull régulier du Discord Audit Log API pour réconcilier les actions manuelles (bans/kicks faits via client Discord) avec la DB moderation
- **Pourquoi** : détecte les actions hors-bot, crucial pour l'intégrité des stats et de l'historique

### 🟡 Moyenne valeur

#### 8. `stats-digest-worker`

- **Rôle** : génère un digest hebdomadaire par modérateur (actions, taux d'appel, overturns) et poste dans un salon staff
- **Pourquoi** : alimente la future commande `/modstats` sans charger l'API en synchrone

#### 9. `reminder-worker` (générique)

- **Rôle** : service unique de rappels programmés réutilisable par tous les bots (ticket inactif, appel en attente, rôle temporaire, note de suivi). Consomme une table `scheduled_reminders` unifiée
- **Pourquoi** : évite que chaque bot réimplémente son propre scheduler

#### 10. `export-worker`

- **Rôle** : déporte les exports JSON/CSV lourds (`/export` moderation-bot, historiques longs) en jobs asynchrones avec notification DM quand le fichier est prêt
- **Pourquoi** : un export de 10k lignes bloque actuellement le bot. Worker async = meilleure UX

#### 11. `ai-worker` 🔴 (très forte valeur)

- **Rôle** : déporte tous les appels IA texte et image (actuellement dans `services/api`) vers un worker dédié qui consomme une queue de jobs
- **Pourquoi** : **c'est le cas d'usage le plus justifié pour un worker dans le projet**. Un appel LLM prend 2–15s, une génération d'image 5–60s. Garder ça dans un handler HTTP Axum = catastrophe en termes de latence, rate-limits et résilience.

##### Problèmes actuels (IA dans l'API)

| Problème | Impact |
|---|---|
| **Latence HTTP** | Un handler qui bloque 30s mange un worker tokio, fait timeout les clients (reqwest 30s, nginx 60s) |
| **Rate-limits** | Chaque pod API qui scale multiplie les appels parallèles → quotas OpenAI/Replicate explosés |
| **Résilience** | Un échec provider = HTTP 500 au bot, pas de retry propre |
| **Coût tracking** | Impossible de centraliser les tokens consommés et de facturer par guild |
| **Streaming** | Impossible de streamer token-par-token via request/response HTTP |
| **IA locale** | Si Ollama/SD local : modèles 4–16 GB RAM qui coexistent avec l'API HTTP = fatal |

##### Architecture recommandée

```
┌─────────┐   POST /ai/jobs        ┌──────────┐
│  Bot    │ ─────────────────────> │   API    │  (persiste job_id, 202 immédiat)
│ Discord │                        │  Axum    │
└─────────┘                        └────┬─────┘
     ▲                                  │
     │                                  ▼
     │                            ┌──────────┐
     │   Redis pub/sub            │ ai_jobs  │  (table Postgres ou Redis Stream)
     │   "ai_result:{job_id}"     └────┬─────┘
     │                                 │
     │                                 ▼
     │                           ┌──────────┐
     └─────────────────────────  │ ai-worker│ ──> OpenAI / Replicate / Ollama
                                 └──────────┘
```

**Flow** :
1. Bot reçoit `/imagine prompt` → POST `/ai/jobs` → répond "⏳ en cours…" avec un `job_id`
2. API enregistre le job et retourne `202 Accepted` immédiatement (pas de blocage)
3. `ai-worker` pop le job, appelle le provider, stocke le résultat (image en S3/disk, texte en DB)
4. Worker publie sur Redis `ai_result:{job_id}` avec l'URL/contenu
5. Bot écoute le pub/sub et édite le message Discord initial avec le résultat

##### Répartition des responsabilités

**Reste dans l'API :**
- Endpoints `POST /ai/jobs`, `GET /ai/jobs/:id`, `GET /ai/jobs/:id/result`
- Validation prompt (longueur, contenu, quota guild)
- Persistance des jobs et résultats
- Métriques exposées

**Part dans `ai-worker` :**
- Appels aux providers (OpenAI, Anthropic, Replicate, Ollama local…)
- Rate-limiting global (1 seul limiter pour tout le système)
- Retry exponentiel avec dead-letter queue
- Upload des images vers stockage (S3/Minio/disk)
- Modération du contenu sorti (scan NSFW avant retour au bot)
- Streaming des tokens via Redis pub/sub (si texte)

##### Bénéfices concrets

| Axe | Avant (dans API) | Après (worker) |
|---|---|---|
| Latence réponse API | 5–60s bloquants | <50ms (retourne job_id) |
| Scalabilité | Bloque les threads Axum | Worker scalable indépendamment |
| Rate-limit | Chaos entre pods API | Centralisé, 1 seul limiter |
| Coût tracking | Difficile | Tout centralisé au worker |
| Résilience | 500 sur échec | Retry automatique |
| RAM (si IA locale) | Modèles dans process API | Isolé dans le worker/GPU dédié |

##### Variante : séparer texte et image

Si les providers divergent fortement (OpenAI texte + SDXL local image) :
- `ai-text-worker` — LLM, streaming, prompts courts, latence 2–15s
- `ai-image-worker` — génération image, GPU-bound, latence 5–60s, upload vers stockage

Permet d'allouer des ressources différentes (GPU seulement pour l'image) et de scale indépendamment.

---

## Tableau de priorisation

| Priorité | Worker | Type | Effort | Impact |
|---|---|---|---|---|
| 🔴 | `voice-afk-worker` | Extraction | Moyen | Élevé |
| 🔴 | `temp-roles-worker` | Extraction | Faible | Élevé |
| 🔴 | `sanction-expiry-worker` | Nouveau | Moyen | Élevé |
| 🔴 | `appeal-sla-worker` | Nouveau | Moyen | Élevé |
| 🔴 | `discord-audit-sync-worker` | Nouveau | Élevé | Élevé |
| 🟡 | `audit-cache-worker` | Extraction | Faible | Moyen |
| 🟡 | `blackjack-cleanup-worker` | Extraction | Faible | Faible |
| 🟡 | `stats-digest-worker` | Nouveau | Moyen | Moyen |
| 🟡 | `reminder-worker` | Nouveau | Élevé | Élevé (long terme) |
| 🟡 | `export-worker` | Nouveau | Moyen | Moyen |
| 🔴 | `ai-worker` | Extraction API | Moyen | **Très élevé** |

---

## Gains attendus

| Axe | Bénéfice |
|---|---|
| **Latence gateway** | Les bots voice/community/audit restent réactifs aux events Discord |
| **Résilience** | Un worker qui crash ne tombe pas avec son bot |
| **Scalabilité** | Workers scalables horizontalement, indépendamment des bots |
| **Testabilité** | Logique métier sortie des handlers Discord = testable unitairement |
| **Rate-limits** | Tâches REST-heavy isolées du gateway principal |

---

## Recommandation d'ordre d'implémentation

1. **`ai-worker`** — priorité maximale : débloque l'API HTTP, évite les timeouts, centralise coûts et rate-limits
2. **`sanction-expiry-worker`** — quick win, aligné avec `bots/moderation-bot/AMELIORATIONS.md`
3. **`temp-roles-worker`** — extraction simple, impact immédiat sur community-bot
4. **`voice-afk-worker`** — libère le voice-bot des appels REST lourds
5. **`reminder-worker`** — infrastructure réutilisable qui simplifiera les workers suivants
6. **`appeal-sla-worker`** + **`discord-audit-sync-worker`** — complètent la couverture modération
