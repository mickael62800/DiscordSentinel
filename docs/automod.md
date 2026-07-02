# AutoMod — Architecture & fonctionnement complet

> DiscordSentinel — module de modération automatique.
> Règle d'or : **le bot DÉTECTE et EXÉCUTE, l'API/core DÉCIDE.**

## 1. Vue d'ensemble

L'automod est réparti sur 3 services (architecture hexagonale) :

| Service | Rôle |
|---------|------|
| **sentinel-bot** (Rust/Serenity) | Reçoit les messages Discord, détection locale rapide, exécute les sanctions, poste les cartes. |
| **sentinel-api + sentinel-core** | Cerveau : scoring, routage (que faire ?), inférence IA texte/vision, config serveur, persistance. « DECIDE = API ». |
| **sentinel-web** (Vue 3) | Dashboard : config des règles/seuils, consultation des détections, résolution des cartes depuis le web. |

Communication bot → API : **gRPC** (`AutomodService.AnalyzeMessage` = hot path le plus chaud du projet, 1 appel/message). Circuit breaker (5 échecs/10s) : si l'API tombe, le bot **laisse passer** (pas de faux positifs sur une API down), sauf cas de fallback règles-simples.

Fichiers clés :
- Bot : `sentinel-bot/src/modules/automod/` — `mod.rs`, `message_handler.rs`, `backend.rs`, `api_client.rs`, `review.rs`, `detectors/`, `vote/`
- Core : `sentinel-core/src/domain/services/moderation/` — `scoring_service.rs`, `automod_routing.rs` ; `sentinel-core/src/application/ai/analyze_message_service.rs` ; `sentinel-core/src/application/moderation/manage_automod_reviews_service.rs`
- Web : `sentinel-web/src/composables/useAutomod.ts`, `components/pages/AutomodPage.vue`, `organisms/AutomodStatsGrid.vue`

## 2. Le parcours d'un message (`message_handler.rs::process`)

Ordre exact du pipeline sur chaque message :

1. **Déduplication** — `DashMap<MessageId, Instant>`, purge > 2000 entrées / 5 min.
2. **Config serveur** (`guild_config_or_default`, module `automod-bot`). `enabled=false` → stop.
3. **Exclusions** — `ignored_channels`, `ignored_roles`. **Mode nuit** (`night_mode`, ex. 22h–8h) durcit les détecteurs.
4. **Pièces jointes dangereuses** — extensions `exe/bat/cmd/scr/ps1/vbs/js/jar/...` → suppression ou carte de review (`files_review_mode`).
5. **Flood** — tracker local `(channel, user) → [timestamps]`. Au-delà du seuil, le bot demande à l'API via gRPC `evaluate_flood` si c'est **severe** (raid). Severe → auto-protection (mute + suppression) immédiate, même en `human_only`. Fallback seuil local si API KO.
6. **Caps** (majuscules) → warn ou carte (`caps_review_mode`).
7. **Slowmode adaptatif** — active un slowmode temporaire si un salon s'emballe ; désactivé en tâche de fond quand l'activité retombe.
8. **Détection locale** `detectors::analyze()` → 4 flags : `spam`, `insult`, `link`, `phishing`.
9. Si un flag est levé **OU** si l'IA texte est activée (`text_enabled`) → `send_to_backend()` (tâche async). Idem `analyze_message_images()` si vision activée.

## 3. Les détecteurs locaux (`detectors/`)

Fonction pure `analyze(content, config) -> DetectionFlags`. Chaque détecteur skippé si désactivé.

- **spam** (`spam.rs`) : répétition de caractères (défaut seuil 6), de mots (5), + sous-détecteurs regroupés dans le flag `spam` :
  - emoji spam (max 10), mentions massives (max 5), abus unicode (`unicode.rs` : zalgo/combining ≥3, invisibles ≥5, **homoglyphes** cyrilliques).
- **insult** (`insult.rs`) : liste intégrée + `insult_custom_words` par serveur.
- **link** (`link.rs`) : URLs ; `allow_discord_invites`, `allowed_domains` (whitelist).
- **phishing** (`phishing.rs`) : patterns scam (« free nitro », domaines typosquattés type `dlscord.gift`), `phishing_extra_whitelist`.

Config complète : `DetectorConfig` (voir `detectors/mod.rs`), construite depuis la config serveur.

## 4. Scoring (`scoring_service.rs`, service PUR)

Transforme flags → score numérique → action.

**Poids par défaut :** spam 3, insult 5, link 1, phishing 7 ; vision : nsfw 8, illicit 9 ; sentiment IA : anger 3, rage 6, threat 8, harassment 7.

**Score = somme des poids** des flags déclenchés.

**Seuils par défaut → action :** ≥9 Ban · ≥6 Mute · ≥4 Delete · ≥2 Warn · sinon rien.

**Règles serveur** (`Rule` en DB, éditables au dashboard) surchargent poids ET seuils. Finesse `resolve_thresholds` : seules les règles dont le flag a **réellement** été déclenché comptent (une règle stricte sur les liens n'abaisse plus le seuil des insultes).

## 5. Inférence IA (`analyze_message_service.rs`)

Appelé côté API dans `analyze()`. Étapes :

1. Charge les règles (cache Redis → DB) + la config `automod-bot` (poids/seuils, clés IA).
2. Scoring de base (flags bot).
3. **IA texte (sentiment)** si `text_enabled` + modèle dispo + contenu non vide :
   - Rate-limité (`InferenceRateLimiter`), timeout 5s (`spawn_blocking`).
   - Contexte conversationnel (N messages avant) injecté (`context_format` : `natural` / `tagged`).
   - Classes : modèle 2-classes `severe` → `Harassment` ; legacy 5-classes anger/rage/threat/harassment.
   - **Atténuation contextuelle** (`context_dampening`, défaut 0.65) : réduit le score IA si du contexte existe (moins de faux positifs sur les blagues).
   - Score combiné = score bot + score IA ; action recalculée sur seuils per-flag-type.
   - **Garde-fou C5** (`cap_ia_induced_ban`) : l'IA seule ne peut PAS provoquer un Ban auto sur un 1er message → plafonné à Mute (le Ban reste via escalade de strikes ou décision humaine).
4. **Tension de salon** (optionnel, `channel_tension_*`) : somme glissante des scores IA des N derniers messages du salon ; si plus sévère que l'analyse individuelle, override. Buffer vidé après déclenchement.
5. **Décision de routage** (voir §6).
6. Persiste l'`Infraction` en DB, retourne `MessageAnalysis { action, reason, score, duration, route, severe, auto_delete_link }`.

**Vision (images)** — `backend.rs::analyze_message_images` : télécharge l'image (plafond `vision_max_image_size_mb`, défaut 14 Mo, lecture bornée chunk par chunk), soumet un job async `POST /api/ai/jobs` (type `analyze_image`), attend le résultat via Redis (poll 1s, timeout 30s). Overrides `vision_auto_delete_nsfw` / `vision_auto_delete_illicit`. **Fail-safe** : si la vision est indisponible → carte de revue manuelle (jamais silencieux).

## 6. Routage / décision (`automod_routing.rs`, fonction PURE `decide`)

3 issues possibles (`Routing`) : **`Auto`** (bot applique direct), **`Card`** (carte de review humaine), **`None`** (rien).

Règles :
- **phishing** ou **invitation Discord** (`discord.gg/`) = **severe** → auto-protection immédiate même en `human_only`.
- **Lien générique** (hors image, hors phishing) → carte par défaut ; suppression sèche seulement si `auto_delete_links` activé (opt-in agressif).
- `human_only` → toujours carte (aucune sanction auto).
- IA « review mode » (`ai_review_mode`) → carte si score ≥ `review_min_score`.
- Carte possible seulement si un salon de review est configuré (`log_channel_id != 0`).

## 7. Exécution des sanctions (`backend.rs`)

- **`apply_auto_protect`** (cas severe) : mute (timeout Discord, clampé 60s–28j) + suppression + **trace** dans l'historique modération (acteur = le bot, compte dans l'escalade) + carte de sanction + **DM DSA au membre** (motif + `/appeal`). Retourne `(note, sanction_logged)` — `sanction_logged` évite le double-strike à la finalisation.
- **`execute_action`** (mode Auto) : Warn (embed), Delete (embed + suppression), Mute (timeout + suppression), Ban (**proposition** — embed, ban réel non exécuté en auto). `sanction_appeal_enabled` ajoute la mention du droit d'appel.
- **Fallback backend injoignable** : en `human_only` → rien ; sinon suppression simple des messages flaggés (phishing/insulte/spam/lien).

## 8. Cartes de review & votes (`review.rs`, `vote/`)

Deux modes selon la config :

### Review 1-clic (`review.rs`)
Carte embed dans le salon de logs avec boutons `am_{w|d|m|b|i}` (Appliquer / Warn / Delete / Mute / Ignorer). Le modérateur clique → action exécutée + carte mise à jour.
- Custom_id : `am_{action}:{guild}:{channel}:{message}:{user}`.
- Permissions : `MODERATE_MEMBERS` | `MANAGE_MESSAGES` | `ADMINISTRATOR`.
- **Idempotence** : `claim_once("card:{msg.id}")` — une seule action par carte (anti double-clic).
- La review est aussi créée en DB (`POST /api/automod/reviews`) → mapping `discord_action_messages` pour la sync web.

### Mode vote (`vote/`) — si `vote_enabled` ou forcé par `human_only`
Carte de vote (`post_vote_card`) avec boutons de vote des modérateurs, deadline (`vote_deadline_hours`, défaut 72h), fil de discussion optionnel (`vote_thread_enabled`), agrégation optionnelle (`vote_aggregate_enabled` : fusionne les incidents récurrents d'un même user dans une carte, escalade vers l'action la plus sévère).
- Vote : `cast_vote` (core) — seul un modérateur vote, upsert (1 vote/personne), **conflit d'intérêt** interdit (on ne vote pas sur sa propre détection).
- Décision : `tally_votes(votes, quorum, tie_action)` → action majoritaire.
- **Finalisation** (`finalize.rs`, bouton `amf:<id>`) : réservée admin (`can_finalize_review`), statut doit être `decided`, persiste via `POST /reviews/{id}/resolve` (source `discord`), exécute la sanction Discord (`apply_member_sanction`), DM au membre, archive la discussion, édite la carte.
- Carte manuelle (`/card`, `post_manual_vote_card`) : même flux, contexte avant **et** après, score 0.

### Résolution depuis le web (`review.rs::handle_redis_event`)
L'API publie `automod_review_resolved` sur Redis Stream. Le bot (consumer group) : grise la carte Discord (footer « via web par X ») + applique la sanction (`apply_web_resolution`). Idempotent (`claim_once("webres:{id}")`, anti-redelivrance). Skip si `actor.source != "web"` (anti-boucle).

### Cycle de vie d'une review (`manage_automod_reviews_service.rs`)
Statuts : `pending` → `voting` → `decided` → `applied` | `ignored` (+ `reopen` possible). Résolution idempotente (2e resolve = `Conflict` via `UPDATE WHERE status='pending'`). Cartes expirées nettoyées (`expire_review_cards`).

## 9. Traçabilité & escalade

Toute sanction (auto-protect, review 1-clic, finalisation de vote) est journalisée dans le module **moderation** via gRPC `log_action` (`log_sanction_to_moderation`) → compte dans l'historique + escalade de strikes, au même titre qu'un `/warn` manuel. `prevention` = tracée mais **hors** escalade. Anti double-strike : `already_sanctioned` propagé de l'auto-protect jusqu'à la finalisation.

## 10. Configuration serveur (module `automod-bot`)

Toutes les clés sont éditables au dashboard (`AutomodPage.vue`). Principales :

- **Général** : `enabled`, `log_channel_id`, `ignored_channels`, `ignored_roles`, `mute_duration_secs`.
- **Modes** : `human_only_enabled`, `auto_protect_enabled`, `auto_delete_links_enabled`, `ai_review_mode`, `review_min_score`, `*_review_mode` (caps/flood/files).
- **Flood** : `flood_max_messages`, `flood_window_secs`, `severe_flood_max_messages`.
- **Mode nuit** : `night_mode_enabled`, `night_start_hour`, `night_end_hour`.
- **Scoring** : `score_weight_*` (10 flags), `score_threshold_{warn,delete,mute,ban}`.
- **IA texte** : `text_enabled`, `text_threshold`, `context_dampening`, `context_format`, `context_max_messages`, `context_max_chars`.
- **Tension salon** : `channel_tension_enabled`, `channel_tension_buffer_size`, `channel_tension_threshold_*`, `channel_tension_mute_duration_secs`.
- **Vision** : `vision_enabled`, `vision_queue_enabled`, `vision_max_image_size_mb`, `vision_scan_embeds`, `vision_queue_max_retries`, `vision_auto_delete_nsfw`, `vision_auto_delete_illicit`.
- **Vote** : `vote_enabled`, `vote_deadline_hours`, `vote_context_before`, `vote_thread_enabled`, `vote_aggregate_enabled`, `vote_aggregate_window_minutes`, `vote_admin_role_id`, `discussion_channel_enabled`.
- **Conformité** : `auto_protect_notify_member`, `sanction_appeal_enabled`.

## 11. Dashboard web (consultation)

`useAutomod.ts` (singleton) charge les 100 dernières détections (`GET` via `automodService.listDetections`), agrège par catégorie (parsing du champ `reason`) et par top-utilisateurs (récidivistes). Rafraîchi **en live** via WebSocket sur l'event `moderation_detection`. Affiché par `AutomodStatsGrid.vue` (KPIs + listes). Section « Qualité des détections » (`AutomodFalsePositives.vue`) : taux de faux positifs (sur-blocage) via `GET /api/automod/{guild}/fp-stats`.

## 12. Limites connues (CR 2026-07-02)

1. **Seuils figés au démarrage (workers)** — les réglages worker sont chargés au boot ; un seuil worker changé au dashboard ne s'applique qu'au **redémarrage** (badge « redémarrage requis » ajouté côté web). Les seuils des **modules** type automod sont, eux, lus en direct à chaque message.
2. **Taux de faux positifs** — désormais mesuré (endpoint `fp-stats` + section web). L'IA généraliste reste bornée en warn/timeout court, jamais ban autonome.
3. **Durcissement comptes < 7 jours** ciblé liens+mentions, à spécifier.
4. Conformité DSA : action auto ⇒ toujours motif + voie d'appel (`/appeal`).

## 13. Diagramme de séquence — cycle complet d'un message

```
Membre        sentinel-bot                    sentinel-api / core            Salon de review        Modérateur/Web
  |                |                                  |                              |                    |
  |--- message --->|                                  |                              |                    |
  |                |-- dédup + config guild           |                              |                    |
  |                |-- exclusions (salons/rôles/nuit) |                              |                    |
  |                |                                  |                              |                    |
  |                |== FLOOD ? =====================> gRPC evaluate_flood            |                    |
  |                |                                  |-- severe ? (config)          |                    |
  |                |<===== (severe, mute_secs) =======|                              |                    |
  |                |-- si severe: mute+delete+DSA DM  |                              |                    |
  |                |                                  |                              |                    |
  |                |-- detectors::analyze()           |                              |                    |
  |                |   -> flags {spam,insult,link,phishing}                          |                    |
  |                |                                  |                              |                    |
  |                |== ANALYZE (hot path) ==========> gRPC AnalyzeMessage            |                    |
  |                |   (flags + contenu + contexte)   |-- rules (cache->DB)          |                    |
  |                |                                  |-- scoring (poids+seuils)     |                    |
  |                |                                  |-- IA texte (sentiment, 5s)   |                    |
  |                |                                  |   + context_dampening + C5   |                    |
  |                |                                  |-- tension de salon           |                    |
  |                |                                  |-- decide() -> route          |                    |
  |                |                                  |-- persist Infraction         |                    |
  |                |<== {action,score,route,severe,auto_delete_link} ==|            |                    |
  |                |                                  |                              |                    |
  |     +----------+----------+                       |                              |                    |
  |     | route = Auto        |-- execute_action (warn/delete/mute/ban-proposal)    |                    |
  |     | route = None        |-- rien (ou notice auto-mute si severe)              |                    |
  |     | route = Card        |----- POST /api/automod/reviews --> (DB)             |                    |
  |     +---------------------+                       |------ carte embed + boutons ->|                    |
  |                                                    |                              |<-- clic vote/1clic-|
  |                                                    |<-- cast_vote / resolve ------|                    |
  |                                                    |-- tally_votes -> decided     |                    |
  |                                                    |<-- finalize (admin) ---------|                    |
  |                |<-- Redis: automod_review_resolved-|                              |                    |
  |                |-- apply_member_sanction + edit carte (grisée) + DM /appeal      |                    |
  |                |-- log_action (historique + escalade strikes)                    |                    |
```

Note vision (images) : chemin async parallèle — le bot soumet `POST /api/ai/jobs` (type `analyze_image`), l'ai-worker traite, résultat récupéré via Redis (`ai_result:{job_id}`, poll 1s / timeout 30s). Fail-safe → carte de revue manuelle.

## 14. Schéma de base de données

### `automod_reviews` (migration 176, étendue par 251/264/269/295…)
Carte de review/vote persistée — pivot de la sync bot ↔ web.

| Colonne | Type | Notes |
|---------|------|-------|
| `id` | UUID PK | référencé partout comme `action_id` |
| `guild_id`, `channel_id`, `message_id` | TEXT | message Discord ciblé |
| `user_id`, `user_name` | TEXT | membre visé |
| `content_preview` | TEXT | extrait du message (sanitizé) |
| `suggested_action` | TEXT CHECK | `warn\|delete\|mute\|ban` (IA ne suggère pas `prevention`) |
| `score` | DOUBLE | score de scoring |
| `reason` | TEXT | motif combiné (bot + IA + tension) |
| `flags` | JSONB | `{spam,insult,link,phishing}` |
| `status` | TEXT CHECK | `pending\|voting\|decided\|applied\|ignored` |
| `voting_deadline` | TIMESTAMPTZ | échéance du vote (index partiel `WHERE status='voting'`) |
| `decided_action` | TEXT CHECK | verdict calculé (`+prevention`) |
| `quorum_met` | BOOL | quorum atteint ? |
| `applied_action` | TEXT CHECK | action réellement appliquée (`+prevention`, `+ignore`) |
| `resolved_by_id/name`, `resolved_source` | TEXT | `discord\|web` |
| `incident_count` | INT | agrégation (264) |
| `already_sanctioned` | BOOL | anti double-strike (295) |
| `created_at`, `decided_at`, `resolved_at` | TIMESTAMPTZ | |

Cycle de vie : `voting → decided → applied\|ignored` (`pending` = legacy avant refonte vote). Reopen possible. Index : `(guild_id, status, created_at DESC)`, `(user_id)`, `(voting_deadline) WHERE status='voting'`.

### `automod_review_votes` (migration 251)
Un vote par (review, modérateur), upsert pour changer d'avis.

| Colonne | Type | Notes |
|---------|------|-------|
| `id` | UUID PK | |
| `review_id` | UUID FK → automod_reviews | `ON DELETE CASCADE` |
| `voter_id`, `voter_name` | TEXT | |
| `vote_action` | TEXT CHECK | `prevention\|warn\|delete\|mute\|ban\|ignore` |
| `created_at`, `updated_at` | TIMESTAMPTZ | |
| | | `UNIQUE (review_id, voter_id)` |

### `automod_discussion_channels` (migration 266)
Salon Discord de débat lié à une review (bouton « Ouvrir une discussion »).

| Colonne | Type | Notes |
|---------|------|-------|
| `id` | UUID PK | |
| `review_id` | UUID FK → automod_reviews | `UNIQUE` (1 salon/review), `ON DELETE CASCADE` |
| `guild_id`, `channel_id` | TEXT | |
| `opened_by_id/name` | TEXT | |

Messages archivés dans `automod_discussion_messages` (migration 279) au moment de la clôture (snapshot transcript).

### `ai_dataset_messages` (migration 243) — module dataset IA (distinct)
Collecte de messages pour entraîner un modèle. Toggle par guild via `bot_guild_config` (`bot_name='ai-dataset-bot'`, `config_key='enabled'`, défaut OFF).

| Colonne | Type |
|---------|------|
| `id` | UUID PK |
| `guild_id` | TEXT NOT NULL |
| `channel_id`, `channel_name` | TEXT |
| `user_id` | TEXT NOT NULL |
| `content` | TEXT NOT NULL |
| `created_at` | TIMESTAMPTZ |

Index : `(guild_id, created_at DESC)`, `(guild_id, user_id)`.

### Autres tables liées
- `infractions` — chaque analyse automod y est persistée (`AnalyzeMessageService` → `infraction_repo.save`). Source des « détections » affichées au dashboard.
- `discord_action_messages` — mapping `action_id ↔ (channel, message)` Discord, permet au web de retrouver/éditer la carte.
- `bot_guild_config` (`bot_name='automod-bot'`) — toutes les clés de config du §10 ; schéma déclaré dans `bot_definitions.config_schema`.
- Sanctions tracées dans le module **moderation** (historique + escalade de strikes) via gRPC `log_action`.

---

> Doc générée lors de la session de revue automod (2026-07-02). Voir aussi le CR `cr/2026-07-02_automod-usage.md`.
