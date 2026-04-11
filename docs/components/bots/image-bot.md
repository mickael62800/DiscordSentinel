# image-bot

**Rôle** : Analyse les images envoyées pour détecter le contenu problématique (NSFW, hachage perceptuel) via une queue asynchrone.

## Commandes / Events Discord principaux

- Slash `/image stats` — statistiques (images analysées, taux de cache hit)
- Event `message` — extraction et analyse des images attachées

## API interne (Phase 7A)

**Statut : full gRPC avec gain perf significatif.**

**gRPC** (`ImagesService` sur `:50051`, défini dans `services/proto/proto/images.proto`) :
- `AnalyzeImage` — wrappe `AnalyzeImageUseCase`. **Gain majeur** : `bytes image_data` envoie l'image en **binaire pur**, plus besoin d'encoder en base64. Économie ~33% sur la taille du payload vs l'ancien JSON HTTP, et la sérialisation/désérialisation est plus rapide. Pour des images de plusieurs centaines de Ko (typique en Discord), c'est significatif sur des serveurs à fort trafic d'images.

**HTTP retenu** :
- `download_image(url)` reste sur le client `reqwest` brut — c'est un téléchargement d'attachment Discord externe (CDN), pas un appel à l'API Sentinel.

## Comportement si l'API tombe

- **`analyze_image`** : circuit breaker → `Err("API indisponible")`. Les images ne sont pas analysées pendant la panne.
- **Stratégie de dégradation côté queue** : la queue d'analyse fait jusqu'à **3 retries avec backoff 10s** entre chaque. Si tous les retries échouent, le bot **supprime préventivement le message** et envoie un embed expliquant que l'API est indisponible. Ce comportement est conservé tel quel : circuit breaker + retries de la queue se complètent.
- **Cache de hash** : les images déjà analysées (cache hit dans `image_hash::ImageHashCache`) ne nécessitent pas d'appel API → pas d'impact sur les images répétées (memes, etc.).

## Modules clés

- `src/analysis_queue.rs` — queue asynchrone pour ne pas bloquer les messages
- `src/image_hash.rs` — cache des hashes perceptuels (évite de réanalyser)
- `src/channel_thresholds.rs` — seuils d'action par canal
- `src/handler.rs` — traitement des messages + extraction attachments (plus d'encodage base64 depuis Phase 7A)
- `src/api_client.rs` — wrapper gRPC `ImagesService` avec circuit breaker

## Variables d'env

- `IMAGE_DISCORD_TOKEN`
- `API_BASE_URL`
- `GRPC_API_URL`
- `API_KEY`
- `MAX_IMAGE_SIZE` (taille max en bytes)

## Cache Serenity (Phase 1)

**Tier : `small`** — cache léger des channels.
