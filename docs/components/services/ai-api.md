# ai-api — API de training ML (Python FastAPI)

**Rôle** : API Python séparée pour entraîner et fine-tuner les modèles **text-sentiment** et **image-classification** utilisés par l'API Rust (inférence ONNX). Upload datasets, lance le training en thread async, export ONNX, notifie le backend pour reload.

**Tourne à la demande** — n'est pas dans le chemin critique de production. Peut être arrêtée sans impacter les bots/api.

## Architecture

Monolithe FastAPI. Stack : **Python 3.10+ / FastAPI / Uvicorn / PyTorch 2.2+ / Transformers 4.40+ / ONNX 1.16+ / scikit-learn / Pillow**.

## Structure du code

```
ai/
├── api/
│   ├── main.py              (FastAPI app + lifespan)
│   ├── constants.py         (ModelType enum, MAX_UPLOAD_BYTES, ALLOWED_ORIGINS)
│   ├── routes/
│   │   ├── datasets.py      (GET/POST/DELETE /api/ai/datasets/*)
│   │   ├── training.py      (POST /api/ai/training/start /status /stop)
│   │   └── export.py        (POST /api/ai/export/{model_type}, GET /api/ai/models)
│   ├── ml/
│   │   ├── text_dataset.py     (TextDataset loader JSONL)
│   │   └── vision_dataset.py   (VisionDataset loader images)
│   └── tests/
│
├── training/
│   ├── text/
│   │   ├── datasets/        (.jsonl : toxic comments labellisés)
│   │   ├── configs/train_config.yaml  (LR, epochs, batch, warmup)
│   │   └── exports/text_sentinel.onnx  (exported model)
│   └── vision/
│       ├── datasets/        (jpg/png/webp catégorisés)
│       ├── configs/train_config.yaml
│       └── exports/image_sentinel.onnx
│
├── Datasets/               (datasets bruts, pré-processing)
└── shared/                 (utilitaires Python partagés)
```

## Endpoints

| Route | Méthode | Description |
|---|---|---|
| `/health` | GET | Health check |
| `/api/ai/datasets` | GET | Liste des datasets disponibles |
| `/api/ai/datasets/{model_type}/upload` | POST | Upload d'un dataset (multipart) |
| `/api/ai/datasets/{model_type}` | DELETE | Supprimer un dataset |
| `/api/ai/training/start` | POST | Démarrer un training (body : `{model_type, hyperparams}`) |
| `/api/ai/training/status` | GET | État real-time (epoch, loss, accuracy, batch progress) |
| `/api/ai/training/stop` | POST | Arrêter le training en cours |
| `/api/ai/export/{model_type}` | POST | Exporter le modèle en ONNX |
| `/api/ai/models` | GET | Liste des modèles (checkpoints + exports) |

## Flux de training

```
Desktop ──POST /api/ai/datasets/text/upload─► ai-api (stocke JSONL)
Desktop ──POST /api/ai/training/start────────► ai-api (spawn thread)
         │
         │  Thread de training (PyTorch) :
         │  - charge le dataset
         │  - fine-tune BERT (texte) ou ResNet/MobileNet (vision)
         │  - update TrainingState thread-safe (epoch, loss, accuracy)
         │  - early stopping si pas d'amélioration
         │
Desktop ──GET /api/ai/training/status (polling)──► ai-api (retourne state)
         │
         │  Training terminé →
         │
Desktop ──POST /api/ai/export/text──► ai-api (export ONNX)
         │                              │
         │                              └─► POST http://api:3000/api/models/reload
         │                                    (api Rust reload les ONNX sessions)
         │
Desktop ──GET /api/ai/models─► liste des exports disponibles
```

## Dépendances externes

- **PyTorch 2.2+** (entraînement)
- **Transformers 4.40+** (BERT et variantes pour le texte)
- **TorchVision** (modèles vision pré-entraînés)
- **ONNX 1.16+** (export format)
- **ONNX Runtime 1.17+**
- **scikit-learn** (metrics d'évaluation)
- **Pillow** (prétraitement images)
- **Requests** (notification backend pour reload)

## Variables d'env

| Variable | Défaut | Rôle |
|---|---|---|
| `AI_API_PORT` | 8000 | Port Uvicorn |
| `API_BASE_URL` | `http://localhost:3000` | URL de l'API Rust pour reload models |
| `ALLOWED_ORIGINS` | — | CORS |

Les hyperparamètres (epochs, batch_size, learning_rate, warmup_steps, etc.) sont lus depuis les fichiers YAML `ai/training/{text,vision}/configs/train_config.yaml`.

## Observabilité

- Logging structuré (INFO) vers stdout
- **TrainingState** — état partagé thread-safe exposé via `/api/ai/training/status` (current_epoch, loss, accuracy, phase, progress)
- Notification du backend via `POST /api/models/reload` après export réussi
- Early stopping tracé avec `best_epoch` et `final_metrics`
- Pas de Prometheus ici (le backend Rust couvre l'observabilité prod)

## Intégration avec l'écosystème

1. Le **desktop** expose la page `IaTrainingPage.vue` qui consomme cette API
2. Les modèles exportés sont placés dans `ai/training/{text,vision}/exports/*.onnx`
3. Le **api (Rust)** charge ces ONNX au démarrage (`VISION_MODEL_PATH`, `TEXT_MODEL_PATH`) — volumes Docker mount
4. Après un nouvel export, l'api Rust peut être reload via `POST /api/models/reload` sans redémarrage
