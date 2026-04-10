# automod-bot

**Rôle** : Détecte et modère automatiquement les messages spam, phishing, contenu offensant et incohérent via analyse temps-réel avec throttling adaptatif.

## Commandes / Events Discord principaux

- Slash `/automod status` — état du bot et compteurs (messages traités, détections)
- Slash `/automod test` — test d'analyse d'un message sur les patterns de détection
- Event `message` — analyse en temps réel (spam, liens, phishing, insultes, unicode)
- Throttling adaptatif automatique en cas d'inondation

## Dépendances externes

- API interne via `BaseApiClient` — appels `/analyze` (synchrone, timeout 5s)
- Discord Gateway

## Modules clés

- `src/detectors/` — 5 patterns : `spam.rs`, `phishing.rs`, `link.rs`, `insult.rs`, `unicode.rs`
- `src/adaptive_slowmode.rs` — slowmode adaptatif par canal
- `src/handler.rs` — dispatch messages → détecteurs → appel API

## Variables d'env

- `AUTOMOD_DISCORD_TOKEN`
- `API_BASE_URL`
- `API_KEY`

## Cache Serenity (Phase 1)

**Tier : `small`** — cache des channels pour logs contextuels.

## Note Phase 4

Pourrait opter pour `POST /api/ai/jobs` (async queue) au lieu de `/analyze` synchrone pour supprimer le timeout 5s. Non migré à date — queue prête côté backend.
