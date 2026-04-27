# 📘 Guide de poche — Sync Discord ↔ Web

> Référence rapide pour comprendre, debugger et étendre la synchronisation
> bilatérale Discord ↔ Web admin via notre API Rust.

---

## 🏛️ Architecture en une image

```
┌─────────────┐         ┌──────────────┐         ┌─────────────┐
│  Vue Web    │ ──────► │   Rust API   │ ◄────── │  Rust Bot   │
│ (apps/web)  │ HTTP/WS │   (axum)     │  HTTP   │ (serenity)  │
└─────────────┘         │              │         └──────┬──────┘
                        │  Postgres    │                │
                        │  Redis       │                │ Discord API
                        └──────┬───────┘                ▼
                               │ Redis Stream    ┌──────────┐
                               └───────────────► │ Discord  │
                                                 └──────────┘
```

**Règle d'or** : le Web ne parle JAMAIS directement à Discord.
Tout transite par notre API. La DB Postgres est la source unique de vérité.

---

## 🔄 Le pattern de sync en 5 étapes

Pour qu'une carte Discord avec boutons soit synchronisée avec la web :

| # | Étape | Où |
|---|---|---|
| 1 | **Bot poste l'embed** Discord avec boutons | `bots/sentinel-bot/src/modules/{feature}/` |
| 2 | **Bot enregistre le mapping** `(action_id ↔ message_id)` via `register_action_message` | helper `crate::sync` |
| 3 | **Web déclenche une action** → `POST /api/{feature}/.../resolve` | `apps/web/src/services/` |
| 4 | **API broadcast** un event Redis + WS avec `actor.source = "web"` | `state.broadcaster.broadcast(...)` |
| 5 | **Bot listener** récupère le mapping, édite l'embed Discord (gris + footer « via web ») | `tokio::spawn(listen_stream_group(...))` |

**Anti-boucle** : si `actor.source != "web"`, le listener skip.

---

## 📂 Fichiers clés

### Côté API (Rust)

| Rôle | Fichier |
|---|---|
| Table mapping `discord_action_messages` | `services/api/migrations/175_discord_action_messages.sql` |
| Entity + kinds | `services/api/src/domain/entities/discord_action_message.rs` |
| Port inbound (use case) | `services/api/src/ports/inbound/manage_discord_action_messages.rs` |
| Service | `services/api/src/application/manage_discord_action_messages_service.rs` |
| Handler HTTP | `services/api/src/adapters/inbound/http/handlers/discord_action_messages.rs` |
| Broadcaster (Redis + WS) | `services/api/src/adapters/inbound/ws/broadcaster.rs` |

### Côté Bot (Rust)

| Rôle | Fichier |
|---|---|
| Helper `register_action_message` + `kinds` + `build_action_custom_id` | `bots/sentinel-bot/src/sync.rs` |
| Helper `listen_stream_group` | `shared/sentinel-shared/src/event_bus.rs` |

### Côté Web (Vue/TS)

| Rôle | Fichier |
|---|---|
| HTTP client | `apps/web/src/api/http.ts` |
| Event bus WS local | `apps/web/src/api/events.ts` (`onWsEvent("ws:<event_name>", cb)`) |

---

## ✅ Features synchronisées (5)

| Feature | `kind` | Event API | Bot listener | Web subscribe |
|---|---|---|---|---|
| **Tickets** (close from web) | `ticket` | `ticket_closed` | `tickets/mod.rs::handle_redis_event` | `useTickets.ts` |
| **Automod Review** (Apply/Ignore) | `automod_review` | `automod_review_resolved` | `automod/review.rs::handle_redis_event` | `AutomodPage.vue` |
| **Voice Admin Panel** (Lock/Hide/Transfer) | `voice_panel` | `voice_channel_updated/closed` | `voice/handlers/voice/channel_lifecycle.rs::handle_voice_redis_event` | `VoiceChannelsPage.vue` |
| **Blackjack Tables** (close from web) | `blackjack_table` | `blackjack_table_closed` | `blackjack/table.rs::handle_redis_event` | `BlackjackPage.vue` |
| **Coude Combats** (cancel from web) | `combat_challenge` | `coude_combat_cancelled` | `coude/mod.rs::handle_combat_redis_event` | (via cancel endpoint) |

---

## 🛠️ Comment ajouter une nouvelle feature sync

### 1. Si pas encore de table DB pour ta feature → créer

```sql
-- migrations/XYZ_my_feature.sql
CREATE TABLE my_feature_cards (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    -- ... champs métier ...
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

### 2. Domain + ports + service (architecture hexagonale)

- `domain/entities/my_feature.rs` : entity
- `ports/outbound/my_feature_repository.rs` : trait repo
- `ports/inbound/manage_my_feature.rs` : use case trait
- `application/manage_my_feature_service.rs` : implémentation
- `adapters/outbound/postgres/my_feature_repository.rs` : adapter Postgres

**Wiring** : ajouter au `bootstrap.rs` + `state.rs`.

### 3. Endpoints HTTP

```rust
// handlers/my_feature.rs
pub async fn resolve_card(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<ResolveBody>,
) -> Result<Json<...>, ApiError> {
    let card = state.my_feature_uc.resolve(...).await?;

    // ─── Étape clé : broadcast pour sync bot + web ───
    state.broadcaster.broadcast(
        "my_feature_resolved",
        serde_json::json!({
            "action_id": card.id.to_string(),
            "actor": { "source": "web", "id": ..., "name": ... },
            // ... état métier ...
        }),
    );

    Ok(Json(card.into()))
}
```

### 4. Côté Bot — register au moment du post

```rust
// Dans le module qui poste l'embed
let posted = channel.send_message(&ctx.http, msg).await?;

// Récupère le UUID métier (créé via API)
let card_uuid = api.create_my_card(...).await?.id;

// Sync : enregistre le mapping
crate::sync::register_action_message(
    &api_arc,
    card_uuid,
    "my_feature",  // ← ajoute la const dans sync.rs::kinds
    &guild_id_str,
    &channel_id_str,
    &posted.id.to_string(),
).await;
```

### 5. Côté Bot — listener Redis

```rust
// Au ready()
fn spawn_my_feature_sync_listener(ctx: Context) {
    tokio::spawn(async move {
        let consumer = sentinel_shared::event_bus::default_consumer_name();
        sentinel_shared::event_bus::listen_stream_group(
            "my-bot-sync".to_string(),  // ← consumer group unique
            consumer,
            move |payload| {
                let ctx = ctx.clone();
                async move { handle_my_feature_event(&ctx, &payload).await; }
            },
        ).await;
    });
}

async fn handle_my_feature_event(ctx: &Context, payload: &str) {
    let event: serde_json::Value = serde_json::from_str(payload).ok()?;
    if event["event"] != "my_feature_resolved" { return; }

    // Anti-boucle : skip si event vient de Discord (pas du web)
    if event["data"]["actor"]["source"] != "web" { return; }

    let action_id = event["data"]["action_id"].as_str()?;

    // Fetch le mapping pour retrouver channel_id + message_id
    let mappings: Vec<Mapping> = api
        .get_json(&format!("/api/discord-messages/{action_id}"))
        .await?;
    let m = mappings.into_iter().find(|m| m.kind == "my_feature")?;

    // Édite l'embed Discord (gris + footer "via web" + retire boutons)
    channel_id.edit_message(&ctx.http, msg_id,
        EditMessage::new()
            .embed(new_embed.color(0x95A5A6).footer("...via web"))
            .components(vec![])
    ).await?;
}
```

### 6. Côté Web — subscribe + UI

```ts
// services/myFeatureService.ts
export const myFeatureService = {
  listPending(guildId) { return httpGet(`/api/my-feature/${guildId}/pending`); },
  resolve(id, body)    { return httpPost(`/api/my-feature/${id}/resolve`, body); },
};
```

```vue
<!-- Dans la page -->
<script setup lang="ts">
import { onMounted, onUnmounted } from "vue";
import { on as onWsEvent } from "@/api/events";

const offResolved = onWsEvent("ws:my_feature_resolved", () => fetchCards());
onUnmounted(() => offResolved());
</script>
```

---

## 🐛 Debugging — checklist quand ça ne sync pas

1. **Migration appliquée ?** `psql -c "\dt automod_reviews"` (ou ta table).
2. **Mapping enregistré ?** `psql -c "SELECT * FROM discord_action_messages WHERE kind='...'"`.
3. **Event publié côté API ?** Logs Redis : `redis-cli XRANGE sentinel:events - +`.
4. **Bot consomme bien le stream ?** Vérifier qu'un consumer group unique a été choisi (pas de collision avec un autre listener).
5. **Anti-boucle correct ?** L'event publié depuis le bot doit avoir `source != "web"` ; celui du web doit avoir `source = "web"`.
6. **Permissions Discord** : le bot a-t-il `MANAGE_MESSAGES` sur le channel ? Sinon `edit_message` échoue silencieusement.
7. **Mapping perdu après redéploiement** : si une carte a été postée AVANT le déploiement de `register_action_message`, elle ne sera pas synchronisable. Acceptable (legacy).

---

## ⚠️ Pièges classiques

- **Ne jamais** bypasser l'API depuis le web ou le bot — toute écriture passe par les use cases.
- **Ne jamais** publier un event sans `actor.source` → le listener ne peut pas appliquer l'anti-boucle.
- **Toujours** utiliser un consumer group Redis unique par feature/bot (`automod-bot`, `voice-bot-sync`, `coude-bot-combat-sync`, etc.) — sinon les messages sont distribués entre listeners et certains sont perdus.
- **Idempotence** : la résolution d'une carte doit être un `UPDATE WHERE status='pending'` puis tester si la ligne a été affectée. Sinon double-clic = double action.
- **Ne pas** stocker un message Discord supprimé manuellement : ajouter un handler `MESSAGE_DELETE` qui appelle `DELETE /api/discord-messages/{action_id}/{kind}` (cf. `find_by_message`).

---

## 📚 Docs liées

- `SYNC_DISCORD_WEB_DESIGN.md` — design technique détaillé (transport, formats, idempotence)
- `WEB_ADMIN_GAPS.md` — audit des manques web admin (Q1 2026)
- `ROADMAP.md` — plan 12 semaines (achevé)

---

*Réfère-toi à ce guide à chaque fois que tu ajoutes un nouvel embed Discord avec boutons. Si la feature n'est pas dans le tableau « Features synchronisées », elle n'est pas sync — c'est ta chance de l'ajouter en suivant le pattern.*
