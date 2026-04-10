# image-bot

**Rôle** : Analyse les images envoyées pour détecter le contenu problématique (NSFW, hachage perceptuel) via une queue asynchrone.

## Commandes / Events Discord principaux

- Slash `/image stats` — statistiques (images analysées, taux de cache hit)
- Event `message` — extraction et analyse des images attachées

## Dépendances externes

- API interne — appel `/analyze/image` (synchrone ONNX vision)
- Discord Gateway

## Modules clés

- `src/analysis_queue.rs` — queue asynchrone pour ne pas bloquer les messages
- `src/image_hash.rs` — cache des hashes perceptuels (évite de réanalyser)
- `src/channel_thresholds.rs` — seuils d'action par canal
- `src/handler.rs` — traitement des messages + extraction attachments

## Variables d'env

- `IMAGE_DISCORD_TOKEN`
- `API_BASE_URL`
- `API_KEY`
- `MAX_IMAGE_SIZE` (taille max en bytes)

## Cache Serenity (Phase 1)

**Tier : `small`** — cache léger des channels.

## Note Phase 4

Même remarque que `automod-bot` : peut migrer vers `POST /api/ai/jobs` avec `job_type=analyze_image` pour supprimer tout blocage côté gateway.
