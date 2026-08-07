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

## 3. Migration gRPC : 66 appels HTTP restants dans le bot

Les modules qui ont déjà un service proto sont terminés. Ce qui reste appartient à des modules **sans proto du tout** — chacun demande son `.proto`, son handler serveur, son câblage, ses tests.

| Module | Appels | Remarque |
|---|---|---|
| `confessions` | 11 | Le plus gros bloc ; utilise aussi `base_url()` en direct |
| `automod/vote` (6 fichiers) | 15 | `buttons`, `cards`, `discussion`, `events`, `finalize`, `post` |
| `automod/review` | 5 | |
| `rotation` | 5 | |
| `bump` | 4 | |
| `moderation/ban_sursis` | 4 | |
| `security` (5 fichiers) | 6 | `background`, `captcha_handler`, `join_handler`, `detectors/{lockdown,slowmode}` |
| `announcements` | 2 | |
| `welcome/handler` | 2 | |
| `handler.rs`, `sync.rs`, `logs_setup.rs` | 4 | Hors module |
| divers (`audit/handlers/message`, `embeds`, `tickets`, `voice`, `moderation/appeal`) | 5 | 1 appel chacun |

Estimation : ~8 nouveaux services proto.

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
