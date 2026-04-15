# 🎙️ Voice-bot — Audit & plan de refactor

> Date : 2026-04-15 — référence `docs/ARCHITECTURE_RULES.md`
> Cible : `bots/voice-bot/` (aucun worker voice séparé)

Audit du bot voice contre les règles d'architecture imposées à
DiscordSentinel : bot 100 % thin, zéro SQL, zéro logique métier, zéro
catalogue hardcodé côté bot.

---

## 🟢 Score de conformité : **92 / 100**

| Critère | Statut |
|---|---|
| Zéro SQL | ✅ aucun `sqlx`, aucun `PgPool` |
| gRPC-only pour parler à l'API | ✅ Phase 7A migré |
| Pas de catalogue hardcodé | ⚠️ 5 constantes business hardcodées |
| Pas de décision métier | ⚠️ 1 check d'autorisation en bot |
| Hexagonal côté API | ✅ entités + ports + service + adapter + gRPC |

---

## ✅ Ce qui est conforme

- **Aucun accès SQL direct** : `grep -r "sqlx\|PgPool" bots/voice-bot/src/`
  → 0 résultat.
- **Client gRPC propre** : `ApiClient::from_data()` →
  `SentinelGrpcClient`, migration Phase 7A finalisée.
- **Séparation des responsabilités** : les handlers ne font que
  interactions Discord, routing, et appels gRPC.
- **Caches en mémoire corrects** : `VoiceOwnerMap`, `TextToVoiceMap`,
  `MembersToVoiceMap` chargées au boot depuis l'API, pas depuis une DB
  locale.
- **Delegation complète** : ownership, bans, whitelist, thèmes —
  toutes les mutations passent par `ManageVoiceChannelsUseCase` via
  gRPC.
- **API side complet** : domain `voice_channel.rs`, ports in/out,
  service `manage_voice_channels_service.rs`, adapter Postgres, gRPC
  handler — tout est en place.

---

## ⚠️ Violations trouvées

Triées par sévérité : constantes hardcodées d'abord (faciles à
corriger), logique métier à la fin.

### V1 — Limite de membres des salons "game"

**Fichier** : `bots/voice-bot/src/handlers/voice/channel_lifecycle.rs:60`

```rust
let default_user_limit: u32 = if kind == "game" { 10 } else { 0 };
```

- **Violation** : la règle « les salons `game` ont une limite de 10
  membres par défaut » est une règle gameplay hardcodée dans le bot.
- **Fix** : la limite existe déjà dans
  `VoiceChannelTheme.member_limit: Option<i32>` côté API. Le bot doit
  charger les thèmes au boot (ou au moment du create) et utiliser la
  valeur retournée au lieu du `if kind == "game"`.
- **Effort** : ~15 LOC (fetch + usage).

### V2 — Cooldown de création de salon

**Fichier** : `bots/voice-bot/src/state/cooldown_tracker.rs:6`

```rust
const COOLDOWN_SECS: u64 = 5;
```

- **Violation** : 5 s de cooldown entre créations de salon — règle
  moderation hardcodée en dur.
- **Fix** :
  1. Nouvelle entité domain `VoiceChannelConfig` avec
     `creation_cooldown_secs: u64`.
  2. Nouvelle méthode UC `get_config(guild_id)` qui lit
     `bot_guild_config` (fallback constant par défaut si pas de
     valeur).
  3. Nouveau RPC `VoiceChannelsService.GetConfig`.
  4. Le bot fetch au boot et cache dans un `Arc<VoiceConfig>` passé
     au `CooldownTracker::new(config)`.
- **Effort** : ~30 LOC (domain + UC + proto + bot wiring).

### V3 — Détection de flood texte

**Fichier** : `bots/voice-bot/src/state/flood_tracker.rs:6-7`

```rust
const MAX_MESSAGES: usize = 5;
const TIME_WINDOW_SECS: u64 = 5;
```

- **Violation** : seuils de flood (5 messages / 5 s = flood) hardcodés.
- **Fix** : ajoute `flood_max_messages` et `flood_time_window_secs` au
  `VoiceChannelConfig` domain (même migration que V2). Le
  `FloodTracker::new(config)` prend ces valeurs au lieu des constantes.
- **Effort** : ~20 LOC supplémentaires sur V2.

### V4 — Délai anti-race avant delete d'un salon vide

**Fichier** : `bots/voice-bot/src/handlers/voice/channel_lifecycle.rs:371`

```rust
tokio::time::sleep(std::time::Duration::from_secs(2)).await;
```

- **Violation** : 2 s avant de vérifier qu'un salon est bien vide —
  anti-race contre un rejoin tardif. Hardcodé.
- **Fix** : ajoute `empty_cleanup_delay_secs` au `VoiceChannelConfig`
  domain. Propager dans `check_and_delete_empty(…, delay)`.
- **Effort** : ~15 LOC (juste passer la valeur).

### V5 — Limite de 10 users dans les select menus

**Fichier** : `bots/voice-bot/src/handlers/voice/channel_lifecycle.rs:664`
et `handlers/voice/access_control.rs:150`

```rust
.max_values(10)  // dans le menu invite user
```

- **Violation** : « max 10 users par select » hardcodé côté UI. Ne
  matche pas forcément le `member_limit` réel du salon.
- **Fix** : passer la `member_limit` du `VoiceChannelResponse` déjà
  dans le flow aux builders qui créent les select menus. Utiliser
  `member_limit.min(100)` (limite Discord SDK).
- **Effort** : ~15 LOC.

### V6 — Double check d'ownership côté bot

**Fichier** : `bots/voice-bot/src/interactions/mod.rs:77`

```rust
if ch.owner_id != user_id.get().to_string() {
    reply_ephemeral(…, "Ce salon ne t'appartient pas.");
    return;
}
```

- **Statut** : **accepté comme UX-only**. L'API enforce l'ownership
  sur toutes les RPCs mutantes ; ce check bot est une optimisation
  pour éviter un round-trip gRPC quand on peut prédire l'échec. OK
  tant que ça ne **remplace pas** la vérif côté API.
- **Action** : aucune. Documenté ici pour traçabilité.

### V7 — Vote-kick : check présence de l'admin dans le vocal

**Fichier** : `bots/voice-bot/src/interactions/vote_kick.rs:62-70`

```rust
let admin_present = component.guild_id
    .and_then(|gid| ctx.cache.guild(gid))
    .map(|guild| {
        guild.voice_states.values().any(|vs| {
            vs.channel_id == Some(voice_channel_id) && vs.user_id == owner_id
        })
    })
    .unwrap_or(false);

if admin_present {
    respond_ephemeral(…, "Un admin est present…");
    return;
}
```

- **Violation** : le bot prend une décision métier (« peut-on voter
  pour kicker ? »). Le check "owner présent dans le vocal" est
  **pseudo-logique bot** parce que l'info vient de la cache serenity
  (`voice_states`), donc techniquement l'API n'aurait pas cette info
  sans que le bot la pousse.
- **Fix** :
  - Soit exposer côté API une règle `can_initiate_vote_kick(channel_id,
    owner_present: bool)` qui retourne oui/non avec le message. Le
    bot lui passe `owner_present` depuis sa cache. L'API applique la
    règle.
  - Soit laisser la cache serenity comme source de vérité (c'est la
    seule qui l'a) mais **formaliser** que c'est une exception,
    documentée en commentaire.
- **Recommandation** : option 2 (commenter comme exception) — option
  1 n'apporte qu'un aller-retour gRPC pour une logique de 3 lignes.
- **Effort** : 5 LOC de commentaire.

---

## 📦 Refactor proposé — plan d'action

### Phase 1 : `VoiceChannelConfig` en domain (corrige V2/V3/V4)

**Effort** : ~2 h

1. **Migration SQL `NNN_voice_channel_config.sql`** — rien à
   persister, on utilise `bot_guild_config` existant avec des clés
   spécifiques (`voice_creation_cooldown_secs`, `voice_flood_max`,
   `voice_flood_window_secs`, `voice_cleanup_delay_secs`).

2. **Domain** : `services/api/src/domain/entities/voice_channel_config.rs`

   ```rust
   #[derive(Debug, Clone, Copy)]
   pub struct VoiceChannelConfig {
       pub creation_cooldown_secs: u64,
       pub flood_max_messages: usize,
       pub flood_time_window_secs: u64,
       pub empty_cleanup_delay_secs: u64,
   }

   impl Default for VoiceChannelConfig {
       fn default() -> Self {
           Self {
               creation_cooldown_secs: 5,
               flood_max_messages: 5,
               flood_time_window_secs: 5,
               empty_cleanup_delay_secs: 2,
           }
       }
   }
   ```

3. **Inbound port** : ajouter
   `ManageVoiceChannelsUseCase::get_config(guild_id) -> Result<VoiceChannelConfig>`.

4. **Service application** : lit `bot_config_repo.get_config(guild_id,
   "voice-bot")` et parse chaque clé, fallback `Default::default()`.

5. **Proto** : nouveau message `VoiceChannelConfig` + RPC `GetConfig`.

6. **Bot** : nouveau `ApiClient::get_voice_config(guild_id)`, appelé
   une seule fois par guild au boot (ou à la première interaction),
   caché dans un `RwLock<HashMap<GuildId, VoiceChannelConfig>>`.

7. **Remplacer** les constantes dans `cooldown_tracker.rs`,
   `flood_tracker.rs`, `channel_lifecycle.rs` par une lecture sur le
   cache.

### Phase 2 : Thèmes dynamiques pour `member_limit` (corrige V1/V5)

**Effort** : ~1 h30

- **Déjà en place** : `VoiceChannelTheme.member_limit` existe en
  domain et est exposé via le RPC `ListThemes`.
- **À faire** :
  1. Le bot fetch la liste des thèmes au boot (comme coude-bot fetch
     `GetCatalog`) et cache dans `ThemeCacheKey: TypeMapKey`.
  2. Dans `channel_lifecycle.rs:60`, remplacer le `if kind == "game"`
     par `theme_cache.get(kind).member_limit.unwrap_or(0) as u32`.
  3. Dans les select menus (V5), passer le `member_limit` récupéré
     depuis la réponse API au builder du menu.

### Phase 3 : Vote-kick documenté comme exception (V7)

**Effort** : ~10 min

- Ajouter un commentaire au-dessus du bloc :

  ```rust
  // ARCHITECTURE: exception aux regles thin — la presence de
  // l'owner dans le voice channel vient de la cache serenity
  // (`voice_states`), qui n'est pas accessible cote API sans que le
  // bot la pousse. Faire un RPC juste pour appliquer la regle
  // "owner present → pas de vote" couterait plus cher qu'il ne
  // rapporte. Justification dans docs/VOICE_BOT_REFACTOR.md#v7.
  ```

### Phase 4 : Tests + cargo check + commit

- `cargo test -p sentinel-api --lib voice_channel_config`
- `cargo check --workspace`
- Commit en 3 pièces :
  1. `feat(voice-api): VoiceChannelConfig domain + RPC GetConfig`
  2. `refactor(voice-bot): lit config depuis l'API au lieu des constantes`
  3. `refactor(voice-bot): utilise member_limit des themes pour les select menus`

### Total

~200 LOC modifiés / ajoutés, réparties :
- 80 LOC côté API (domain + port + service + proto + gRPC handler)
- 120 LOC côté voice-bot (cache + consommation + suppression des constantes)

---

## 🗓️ Recommandation de priorité

**Pas urgent**. Le bot est 95 % conforme et ces violations sont toutes
des constantes hardcodées "raisonnables" avec des valeurs sensées par
défaut. Le refactor apporterait surtout de la **flexibilité ops**
(tuning sans redeploiement) et un **aligement strict avec les
règles** — utile pour garder la discipline sur les prochaines features.

À planifier en **sprint fillers** plutôt qu'en urgence.

---

## 🔗 Références

- [`docs/ARCHITECTURE_RULES.md`](./ARCHITECTURE_RULES.md) — règles
  appliquées.
- [`bots/voice-bot/src/state/cooldown_tracker.rs`](../bots/voice-bot/src/state/cooldown_tracker.rs)
- [`bots/voice-bot/src/state/flood_tracker.rs`](../bots/voice-bot/src/state/flood_tracker.rs)
- [`bots/voice-bot/src/handlers/voice/channel_lifecycle.rs`](../bots/voice-bot/src/handlers/voice/channel_lifecycle.rs)
- [`services/api/src/domain/entities/voice_channel.rs`](../services/api/src/domain/entities/voice_channel.rs)
  — emplacement du nouveau `VoiceChannelConfig`.
- [`services/api/src/ports/inbound/manage_voice_channels.rs`](../services/api/src/ports/inbound/manage_voice_channels.rs)
  — trait à étendre.
