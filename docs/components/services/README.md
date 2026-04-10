# Services backend — Index

3 services serveur : API principale, Gateway WebSocket, AI training API Python.

| Service | Stack | Port |
|---|---|---|
| [api](./api.md) | Rust / Axum / PostgreSQL / Redis / ONNX | 3000 |
| [gateway](./gateway.md) | Rust / Axum WebSocket / Redis | 3001 |
| [ai-api](./ai-api.md) | Python / FastAPI / PyTorch / ONNX | 8000 |

## Rôles et dépendances

```
Desktop ──HTTP──► api (auth + CRUD + inference)
   │                │
   │                ▼
   │           PostgreSQL + Redis
   │                │
   └──WebSocket─► gateway ──SUBSCRIBE─► Redis pub/sub
                                           ▲
                                           │
                      Bots + Workers ──PUBLISH──┘

Desktop ──HTTP──► ai-api (training ML async)
                      │
                      ▼
                 PyTorch + ONNX export
                      │
                      ▼  POST /api/models/reload
                    api (reload des ONNX sessions)
```

- **api** : le cerveau. Persiste tout, expose ~34 familles de routes HTTP, fait l'inférence texte/image ONNX synchrone (le `ai-worker` de Phase 4 permet désormais le async via `POST /api/ai/jobs`).
- **gateway** : ultra-fin, stateless. Son seul rôle = pont Redis pub/sub ↔ WebSocket pour permettre aux clients desktop de recevoir des events temps-réel sans poll.
- **ai-api** : entraînement ML (fine-tuning). Tourne à la demande, expose upload datasets + start/stop training + export ONNX. N'est pas dans le chemin critique de production — peut être arrêté sans impacter les bots/api.
