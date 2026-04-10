# voice-bot

**Rôle** : Gère les salons vocaux temporaires, transfert d'ownership, vote-kick, AFK sweep, limite de membres et session cards de logs.

## Commandes / Events Discord principaux

- Slash `/voice setup`, `/voice transfer`, `/voice co-admin`, `/voice queue`, `/voice access-control`
- Event `voice_state_update` — création/destruction de salons temporaires
- Event `message` — commandes et votes dans les text-channels associés
- Background task — AFK sweep (60s) + nettoyage des salons vides

## Dépendances externes

- API interne (`voice_channels`, `voice_sessions`, `voice_channel_themes`, etc.)
- Discord Gateway + REST (critique : voice_states)

## Modules clés

- `src/handlers/voice/channel_lifecycle.rs` — création/destruction des salons temporaires
- `src/interactions/channel_management.rs` — limites, permissions, customisation
- `src/interactions/vote_kick.rs` — vote-kick avec majorité
- `src/session_card.rs` — carte de session vocale en logs (live updates)
- `src/state/afk_tracker.rs` — suivi AFK et kick automatique
- `src/tasks/afk_sweep.rs` — sweep périodique (60s)

## Variables d'env

- `VOICE_DISCORD_TOKEN`
- `API_BASE_URL`
- `API_KEY`
- `VOICE_GUILD_ID`
- `VOICE_PUBLIC_CREATOR_CHANNEL_ID`
- `VOICE_PRIVATE_CREATOR_CHANNEL_ID`
- `VOICE_LOG_CHANNEL_ID` (optionnel)

## Cache Serenity (Phase 1)

**Tier : `full`** — cache complet requis pour résoudre les `voice_states`.

## Note Phase 2 / 4

- Les colonnes `voice_channels.kind` et `voice_channels.channel_status` sont maintenant typées (enum Postgres `voice_channel_kind` + index partiel `idx_voice_channels_active WHERE channel_status='open'`).
- Le `voice-afk-worker` a été **différé** (sweep 100% in-memory difficile à extraire). Le sweep reste dans le bot pour l'instant.
