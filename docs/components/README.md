# Documentation des composants — DiscordSentinel

Cette arborescence documente chaque composant du monorepo. Un fichier Markdown par composant, regroupés par catégorie.

## 📂 Structure

| Dossier | Contenu | Nombre |
|---|---|---|
| [`bots/`](./bots/) | 15 bots Discord + librairie `shared` | 16 |
| [`workers/`](./workers/) | 8 workers périodiques + librairie `worker-common` | 9 |
| [`services/`](./services/) | API backend, Gateway WebSocket, AI training API | 3 |
| [`apps/`](./apps/) | Application desktop Tauri (Rust + Vue 3) | 1 |

**Total : 29 composants documentés.**

## 🗺️ Architecture globale

```
┌────────────────────────────────────────────────────────────────────┐
│                    15 Bots Discord (Serenity)                       │
│  audit automod blackjack cleanup community coude game image         │
│  moderation progression roles security ticket voice welcome         │
└────────────┬──────────────────────────────────┬────────────────────┘
             │ HTTP (BaseApiClient)             │ Redis pub/sub
             ▼                                  ▼
┌────────────────────────┐         ┌────────────────────────────┐
│  API backend (Axum)    │◄────────┤  Gateway WebSocket         │
│  services/api          │         │  (relay Redis → desktop)   │
│  - ~105 migrations     │         └──────────┬─────────────────┘
│  - ONNX inférence      │                    │
│  - 34 handlers HTTP    │                    │
│  - Architecture hex.   │                    │
└────┬──────────────┬────┘                    │
     │              │                         │
     │ PostgreSQL   │ Redis                   │
     ▼              ▼                         ▼
┌───────────┐  ┌───────────┐       ┌──────────────────────┐
│ Postgres  │  │  Redis    │       │  Desktop Tauri       │
│ (PgBouncer)│  │  (cache + │       │  Vue 3 + Pinia      │
│ + 8 MV/   │  │   pub/sub)│       │  OAuth2 Discord      │
│ partitions│  │           │       └──────────────────────┘
└─────┬─────┘  └─────┬─────┘
      │              │
      │    ┌─────────┴──────────────────────────────┐
      │    │                                         │
      ▼    ▼                                         ▼
┌────────────────────────────────────┐      ┌──────────────────┐
│  8 Workers periodiques (Tokio)     │      │  AI Training API │
│  ai analytics cache cleanup        │      │  (Python FastAPI)│
│  coude moderation monitoring       │      │  PyTorch + ONNX  │
│  temp-roles                        │      │  fine-tuning     │
└────────────────────────────────────┘      └──────────────────┘
```

## 📖 Documents transverses

- [`../ROADMAP.md`](../ROADMAP.md) — Roadmap 7 phases avec état d'avancement
- [`../PHASES_0_2_DIFFERES.md`](../PHASES_0_2_DIFFERES.md) — Ce qui a été différé pendant les phases 0-2
- [`../BASELINE_METRICS.md`](../BASELINE_METRICS.md) — Template baseline + requêtes PromQL/SQL
- [`../DB_OPTIMISATIONS.md`](../DB_OPTIMISATIONS.md) — 12 optimisations schéma Postgres
- [`../WORKERS_PROPOSES.md`](../WORKERS_PROPOSES.md) — Workers proposés (dont déjà créés)
- [`../OPTIMISATIONS_PERFORMANCES.md`](../OPTIMISATIONS_PERFORMANCES.md) — 12 optimisations perf/scalabilité
