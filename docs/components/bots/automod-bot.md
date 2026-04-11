# automod-bot

**Rôle** : Détecte et modère automatiquement les messages spam, phishing, contenu offensant et incohérent via analyse temps-réel avec throttling adaptatif.

## Commandes / Events Discord principaux

- Slash `/automod status` — état du bot et compteurs (messages traités, détections)
- Slash `/automod test` — test d'analyse d'un message sur les patterns de détection
- Event `message` — analyse en temps réel (spam, liens, phishing, insultes, unicode)
- Throttling adaptatif automatique en cas d'inondation

## API interne (Phase 7A)

**Statut : full gRPC.** automod-bot est le **hot path le plus chaud du projet** — un appel API par message Discord reçu sur tous les serveurs. C'est ici que le gain perf gRPC est le plus visible.

**gRPC** (`AutomodService` sur `:50051`, défini dans `services/proto/proto/automod.proto`) :
- `AnalyzeMessage` — wrappe `AnalyzeMessageUseCase` côté API. Payload protobuf binaire (vs JSON HTTP), multiplexage HTTP/2 sur un Channel persistant. Gain typique de latence p99 : ~30-60% selon la taille du payload et le RTT réseau interne.

**HTTP retenu** : aucun appel métier. Le `BaseApiClient` reste injecté pour le heartbeat partagé (`spawn_heartbeat`).

## Comportement si l'API tombe

Le circuit breaker partagé (5 échecs / cooldown 10 s) court-circuite immédiatement les appels suivants. Pendant l'ouverture :

- `analyze_message` retourne `Err("API indisponible")` instantanément (pas de timeout, pas de hang du bot).
- **Stratégie de dégradation** : le bot **n'applique aucune action de modération** sur les messages reçus pendant la panne. Comportement par défaut = laisser passer (pas de faux positifs sur API down).
- Le timeout original de 5 s côté HTTP est remplacé par le timeout 30s du Channel tonic — le breaker court-circuite avant.
- Au retour de l'API : le breaker passe en half-open après 10 s, un message test passe, succès → referme.

## Modules clés

- `src/detectors/` — 5 patterns : `spam.rs`, `phishing.rs`, `link.rs`, `insult.rs`, `unicode.rs`
- `src/adaptive_slowmode.rs` — slowmode adaptatif par canal
- `src/handler.rs` — dispatch messages → détecteurs → appel gRPC `analyze_message`
- `src/api_client.rs` — wrapper gRPC `AutomodService` avec circuit breaker

## Variables d'env

- `AUTOMOD_DISCORD_TOKEN`
- `API_BASE_URL` (HTTP, conservé pour `BaseApiClient` heartbeat)
- `GRPC_API_URL` (gRPC interne — défaut `http://127.0.0.1:50051`)
- `API_KEY`

## Cache Serenity (Phase 1)

**Tier : `small`** — cache des channels pour logs contextuels.
