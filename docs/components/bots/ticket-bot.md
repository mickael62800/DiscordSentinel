# ticket-bot

**Rôle** : Gestion des tickets de support avec panel de création, escalade SLA, transcripts, satisfaction rating et FAQ.

## Commandes / Events Discord principaux

- Slash `/ticket panel` — déployer le panel de création
- Slash `/ticket close` — fermer le ticket du salon courant
- Slash `/ticket invite` — inviter un membre au ticket
- Event `interaction` (buttons) — création, fermeture, actions interactives
- Background task — escalade SLA (5 min) pour tickets sans réponse

## Dépendances externes

- API interne (`tickets`, `ticket_messages`, `ticket_assignments`)
- Discord Gateway + REST
- Redis (listener pour events temps-réel)

## Modules clés

- `src/sla.rs` — tracker d'escalade (SLA, timeouts tickets)
- `src/transcript.rs` — génération des transcripts
- `src/templates.rs` — templates de réponse prédéfinies
- `src/satisfaction.rs` — rating post-closure

## Variables d'env

- `TICKET_DISCORD_TOKEN`
- `API_BASE_URL`
- `API_KEY`
- `REDIS_URL` / `REDIS_CHANNEL`

## Cache Serenity (Phase 1)

**Tier : `small`** — cache channels pour panneaux.
