# ai-worker

**Rôle** : Dépile la file `ai_jobs` et dispatch les requêtes d'inférence vers l'API backend, puis persiste les résultats et publie sur Redis pub/sub. Créé en Phase 4 A.

## Jobs périodiques

| Job | Intervalle défaut | Fichier |
|---|---|---|
| `drain_ai_jobs` | 2 s (`AI_POLL_INTERVAL`) | `src/jobs/drain_ai_jobs.rs` |

### Logique du drain

1. Reset des jobs zombies (`status='processing'` depuis plus de `AI_JOB_TIMEOUT` secondes → remis à `pending`).
2. Claim atomique d'un batch de 5 jobs via `UPDATE ... FROM (SELECT ... FOR UPDATE SKIP LOCKED) RETURNING ...` — concurrence-safe pour scaler horizontalement.
3. Pour chaque job : appel HTTP vers `POST /analyze` ou `POST /analyze/image` de l'API.
4. Persiste le résultat (`status='done'`, `result_payload`, `completed_at`) ou incrémente `retries` (status `pending` si retry possible, sinon `dead` = DLQ logique).
5. Publie le résultat sur Redis pub/sub canal `ai_result:{job_id}` + stocke en clé avec TTL 600s (pour les bots qui se réveillent en retard).

## Dépendances externes

- PostgreSQL (table `ai_jobs`)
- Redis (pub/sub + SET)
- API interne (`/analyze`, `/analyze/image`)

## Modules clés

- `src/main.rs` — startup + redis_client + scheduler
- `src/config.rs` — `poll_interval_secs`, `job_timeout_secs`
- `src/scheduler.rs` — enregistre `drain_ai_jobs`
- `src/jobs/drain_ai_jobs.rs` — logique de claim + dispatch + persist

## Variables d'env

- `DATABASE_URL` / `REDIS_URL` / `API_URL` / `API_KEY`
- `AI_POLL_INTERVAL` (défaut 2s)
- `AI_JOB_TIMEOUT` (défaut 120s)

## Tables DB

- `ai_jobs` (UPDATE status/retries/result_payload)

## Note architecture

Le worker n'embarque **pas** les modèles ONNX : il délègue à l'API qui les a déjà chargées en mémoire. Simplification déploiement (pas de duplication de modèles ONNX dans chaque worker). Le gain est architectural : découple le temps de wait des bots (qui ont actuellement un timeout 5s sur `/analyze`) du temps de traitement réel.
