# Audit de conception — DiscordSentinel

> Audit des violations d'**architecture hexagonale** (API/core), de **logique métier mal placée dans le bot**, et d'**atomic design** (web).
> Méthode : exploration ciblée du code, chaque item cité avec `fichier:ligne`. Les findings « haute gravité » marqués ✅ ont été relus et confirmés manuellement.

---

## Synthèse

| Axe | Haute | Moyenne | Basse | État global |
|---|---|---|---|---|
| 1. Hexagonale (API/core) | 2 | — | 1 | Cœur quasi propre, **couche inbound systémiquement couplée à la DB** |
| 2. Métier dans le bot | 4 | 5 | 2 | Jeux casino propres ; **coude + automod portent de la vraie logique** |
| 3. Atomic design (web) | 5 | 6 | 3 | Architecture saine, **atomes/molecules court-circuitent services/composables** |

---

## Axe 1 — Architecture hexagonale (sentinel-core / sentinel-api)

Le **cœur** (`sentinel-core` : domain + application + ports) ne doit dépendre d'aucune infrastructure. Les **adapters inbound** (handlers HTTP/gRPC) ne doivent contenir aucune logique métier ni accès DB : ils délèguent à un use case (port inbound).

### 🟥 A. Le cœur dépend de l'infra ✅ (haute)

| Fichier:ligne | Problème |
|---|---|
| `sentinel-core/src/application/casino/manage_wallet_service.rs:304` | Un service applicatif lit `std::env::var("WALLET_STARTING_COINS")` en fallback. Le cœur ne doit voir que des ports — un accès env est de l'I/O infra. |

> C'est la **seule** dépendance infra restante dans `sentinel-core/src` (hors tests). Le reste est exemplaire : `ports/.../uow.rs` abstrait les transactions par handle opaque (zéro `sqlx`), aucun service n'instancie d'adapter concret (injection par traits partout).

**Reco** : injecter le solde de départ via un port de configuration, ou résoudre l'env côté adapter (bootstrap) et le passer au constructeur.

### 🟥 B. Les adapters inbound tapent la DB directement ✅ (haute, systémique)

~40 handlers HTTP **+** l'adapter gRPC exécutent du `sqlx::query`/`query_as`/`execute` directement sur `state.pg_pool` / `self.pg_pool`, au lieu de passer par un use case.

| Fichier:ligne | Problème |
|---|---|
| `adapters/inbound/grpc/community/sponsorships.rs:18,49,67…` | `pub pg_pool: sqlx::PgPool` membre de l'adapter gRPC + INSERT/SELECT bruts. En-tête (l.3-5) : *« sqlx direct car pas de use case unifié côté API »*. |
| `adapters/inbound/http/handlers/community/bump.rs:74,86,117,146,162` | **Métier + DB dans le handler** : calcul d'orchestration de récompense, INSERT `bump_events`, upsert `bump_guild_state`, SELECT due-reminders. |
| `adapters/inbound/http/handlers/system/rbac.rs:190,200,216` | **Métier (garde « dernier owner ») + requêtes DB brutes** dans le handler → le handler joue le rôle d'un use case. |
| `adapters/inbound/http/middleware/{component_gates,rbac,whitelist}.rs` | sqlx direct dans les middlewares. |

Autres fichiers concernés (hits `sqlx` confirmés dans `adapters/inbound`) :
`ai/dataset.rs`, `ai/ai_jobs.rs`, `casino/blackjack/game.rs`, `audit/{dashboard,security,snapshots}.rs`, `community/{announcements,guild_members,voice_channels}.rs`, `system/{bot_persistence,component_visibility,component_min_role,exports,health,info,invitations,lockdown,oauth,quarantine,security,server_events,slowmode,tickets}.rs`, `moderation/{actions,notes,automod}.rs`, `coude/{prestige,steal_attempts,tournaments}.rs`.

> Une partie du code est correctement portée (ex. `wallet_uc.credit(...)`, `export.execute(...)`), ce qui rend l'incohérence d'autant plus visible.

**Reco** : introduire des use cases/ports pour ces domaines, en priorisant ceux qui contiennent de la logique métier (`rbac`, `bump`, `sponsorships`).

### 🟦 C. Réserve mineure (basse)

| Fichier:ligne | Note |
|---|---|
| `ports/outbound/discord_api.rs:31,41` | `create_role`/`edit_role` renvoient un `serde_json::Value` brut de l'API Discord → l'adapter refait du parsing métier. Préférer un type domaine (`Role`). Ce n'est **pas** une fuite d'infra (sqlx/axum), juste un modèle anémique. |

### ✅ Propre
- Aucun port ne fuit de type `sqlx`/`axum`/`reqwest`/`tonic`.
- Aucune logique métier dans les adapters **outbound** (les repos Postgres se contentent de persister).
- Aucun use case ne court-circuite ses ports.

---

## Axe 2 — Logique métier mal placée dans le bot (sentinel-bot)

Principe du projet : *« Bot = interface légère ; logique métier centralisée dans l'API, jamais dans les modules du bot. »*

### 🟥 Haute gravité

| # | Fichier:ligne | Ce que fait le bot | Reco |
|---|---|---|---|
| 1 ✅ | `coude/commands/voler.rs:334-428` | **Résolution de combat de vol** : l'API renvoie des d20 bruts (`thief_d20`, `victim_d20`, `steal_pct_bp`), le bot ajoute les bonus (classe `fourbe`+4, boost, `DEF/10`, malus AFK), **décide le gagnant** (`if thief_total > target_total`) et calcule le butin `(coins*pct).max(1)`. | `POST /api/coude/steal` qui renvoie `{success, stolen, rolls_detail}` déjà résolus. |
| 2 ✅ | `automod/message_handler.rs:194-207` | `severe = flood_count >= severe_flood_max` puis **mute + suppression appliqués immédiatement**, sans verdict API, sur un seuil local. | Forwarder le signal flood ; l'API renvoie `{severe, auto_protect}`. |
| 3 | `coude/commands/donner.rs:176,224-225` | **Calcul de taxe + solde net** d'un don (`tax = amount*rate`, `received = amount-tax`) + validation `solde min` côté bot. | Déléguer à `transfer_coins()` (atomique côté API). |
| 4 | `community/sponsor.rs:460-506` | **Éligibilité parrainage** : calcule l'ancienneté et applique `min_parrain_days`/`max_filleul_days` pour autoriser/refuser. | `POST /api/community/sponsorships/validate`. |

### 🟧 Moyenne gravité

| Fichier:ligne | Problème |
|---|---|
| `tamagotchi/panel.rs:376-384` | Effets d'objets boutique **hardcodés** (`croquettes→hunger 25`, `repas→60`, `potion→cure`…) ; seul le prix est configurable. → exposer `/api/tamagotchi/shop/items`. |
| `automod/message_handler.rs:268-289` | Slowmode adaptatif **activé par le bot** sur seuil local (`channel.edit(rate_limit)`). |
| `automod/backend.rs:605-614` | Le bot fait du string-matching sur `reason` et **upgrade `Warn`→`Delete`**, écrasant le verdict API. |
| `audit/anomaly.rs:74-93` | **Décision** de déclencher une alerte sur seuil fixe (« 5 bans/60s ») — non contextualisé par taille du serveur. (Le tracker en mémoire est OK ; c'est la décision qui doit migrer.) |
| `coude` divers : `classe.rs:59-68,217-222`, `repos.rs:52-69`, `potion.rs:124-125,171-172`, `train.rs:96-98` (formules ATK/DEF/HP), `protection.rs:177-191`, `pari.rs:93-104`, `community/{sponsorship,exclusive_groups,prerequisites}.rs` | **Cooldowns / soldes / formules / règles d'exclusivité pré-validés ou recalculés côté bot** → duplication de règle + race entre check et débit. Laisser l'API rejeter atomiquement. |

### 🟦 Basse gravité
- `coude/commands/voler.rs:193-204` — pré-check UI du solde cible (tolérable si l'API reste l'autorité).
- `community/cooldown.rs:27-52` — cooldown en mémoire : acceptable si pur anti-spam d'interactions ; à migrer seulement s'il garde une récompense économique.

### ✅ Modules propres (vérifiés)
**blackjack** (mise/cartes/payout/cooldown délégués API), **slot / wheel / bump** (RNG & payout côté API, le bot anime un résultat déjà décidé), **détecteurs automod** (spam/insult/link/phishing/unicode → renvoient des flags, verdict côté backend), **audit** (agrégation/log/forward hors anomaly), **card_render tamagotchi** (mapping niveau→label, présentation).

---

## Axe 3 — Atomic design (sentinel-web/src)

Structure réelle : `atoms/` (~22), `molecules/` (~19), `organisms/` (~90+), `layouts/`, + logique externalisée dans `composables/` (~50) et `services/` (~45). Pas de dossier `pages/`/`templates/` dédié (géré au niveau `App.vue` + organisms). L'architecture cible est saine ; les violations sont des composants bas niveau qui **court-circuitent** services/composables.

### 🟥 Haute — des atomes font du réseau

| Fichier:ligne | Problème | Reco |
|---|---|---|
| `atoms/ChannelSelect.vue:3,28` ✅ | Importe `guildChannelsService`, fetch `listTextChannels(guildId)` en `onMounted`+`watch`, gère `loading`/`error`. | Recevoir `channels` en prop ; fetch dans une molecule/composable. |
| `atoms/CategorySelect.vue:3,32` | Idem (`listAllChannels()`). | Idem. |
| `atoms/RoleSelect.vue:3,28` | Idem (`discordRolesService.getAll()`) + tri/format couleur inline (l.37-51). | Idem. |
| `atoms/VoiceChannelSelect.vue:3` | Idem (`guildChannelsService` + état réseau). | Idem. |
| `atoms/ConnectionBanner.vue:13` | `fetch(/health)` + `listen("ws:event")` + `setInterval` polling 90s : infra réseau complète dans un atome. | Extraire `useConnectionStatus()`. |

### 🟧 Moyenne — molecules avec réseau / mauvaise classification

| Fichier:ligne | Problème |
|---|---|
| `molecules/AddWatchModal.vue:62` | `fetch(/api/watched-users, POST)` **brut** (n'utilise même pas `watchedUsersService`). |
| `molecules/UserDossierPanel.vue:6,31` | **Importe `organisms/DataTable.vue`** (inversion de hiérarchie) + action réseau `watchedUsersService.remove()` + helpers métier (`riskLabel`, `totalInfractions`). → c'est un **organism**. |
| `molecules/IdMultiplierMapField.vue:3-4` | Double-service (`guildChannelsService` + `discordRolesService`) + fetch interne → plutôt un organism. |
| `molecules/IdsListPickerField.vue:3-4` | Idem. |
| `molecules/GameServerStatsBar.vue:4,18` | `gamePortalService` + **`setInterval` polling** interne. → extraire `useGameServerStats()`. |
| `molecules/GameServer{ConfigModal,SessionsModal}.vue` | `gamePortalService` (à surveiller). |

### 🟦 Basse — logique de présentation/métier inline
- `atoms/RoleSelect.vue:37-39,48-51` (tri `position`, `fmtColor` int→hex couplé à `DiscordRole`).
- `molecules/GameServerStatsBar.vue:20-44` (seuils mem/cpu + code couleur).
- `molecules/UserDossierPanel.vue:43-55` (`riskLabel`, `totalInfractions` = règles de domaine en présentation).

### ✅ Propre
Les autres atomes (`AppButton`, `AppInput`, `AppModal`, `AppSelect`, `AppToggle`, `AppBadge`, `EmptyState`, `LoadingState`, `FormField`, …) et molecules restantes : aucun `axios|fetch|/services/|useStore`. Aucun import de store Pinia dans atoms/molecules.

---

## Priorités recommandées

1. **(Haute)** `coude/voler.rs` (résolution de combat) + `automod/message_handler.rs` (sanction flood / slowmode) — logique de jeu/modération contournable côté bot.
2. **(Haute)** Introduire use cases/ports pour les handlers SQL-direct **porteurs de métier** (`bump`, `rbac`, `sponsorships`).
3. **(Haute, rapide)** Retirer `std::env::var` de `manage_wallet_service.rs:304` (cœur).
4. **(Moyenne, rapide)** Sortir le data-fetching des 5 atomes web (`*Select` + `ConnectionBanner`) — l'infra composables/services existe déjà.
5. **(Moyenne)** `donner.rs` (taxe), `sponsor.rs` (éligibilité), effets boutique tamagotchi, override automod, anomaly.
6. **(Nettoyage)** Pré-validations cooldown/solde côté bot : OK en defense-in-depth, mais ne pas les considérer comme source de vérité.
