# Fix — Onglet Surveillance vide pour les entrées/sorties vocales, messages, liens, images

## Symptôme

Dans la page **Membres** → onglet **Surveillance**, on n'affiche que les
infractions, actions de modération, événements de sécurité et notes.
Aucune trace de l'activité réelle de l'utilisateur :

- entrées / sorties / déplacements vocaux
- messages envoyés (texte, contenu, salon)
- messages édités / supprimés
- changements de pseudo / d'avatar
- changements de rôles
- arrivées / départs serveur

## Diagnostic

### Backend — RIEN à changer

Toute la donnée est déjà collectée et exposée :

| Élément | Statut | Référence |
|---|---|---|
| Table SQL | ✅ peuplée | `user_activity_log` (`message_sent`, `voice_join`, `voice_leave`, `voice_move`, `message_deleted`, `message_edited`, `roles_changed`, `nickname_changed`, `avatar_changed`, `member_join`, `member_leave`) |
| Entité domain | ✅ existe | `services/api/src/domain/entities/user_activity.rs` |
| Repository | ✅ existe | `PgUserActivityRepository::list(guild_id, user_id, event_type?, limit, offset)` |
| Endpoint HTTP | ✅ existe | `GET /api/user-activity/{guild_id}/{user_id}` (`services/api/src/adapters/inbound/http/handlers/user_activity.rs:55`) |

Vérifié en BDD :

```text
event_type        | count
------------------+------
message_sent      | 1531
roles_changed     |  285
voice_move        |  169
voice_leave       |  154
voice_join        |  150
message_deleted   |   94
message_edited    |   74
nickname_changed  |   40
member_leave      |   26
member_join       |   22
avatar_changed    |    1
```

### Frontend — c'est ici qu'il manque tout

`UserDossier` ne contient pas de champ activité, et la tab Surveillance
ne fait aucun appel à `/api/user-activity`.

## Fichiers à modifier (frontend uniquement)

### 1. Nouveau service `apps/web/src/services/userActivityService.ts`

```ts
import { httpGet } from "@/api/http";
import { q } from "./_query";

export interface UserActivity {
  id: string;
  guild_id: string;
  user_id: string;
  event_type: string;
  channel_id: string | null;
  channel_name: string | null;
  content: string | null;
  metadata: Record<string, unknown>;
  created_at: string;
}

export const userActivityService = {
  list(
    guildId: string,
    userId: string,
    opts?: { eventType?: string; limit?: number; offset?: number },
  ): Promise<UserActivity[]> {
    return httpGet(
      `/api/user-activity/${guildId}/${userId}${q({
        event_type: opts?.eventType,
        limit: opts?.limit,
        offset: opts?.offset,
      })}`,
    );
  },
};
```

### 2. Étendre `useMembers` (`apps/web/src/composables/useMembers.ts`)

- Ajouter un `ref<UserActivity[]>` à côté de `dossier`.
- Charger en parallèle dans `fetchDossier(userId)` :

```ts
const [dossierData, activity] = await Promise.all([
  watchedUsersService.getDossier(guildId, userId),
  userActivityService.list(guildId, userId, { limit: 100 }),
]);
dossier.value = dossierData;
activityTimeline.value = activity;
```

- Exposer `activityTimeline` dans le `return`.

### 3. Afficher dans l'onglet Surveillance (`MembersPage.vue` ~ligne 525)

Ajouter avant la fermeture du `<template v-if="isWatched(...) && dossier">` :

```vue
<div v-if="activityTimeline?.length" class="section">
  <h3>Activite recente ({{ activityTimeline.length }})</h3>

  <!-- Compteurs rapides -->
  <div class="activity-stats">
    <span><strong>{{ countBy('message_sent') }}</strong> messages</span>
    <span><strong>{{ countBy('voice_join') }}</strong> entrees vocales</span>
    <span><strong>{{ countBy('voice_leave') }}</strong> sorties vocales</span>
    <span><strong>{{ countBy('message_deleted') }}</strong> supprimes</span>
    <span><strong>{{ countLinks() }}</strong> liens</span>
    <span><strong>{{ countAttachments() }}</strong> pieces jointes</span>
  </div>

  <!-- Timeline -->
  <div
    v-for="evt in activityTimeline.slice(0, 50)"
    :key="evt.id"
    class="detail-row"
  >
    <div class="detail-row-header">
      <span class="detail-date">{{ fmt(evt.created_at) }}</span>
      <AppBadge :label="labelFor(evt.event_type)" :variant="variantFor(evt.event_type)" />
      <span v-if="evt.channel_name" class="muted">#{{ evt.channel_name }}</span>
    </div>
    <div v-if="evt.content" class="detail-row-body">{{ evt.content }}</div>
  </div>
</div>
```

Helpers à mettre dans le `<script setup>` :

```ts
const URL_RE = /https?:\/\/[^\s]+/i;

function countBy(type: string): number {
  return (activityTimeline.value ?? []).filter(e => e.event_type === type).length;
}
function countLinks(): number {
  return (activityTimeline.value ?? []).filter(e => e.content && URL_RE.test(e.content)).length;
}
function countAttachments(): number {
  return (activityTimeline.value ?? []).filter(e => {
    const m = e.metadata as Record<string, unknown> | null;
    return Array.isArray(m?.attachments) && (m.attachments as unknown[]).length > 0;
  }).length;
}
function labelFor(t: string): string {
  return ({
    message_sent: "Message",
    message_edited: "Edite",
    message_deleted: "Supprime",
    voice_join: "Entree vocal",
    voice_leave: "Sortie vocal",
    voice_move: "Move vocal",
    roles_changed: "Roles",
    nickname_changed: "Pseudo",
    avatar_changed: "Avatar",
    member_join: "Arrivee",
    member_leave: "Depart",
  } as const)[t] ?? t;
}
function variantFor(t: string): "default" | "warning" | "danger" | "info" {
  if (t === "message_deleted") return "danger";
  if (t === "message_edited" || t === "voice_leave") return "warning";
  if (t.startsWith("voice_") || t === "message_sent") return "info";
  return "default";
}
```

## Validation

1. Mettre un user en surveillance.
2. Ouvrir son dossier → onglet Surveillance.
3. Section **Activite recente** doit afficher les compteurs (messages, vocaux, liens, PJ) et la timeline des 50 derniers événements.
4. DevTools → Network : un seul appel supplémentaire vers
   `GET /api/user-activity/{guild}/{user}?limit=100` (200 OK).

## Notes

- Pour les **images / liens**, il faut inspecter `metadata.attachments`
  (URLs Discord) et chercher `https?://` dans `content`. Le backend ne
  pré-classe pas, on filtre côté UI.
- Pour de gros volumes, prévoir une pagination (`offset` + bouton « Charger plus »).
- Une amélioration future : ajouter des filtres par `event_type` côté UI
  via le paramètre `event_type` déjà supporté par l'endpoint.
