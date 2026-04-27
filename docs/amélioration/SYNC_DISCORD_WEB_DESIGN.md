# 🔄 Synchronisation Discord ↔ Web — Design

> **Date** : 2026-04-27
> **Statut** : Proposition technique — à valider avant implémentation
> **Périmètre** : toutes les actions admin/modération/gameplay qui ont une représentation persistante sur Discord ET sur la web admin.
> **Contrainte** : toute communication passe par l'API Rust (pas de DB ou Discord direct depuis le web).

---

## 1. 🩺 Diagnostic actuel

### Le symptôme observé

> *« Quand on annule une proposition de ban depuis Discord, la page web ne le voit pas. Et inversement. »*

Plus largement : **toutes les actions qui existent en double (Discord + web) divergent silencieusement** parce que les deux clients lisent la même DB sans se notifier mutuellement.

### Pourquoi ça arrive

L'archi actuelle est en **mode pull-pull** :
- **Discord** poste un embed avec boutons → state visuel embarqué dans le message Discord (volatile, jamais re-fetché par le bot).
- **Web** appelle `GET /api/...` à l'ouverture de page → state lu depuis la DB.
- **Aucun des deux ne sait quand l'autre agit**.

Conséquences concrètes :
- Annulation Discord → DB OK, mais le message web reste affiché jusqu'au F5.
- Ban exécuté côté web → DB OK, mais les boutons Discord restent cliquables (re-clic possible 5 min plus tard → race / TOCTOU).
- Ouverture de la page web après une action Discord → entité « zombie » potentielle.
- Two-way race : web et Discord cliquent simultanément → la 2ᵉ action peut écraser la 1ʳᵉ sans guard.

### Ce qui existe déjà côté infra

- ✅ `EventBroadcaster` injecté dans `AppState` (cf. `services/api/src/adapters/inbound/http/state.rs`)
- ✅ `BaseApiClient` côté bot pour appeler l'API
- ✅ Modèle d'infraction unique (`infractions` table) avec `id` UUID
- ✅ Permissions Discord correctement gardées (audit OK)
- ❌ Pas de table de mapping `action_id ↔ Discord message`
- ❌ Pas de canal SSE/websocket pour pousser les events vers le web
- ❌ Bot ne stocke pas systématiquement les `message_id` qu'il poste

---

## 2. 🏗️ Architecture cible — 3 piliers

### Pilier 1 — Single source of truth = API/DB

C'est déjà le cas, juste à renforcer. Toute action (ban, mute, ticket, panel, prime, vendetta, …) passe par un use case API qui :

1. **Valide** l'action (permissions, état)
2. **Persiste** avec un statut explicite (`pending` / `executed` / `cancelled` / `expired`)
3. **Renvoie un `action_id` UUID** — la clé universelle de correspondance
4. **Émet un event** sur le bus

Ni Discord ni Web ne stockent l'état localement. Ils sont des **vues** réactives.

### Pilier 2 — Correlation IDs sur les messages Discord

Nouvelle table de mapping entre une entité métier et le message Discord qui la représente :

```sql
CREATE TABLE discord_action_messages (
    action_id      UUID NOT NULL,        -- FK logique vers l'entité métier
    kind           TEXT NOT NULL,        -- 'ban_proposal' | 'ticket' | 'roles_panel' | ...
    guild_id       TEXT NOT NULL,
    channel_id     TEXT NOT NULL,
    message_id     TEXT NOT NULL,
    posted_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_edited_at TIMESTAMPTZ,
    PRIMARY KEY (action_id, kind),
    UNIQUE (guild_id, channel_id, message_id)
);

CREATE INDEX idx_dam_kind_guild ON discord_action_messages(kind, guild_id);
```

**Règle d'usage** :
- Quand le bot poste un message lié à une action → il INSERT immédiatement.
- Quand l'action change de statut → l'API émet un event, le bot lit la table pour retrouver `(channel_id, message_id)` et edit/delete.
- Quand le message Discord est supprimé manuellement → row supprimée par `MESSAGE_DELETE` event handler.

### Pilier 3 — Event bus bidirectionnel

```
┌─────────┐    HTTP POST    ┌─────────────┐
│   Web   │ ──────────────▶│             │
└─────────┘                 │             │
     ▲                      │     API     │
     │ SSE                  │             │
     │ /api/events          │             │
     │                      │             │
┌─────────┐    HTTP POST    │             │
│   Bot   │ ──────────────▶ │             │
└─────────┘                 └──────┬──────┘
     ▲                             │
     │ gRPC / publish_event        │
     └─────────────────────────────┘
```

L'API émet un événement chaque fois qu'une entité change. Bot et Web s'y abonnent indépendamment. Les deux réagissent à la même source de vérité.

#### Flow A — Action depuis le web

```
[Web clique « Bannir »]
  │ HTTP POST /api/moderation/{action_id}/execute-ban
  ▼
[API] valide + UPDATE infractions SET status='executed' + INSERT moderation_actions
  │ broadcaster.publish("ban.executed", { action_id, guild_id, target_id, by })
  ▼ ▼
  │ │
[Bot subscribe]                [Web subscribe SSE]
  │                              │
  │ lit discord_action_messages  │ retire la ligne de la liste
  │ pour (action_id)             │ affiche toast "Ban exécuté"
  │ edit le message Discord :    │
  │   - retire les boutons       │
  │   - change l'embed couleur   │
  │   - ajoute "Banni par X"     │
  ▼                              ▼
```

#### Flow B — Action depuis Discord

```
[User clique bouton "Annuler" sur Discord]
  │ component handler bot
  │ parse custom_id = "ban_cancel:{action_id}"
  │ HTTP POST /api/moderation/{action_id}/cancel  (Authorization: Bearer bot_token)
  ▼
[API] valide + UPDATE infractions SET status='cancelled'
  │ broadcaster.publish("ban.cancelled", { action_id, ... })
  ▼ ▼
  │ │
[Bot subscribe]                [Web subscribe SSE]
  │ edit le message :            │ retire la ligne / la grise
  │   "Annulé par <X>"           │ refresh le compteur
  │   retire les boutons         │
  ▼                              ▼
```

**Personne ne pousse à personne directement.** Toujours via l'API + event bus.

---

## 3. 📜 Catalogue d'événements

Standardiser le format d'événement :

```typescript
type DomainEvent = {
  event_type: string;          // ex: "moderation.ban.executed"
  guild_id: string;
  action_id: string;           // UUID, clé de corrélation
  emitted_at: string;          // RFC3339
  actor: {                     // qui a déclenché
    user_id: string;
    source: "discord" | "web" | "system";
  };
  payload: Record<string, any>; // données spécifiques à l'event
};
```

### Liste des events à publier

| Domaine | Events | Sources émettrices |
|---|---|---|
| **Modération** | `ban.proposed`, `ban.executed`, `ban.cancelled`, `ban.expired`, `mute.applied`, `mute.removed`, `mute.expired`, `warn.added`, `warn.removed` | API |
| **Tickets** | `ticket.opened`, `ticket.assigned`, `ticket.replied`, `ticket.closed`, `ticket.transcript_ready` | API |
| **Reviews** | `review.requested`, `review.resolved` | API |
| **Roles panels** | `panel.deployed`, `panel.role_added`, `panel.role_removed` | API |
| **Games** | `game.created`, `game.deleted`, `game.panel_deployed` | API |
| **Coude — combats** | `combat.proposed`, `combat.accepted`, `combat.refused`, `combat.cancelled`, `combat.resolved` | API |
| **Coude — bounties** | `bounty.opened`, `bounty.contributed`, `bounty.claimed` | API |
| **Coude — coalitions** | `coalition.opened`, `coalition.joined`, `coalition.broken` | API |
| **Coude — vendettas** | `vendetta.declared`, `vendetta.won`, `vendetta.lost` | API |
| **Coude — tournois** | `tournament.started`, `tournament.bracket_updated`, `tournament.finished` | API |

### Convention de nommage

Format : `{domain}.{entity}.{action}` en passé.

---

## 4. 🌐 Endpoints API à créer / modifier

### Nouveaux

#### `POST /api/discord-messages/register`
Body : `{ action_id, kind, guild_id, channel_id, message_id }`
Effet : INSERT dans `discord_action_messages`. Appelé par le bot juste après chaque post.

#### `DELETE /api/discord-messages/{action_id}/{kind}`
Effet : retire la row (utile si message supprimé manuellement, ou archive nettoyé).

#### `GET /api/discord-messages/{action_id}`
Retourne tous les messages liés à une action (utile au bot pour edit/delete).

#### `GET /api/events?guild_id=...&types=...`
Server-Sent Events. Le client (web) s'abonne, reçoit un flux d'`DomainEvent`.

Headers :
- `Authorization: Bearer <token>` (auth web admin)
- `Last-Event-ID` (resume sur déconnexion)

#### Pour chaque action existante : ajouter un endpoint `/cancel` ou équivalent
Souvent déjà présent, mais s'assurer qu'il est exposé pour les deux clients.

### Modifications

- `POST /api/moderation/execute-ban` → publier event `moderation.ban.executed`.
- `POST /api/moderation/{id}/cancel` → publier event `moderation.ban.cancelled`.
- `POST /api/conduct/sync-ban-proposals` → publier `moderation.ban.proposed` pour chaque création.
- (idem pour tous les use cases qui mutent une entité visible).

---

## 5. 🤖 Refacto côté bot

### A) Enregistrer le message après chaque post

Pattern à appliquer dans tous les handlers qui postent un message « action » :

```rust
let posted_msg = channel
    .send_message(&ctx.http, CreateMessage::new().embed(embed).components(rows))
    .await?;

// Nouveau : enregistrer la correspondance
api.register_discord_message(
    action_id,
    "ban_proposal",
    &guild_id,
    &posted_msg.channel_id.to_string(),
    &posted_msg.id.to_string(),
).await.ok(); // best-effort : si échec, le bot continue, mais on perd le link
```

**Fichiers concernés** (à scanner et patcher) :
- `bots/sentinel-bot/src/modules/moderation/commands/*.rs` (warn, mute, ban…)
- `bots/sentinel-bot/src/modules/tickets/`
- `bots/sentinel-bot/src/modules/community/roles_panel.rs`
- `bots/sentinel-bot/src/modules/games/`
- `bots/sentinel-bot/src/modules/coude/commands/coude/mod.rs` (challenge embed)
- `bots/sentinel-bot/src/modules/coude/commands/coalition.rs`, `bounty.rs`, `vendetta.rs`
- `bots/sentinel-bot/src/modules/coude/tournament_events.rs`

### B) Encoder l'`action_id` dans les `custom_id` des boutons

Convention : `{namespace}:{action}:{action_id}`

Exemple :
```rust
let cancel_btn = CreateButton::new(format!("ban:cancel:{}", action_id))
    .label("Annuler la proposition")
    .style(ButtonStyle::Danger);
```

Le component handler parse cet ID et appelle l'API directement.

### C) Subscribe à l'event bus côté bot

Si le bot doit réagir à des events émis par le web (ex. ban exécuté depuis le web → edit le message Discord), il doit subscribe.

Deux options :
1. **gRPC streaming** : `EventBus.Subscribe()` côté API expose un stream.
2. **HTTP long-polling / SSE** : `GET /api/events` avec `Accept: text/event-stream`.

**Recommandé** : gRPC streaming (déjà l'infra existe, plus performant que SSE entre services internes).

```rust
// services/workers/event-listener-worker (nouveau)
let stream = grpc.events().subscribe(&[
    "moderation.*",
    "tickets.*",
    "community.*",
]).await?;

while let Some(event) = stream.next().await {
    match event.event_type.as_str() {
        "moderation.ban.executed" => update_ban_message(&ctx, &event).await,
        "moderation.ban.cancelled" => update_ban_message(&ctx, &event).await,
        "tickets.ticket.closed" => archive_ticket_channel(&ctx, &event).await,
        // ...
    }
}
```

### D) Helper d'édition centralisé

```rust
async fn update_discord_message_for_action(
    ctx: &Context,
    api: &ApiClient,
    action_id: Uuid,
    kind: &str,
    new_embed: CreateEmbed,
    new_components: Vec<CreateActionRow>,
) -> Result<(), String> {
    let mappings = api.get_discord_messages(action_id).await?;
    for m in mappings.into_iter().filter(|m| m.kind == kind) {
        let channel = ChannelId::new(m.channel_id.parse()?);
        let msg_id = MessageId::new(m.message_id.parse()?);
        channel.edit_message(&ctx.http, msg_id,
            EditMessage::new().embed(new_embed.clone()).components(new_components.clone())
        ).await?;
    }
    Ok(())
}
```

---

## 6. 🌐 Refacto côté web

### A) Subscriber SSE global

Composable Vue à créer :

```typescript
// apps/web/src/composables/useEventStream.ts
export function useEventStream(guildId: Ref<string | null>, types: string[]) {
  const events = ref<DomainEvent[]>([]);

  watchEffect(() => {
    if (!guildId.value) return;
    const url = `/api/events?guild_id=${guildId.value}&types=${types.join(',')}`;
    const sse = new EventSource(url, { withCredentials: true });

    sse.onmessage = (e) => {
      const event = JSON.parse(e.data) as DomainEvent;
      events.value.push(event);
    };

    return () => sse.close();
  });

  return { events };
}
```

### B) Réagir aux events dans chaque composable

Exemple `useBans.ts` :

```typescript
const { events } = useEventStream(guildIdFilter, [
  "moderation.ban.proposed",
  "moderation.ban.executed",
  "moderation.ban.cancelled",
]);

watch(events, (list) => {
  const last = list.at(-1);
  if (!last) return;
  switch (last.event_type) {
    case "moderation.ban.executed":
    case "moderation.ban.cancelled":
      banProposals.value = banProposals.value.filter(b => b.id !== last.action_id);
      break;
    case "moderation.ban.proposed":
      // ajout optimiste sans refetch full
      banProposals.value.push(last.payload as Infraction);
      break;
  }
}, { deep: true });
```

### C) Toast notifications globales

```typescript
// apps/web/src/composables/useNotifications.ts
const { events } = useEventStream(currentGuild, ["*"]);
watch(events, (list) => {
  const last = list.at(-1);
  if (!last || last.actor.source === "web") return; // ne pas toaster ses propres actions
  showToast(formatEventMessage(last));
});
```

---

## 7. 🛡️ Idempotence & garde-fous

### Conflit web/Discord simultané

Le UPDATE en DB doit être conditionnel :

```sql
UPDATE infractions
SET status = 'executed', resolved_at = NOW(), resolved_by = $1
WHERE id = $2 AND status = 'pending'
RETURNING id;
```

Si `RETURNING` est vide → l'action a déjà été résolue. Le caller reçoit `409 Conflict`. Le client (web ou Discord) affiche « Cette action a déjà été traitée par X ».

### Bouton Discord cliqué deux fois

Le 2ᵉ click reçoit aussi `409 Conflict`, le bot répond en ephemeral « Cette action est déjà traitée ».

### Replay / déconnexion SSE

Chaque event a un `event_id` numérique séquentiel. Le client envoie `Last-Event-ID` à la reconnexion, l'API rejoue les events depuis ce point (table `event_log` avec rétention 24-72h).

---

## 8. 🚀 Plan de migration progressif

### Phase 1 — Infrastructure (1 semaine)

1. Migration SQL : table `discord_action_messages` + table `event_log` (rétention 72h).
2. Endpoints API :
   - `POST /api/discord-messages/register`
   - `DELETE /api/discord-messages/{action_id}/{kind}`
   - `GET /api/discord-messages/{action_id}`
   - `GET /api/events` (SSE)
3. Étendre `EventBroadcaster` pour persister chaque event dans `event_log`.
4. gRPC `EventBus.Subscribe(filters)` server streaming.

### Phase 2 — Pilote sur les bans (3 jours)

5. Bot : enregistrer `message_id` à chaque proposal posée. Encoder `action_id` dans les `custom_id`.
6. API : émettre `moderation.ban.{proposed|executed|cancelled}`.
7. Bot : subscribe et edit le message à la réception.
8. Web : `useBans.ts` consomme le SSE.
9. **Test bilatéral** : action sur Discord visible immédiatement sur web et vice versa.

### Phase 3 — Étendre feature par feature (1-2 jours / feature)

Ordre suggéré (priorité décroissante) :
1. Tickets
2. Mutes / Warns
3. Reviews
4. Roles panels
5. Coude — combats (rafraîchir les paris en live)
6. Coude — bounties / coalitions / vendettas
7. Tournois

### Phase 4 — Polish

10. Toast notifications globales sur le web.
11. Indicateur "live" sur la page web (vert quand SSE connecté).
12. Métriques : event lag p95, SSE reconnections, mismatchs détectés.

---

## 9. 📊 Métriques de succès

| Métrique | Cible |
|---|---|
| Délai action Discord → reflet web | < 1 s p95 |
| Délai action web → reflet Discord | < 1 s p95 |
| % d'actions Discord avec `message_id` enregistré | > 99 % |
| Conflits 409 / jour | < 5 (sinon il y a un bug ou de la contention pathologique) |
| SSE reconnexions / utilisateur / heure | < 2 |
| Events perdus (gap dans `event_id`) | 0 |

---

## 10. 🎯 Décisions à prendre avant de commencer

1. **Transport bot ↔ API events** : gRPC streaming ou HTTP SSE interne ?
   *Recommandé : gRPC* (l'infra est là, performant, typé).

2. **Auth SSE web** : cookie session ou bearer token ?
   *Recommandé : cookie session* (existe déjà côté web admin).

3. **Rétention event_log** : 24h ? 72h ? 7 jours ?
   *Recommandé : 72h*. Suffit pour gérer les déconnexions, pas trop lourd.

4. **Format event** : JSON ou Protobuf ?
   *Recommandé : JSON* pour le SSE (web friendly), Protobuf pour le gRPC interne.

5. **Quand un message Discord est supprimé manuellement** par un admin, que faire ?
   *Recommandé* : event `MESSAGE_DELETE` côté bot → DELETE row dans `discord_action_messages`. L'action elle-même reste en DB.

6. **Backfill** : pour les actions existantes sans `message_id`, on accepte qu'elles ne soient plus synchronisables ?
   *Recommandé* : oui (legacy). Nouvelles actions seulement.

---

## 11. 🚧 Risques & mitigations

| Risque | Mitigation |
|---|---|
| Bot down → events perdus | `event_log` permet de rejouer au reboot. Bot maintient un curseur en DB. |
| Web SSE saturé | Filtrer côté serveur par guild + types. Limiter à N connexions/utilisateur. |
| Bot rate-limit Discord | Toujours throttle les edits (déjà fait sur `/progression-resync`). |
| Event spam | Debounce côté web (ne pas refetch si 10 events arrivent en 100 ms). |
| Action_id collision | UUID v4, négligeable (10⁻¹⁸). |
| Compatibilité ascendante | Phase 2 (pilote ban) sans toucher aux autres features → rollback facile. |

---

## 12. ❓ FAQ

**Q : Pourquoi pas un websocket ?**
R : SSE suffit (un seul sens serveur→client, le client renvoie ses commandes par HTTP normal). Plus simple, traverse les proxies, reconnect natif.

**Q : Pourquoi pas Postgres LISTEN/NOTIFY directement vers le bot ?**
R : Couple le bot à la DB. La règle est : tout passe par l'API. L'event bus de l'API est la seule porte d'entrée.

**Q : Pourquoi un `action_id` plutôt que de réutiliser l'`id` de l'infraction ?**
R : Tu peux le faire si chaque entité métier a un UUID stable. Le terme `action_id` est juste un alias. L'important : **un identifiant unique global qui voyage de bout en bout**.

**Q : Et si le bot et le web mutent la même action exactement au même millième de seconde ?**
R : L'UPDATE conditionnel (`WHERE status = 'pending'`) en DB sérialise. Un des deux gagne, l'autre reçoit 409. C'est l'unique solution propre.

---

## 📌 TL;DR

| Quoi | Pourquoi |
|---|---|
| **Table `discord_action_messages`** | Permet à l'API de retrouver quel message Discord représente une action |
| **`action_id` UUID** dans les `custom_id` Discord | Le bot retrouve l'action depuis un click bouton |
| **Event bus** publie `domain.entity.action` à chaque mutation | Bot et web réagissent à la même source de vérité |
| **SSE** pour le web | Refresh sans F5, sans polling |
| **gRPC streaming** pour le bot | Bot écoute les events depuis l'API, edit les messages en conséquence |
| **UPDATE conditionnel** sur le statut | Sérialise les actions concurrentes (web + Discord) |

**Effort estimé** : 1 semaine d'infra + 1-2 jours par feature à brancher.

---

*Document à valider avant implémentation. Voir aussi `COUDE_ARCHITECTURE_AUDIT.md` pour le contexte hexagonal.*
