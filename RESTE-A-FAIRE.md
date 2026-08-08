# Reste à faire

État au 2026-08-07. Ce fichier ne liste que ce qui est **connu et non fait** — pas les idées ni les envies.

---

## 1. Bloquant : `nexus-core` ne compile plus en mode test

```
cargo check -p nexus-core --all-targets   # échoue
```

Les doubles de `nexus-core/src/application/game/worker_jobs.rs` (tests) ont divergé de leurs traits. Une vingtaine d'erreurs, toutes de la même famille :

| Trait | Symptôme |
|---|---|
| `GameServerRepository` | 10 méthodes manquantes (`list_active`, `update_runtime`, `try_transition_status`, `soft_delete`, …) + 7 méthodes du mock qui n'existent plus dans le trait |
| `ContainerRuntime` | 8 méthodes manquantes ; `remove_image` a gagné un paramètre |
| `PortAllocator` | `allocate` prend un `PortKind`, plus un `&str` ; `release` a perdu un paramètre ; `is_available` manquante |
| `PlayerSessionRepository` | `open`/`close`/`close_all_active` manquantes |
| `GameAuditLog` | `log` : lifetimes divergents |

Même famille que le `execute_spin_transaction` ajouté sur `WheelRepository` : le trait a évolué, le mock non. **Le code de production compile** — seuls les tests sont cassés. À corriger avant de pouvoir lancer la suite complète.

## 2. La suite de tests n'a jamais été exécutée

Sur toute la refonte (AppState par domaine, crates `platform-common*`, suppression des `#[allow(dead_code)]`, migration gRPC), la validation s'est faite à `cargo check` + `cargo clippy --all-targets`, jamais à `cargo test` — les builds sont longs, et c'est la consigne du projet.

Les 7 crates Sentinel sont propres à clippy, tous targets compris. Mais **rien ne garantit que les assertions passent**. Un premier `cargo test --workspace` est à prévoir, une fois le point 1 réglé.

---

## 3. Migration gRPC : ~12 appels HTTP restants dans le bot

> Note : le module `rotation` (administrateur tournant, 5 appels) n'a **pas** été migré mais **entièrement supprimé** (bot + API + core + web + tables `admin_rotation*`, migration `018`).


Les modules qui ont déjà un service proto sont terminés. Ce qui reste appartient à des modules **sans proto du tout** — chacun demande son `.proto`, son handler serveur, son câblage, ses tests.

| Module | Appels | Remarque |
|---|---|---|
| ~~`confessions`~~ | ~~11~~ ✅ | **FAIT** — `ConfessionsService` (proto `confessions`, handler `grpc/community/confessions.rs`) : create/get/list/delete/message-refs/reply/reply-msg-id/report/config (9 RPC). Le `DELETE` reqwest brut (`.client()`/`.base_url()`/`.auth()`) est éliminé. Reste 1 `POST /api/bots/config` (`persist_confession_setting`) — config transverse, voir « laissé en HTTP ». |
| `automod/vote` + `automod/review` | ✅ | **FAIT** via `AutomodReviewService` (proto `automod_review`, handler `grpc/moderation/reviews.rs`) : review-core (create/get/find-by-message/resolve/vote/ignore/reopen/list-votes) **+ discussions** (get/open/delete/append-messages). Plus aucun appel HTTP du cycle review. (Reste hors périmètre : `backend.rs` soumet les jobs vision à l'ai-worker via `POST /api/ai/jobs`.) |
| ~~`bump`~~ | ~~4~~ ✅ | **FAIT** — `BumpService` (proto `bump`, handler `grpc/community/bump.rs`) : RecordBump / DueReminders / MarkReminderSent / GuildStatus. `ready_at` calculé server-side. |
| ~~`moderation/ban_sursis`~~ | ~~4~~ ✅ | **FAIT** — `SursisService` (proto `sursis`, handler `grpc/moderation/sursis.rs`) : Create / Get / Resolve. Le délai d'appel reste lu server-side dans la config guild. |
| ~~`security` (5 fichiers)~~ | ~~6~~ ✅ | **FAIT** — nouveau `SecurityStateService` (proto `security_state`, handler `grpc/system/security_state.rs`). Quarantine/slowmode/lockdown migrés. |
| ~~`announcements`~~ | ~~2~~ ✅ | **FAIT** — `AnnouncementsService` (proto `announcements`, handler `grpc/community/announcements.rs`) : RecordRunResult + RecordButtonClick. |
| `welcome/handler` | 2 | |
| `handler.rs`, `sync.rs`, `logs_setup.rs` | 4 | Hors module |
| divers (`audit/handlers/message`, `embeds`, `tickets`, `voice`, `moderation/appeal`) | 5 | 1 appel chacun |

Estimation : ~6 nouveaux services proto restants (+ tranche 2 automod : discussions/discord-messages sur services existants).

> **À nettoyer (suite security)** : les endpoints HTTP `POST/GET /api/security/quarantine`,
> `/api/security/quarantine/active`, `DELETE .../{g}/{u}`, `POST /api/security/{slowmode,lockdown}`
> ne sont plus appelés par le bot. Vérifier qu'aucun autre consommateur (worker) ne les utilise
> avant de retirer routes + handlers. Les `DELETE .../slowmode/{g}` et `.../lockdown/{g}` restent
> potentiellement utilisés côté worker/consumers — ne pas supprimer sans audit.

### Ressource de sync transverse `discord-messages` — ✅ FAIT

Table `discord_action_messages` (mapping `action_id ↔ (channel_id, message_id)`),
migrée comme **service dédié** `DiscordActionMessagesService` (proto `discord_messages`,
handler `grpc/audit/action_messages.rs`, `Register` + `ListForAction`). Le helper partagé
`crate::sync::{register_action_message, list_action_messages}` passe désormais par gRPC ;
tous les appelants (automod, tickets, voice) sont migrés. **Plus aucun appel HTTP
`discord-messages` dans le bot.**

### Volontairement laissé en HTTP

`moderation::set_bot_config` — écriture d'une clé de config bot. C'est de la **config transverse**, pas de la modération : même nature que `get_guild_config_for`, que tous les modules appellent. Les deux mériteraient un service partagé (`BotConfigService`) plutôt qu'être greffés sur `ModerationService`. C'est le seul appel HTTP qui reste dans `moderation/api_client.rs`.

---

## 4. Fonctionnalités annoncées dans l'interface, jamais implémentées

Découvertes en retirant les `#[allow(dead_code)]` — c'est exactement ce qu'ils masquaient. **Décisions produit** : implémenter ou retirer de l'UI. Laisser en l'état est le pire des trois, parce que l'interface promet quelque chose qui n'arrive pas.

### `guild-backup-bot`

- **« Sauvegarde automatique »** + son intervalle : activables dans l'interface, jamais exécutés. Aucun job ne les lit.
- **« Rôles autorisés à restaurer »** (`restore_role_ids`) : présenté comme un contrôle d'accès, **jamais vérifié**. Seule la gate Owner côté API protège le restore. Un serveur qui configure ce champ croit restreindre l'accès ; il ne restreint rien.

### `welcome`

Les 6 réglages `anniversary_*` sont configurables par serveur. Aucun handler du bot ne les rend.

---

## 5. Points de sécurité ouverts

### Quatre gardes de permission vides

Quatre blocs `if user.is_some() {}` dont le commentaire annonçait un contrôle RBAC qui n'était pas écrit. Remplacés par des `TODO(secu)` nommant la protection réelle en place — mais la protection annoncée, elle, reste à écrire.

### Corrigé (pour mémoire, ne pas régresser)

`request_restore` acceptait `requested_by` depuis le corps de la requête : n'importe qui pouvait signer une restauration au nom d'un autre, sur l'opération la plus destructrice du produit. Le champ est maintenant dérivé de `WebUser`, jamais du corps.

---

## 6. Dette de moindre priorité

- **`sentinel-api/tests/test_helpers.rs`** — inclus dans ~40 binaires de test, chacun n'en consommant qu'une partie. Porte un `#[allow(dead_code)]` justifié ; le découper par domaine le supprimerait.
- **4 DTO miroirs de contrats d'API** — le bot ne lit qu'une partie des champs. `allow` justifié et commenté ; à revoir si le contrat se stabilise.
- **`handlers/moderation/purge.rs` et `handlers/community/voice_channels.rs`** restent sur `AppState` faute d'appartenir à un domaine unique. Les forcer dans un sous-état reconstituerait un god-object en miniature — c'est le **rangement des fichiers** qui est à revoir, pas le découpage des sous-états.
