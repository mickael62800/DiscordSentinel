<script setup lang="ts">
import { ref, computed, watch } from "vue";
import type { UserActivity } from "../../../types";
import { useFormatDate } from "../../../composables/useFormatDate";
import { safeImageUrl } from "../../../utils/safeUrl";
import {
  eventCategory,
  activityCount,
  activityLinksCount,
  activityAttachmentsCount,
  activityLabel,
  activityVariant,
  voiceChannelLabel,
  editedBeforeAfter,
  rolesDiff,
  profileDiff,
  avatarDiff,
} from "../../../utils/memberActivity";
import AppBadge from "../../atoms/AppBadge.vue";
import PaginationBar from "../../molecules/PaginationBar.vue";

const props = defineProps<{ activity: UserActivity[] }>();

const { formatShortDateTime: fmt } = useFormatDate();

const activityTypeFilter = ref<"all" | "text" | "vocal" | "other">("all");
const activityDateFrom = ref<string>("");
const activityDateTo = ref<string>("");

const filteredActivity = computed(() => {
  const list = props.activity ?? [];
  const fromTs = activityDateFrom.value ? new Date(activityDateFrom.value).getTime() : null;
  const toTs = activityDateTo.value ? new Date(activityDateTo.value).getTime() + 86400000 : null;
  return list.filter((e) => {
    if (activityTypeFilter.value !== "all" && eventCategory(e.event_type) !== activityTypeFilter.value) return false;
    const ts = new Date(e.created_at).getTime();
    if (fromTs !== null && ts < fromTs) return false;
    if (toTs !== null && ts >= toTs) return false;
    return true;
  });
});

function resetActivityFilters() {
  activityTypeFilter.value = "all";
  activityDateFrom.value = "";
  activityDateTo.value = "";
  activityPage.value = 1;
}

const activityPage = ref(1);
const activityPerPage = ref(25);
const activityTotalPages = computed(() => Math.max(1, Math.ceil(filteredActivity.value.length / activityPerPage.value)));
const activityPageRows = computed(() => {
  const start = (activityPage.value - 1) * activityPerPage.value;
  return filteredActivity.value.slice(start, start + activityPerPage.value);
});

watch([activityTypeFilter, activityDateFrom, activityDateTo], () => { activityPage.value = 1; });
watch(activityTotalPages, (n) => { if (activityPage.value > n) activityPage.value = n; });

function actCount(type: string) { return activityCount(props.activity ?? [], type); }
function linksCount() { return activityLinksCount(props.activity ?? []); }
function attachmentsCount() { return activityAttachmentsCount(props.activity ?? []); }
</script>

<template>
  <div class="section">
    <h3>Activite recente ({{ filteredActivity.length }} / {{ activity.length }})</h3>

    <div class="activity-stats">
      <span><strong>{{ actCount('message_sent') }}</strong> messages</span>
      <span><strong>{{ actCount('voice_join') }}</strong> entrees vocal</span>
      <span><strong>{{ actCount('voice_leave') }}</strong> sorties vocal</span>
      <span><strong>{{ actCount('voice_move') }}</strong> moves</span>
      <span><strong>{{ actCount('message_deleted') }}</strong> supprimes</span>
      <span><strong>{{ actCount('message_edited') }}</strong> edites</span>
      <span><strong>{{ linksCount() }}</strong> liens</span>
      <span><strong>{{ attachmentsCount() }}</strong> pieces jointes</span>
    </div>

    <div class="activity-filters">
      <div class="activity-filter-group">
        <button
          v-for="t in (['all', 'text', 'vocal', 'other'] as const)"
          :key="t"
          type="button"
          :class="['activity-chip', { active: activityTypeFilter === t }]"
          @click="activityTypeFilter = t"
        >
          {{ t === 'all' ? 'Tout' : t === 'text' ? 'Texte' : t === 'vocal' ? 'Vocal' : 'Autre' }}
        </button>
      </div>
      <div class="activity-filter-group">
        <label class="activity-date-label">
          Du
          <input v-model="activityDateFrom" type="date" class="activity-date-input" />
        </label>
        <label class="activity-date-label">
          Au
          <input v-model="activityDateTo" type="date" class="activity-date-input" />
        </label>
        <button
          v-if="activityTypeFilter !== 'all' || activityDateFrom || activityDateTo"
          type="button"
          class="activity-reset"
          @click="resetActivityFilters"
        >
          Reset
        </button>
      </div>
    </div>

    <div v-if="filteredActivity.length === 0" class="empty-small">
      Aucun evenement ne correspond aux filtres.
    </div>
    <div v-for="evt in activityPageRows" :key="evt.id" class="detail-row">
      <div class="detail-row-header">
        <span class="detail-date">{{ fmt(evt.created_at) }}</span>
        <div class="header-badges">
          <AppBadge :label="activityLabel(evt.event_type)" :variant="activityVariant(evt.event_type)" />
          <span v-if="evt.event_type.startsWith('voice_')" class="activity-channel">
            {{ voiceChannelLabel(evt) }}
          </span>
          <span v-else-if="evt.channel_name" class="activity-channel">
            #{{ evt.channel_name }}<span v-if="evt.channel_id" class="channel-id">({{ evt.channel_id }})</span>
          </span>
          <span v-else-if="evt.channel_id" class="activity-channel">
            #{{ evt.channel_id }}
          </span>
        </div>
      </div>

      <template v-if="evt.event_type === 'message_edited'">
        <div class="diff-block">
          <div class="diff-row diff-before">
            <span class="diff-label">Avant :</span>
            <span v-if="editedBeforeAfter(evt).before" class="diff-content">{{ editedBeforeAfter(evt).before }}</span>
            <span v-else class="diff-content diff-missing">
              <em>(message non disponible — n'était pas dans le cache du bot au moment de la modif)</em>
            </span>
          </div>
          <div class="diff-row diff-after">
            <span class="diff-label">Après :</span>
            <span v-if="editedBeforeAfter(evt).after" class="diff-content">{{ editedBeforeAfter(evt).after }}</span>
            <span v-else class="diff-content diff-missing">
              <em>(contenu vide)</em>
            </span>
          </div>
        </div>
      </template>

      <template v-else-if="evt.event_type === 'roles_changed'">
        <div class="diff-block">
          <div v-if="rolesDiff(evt).added.length > 0" class="diff-row diff-after">
            <span class="diff-label">+ Ajoutés :</span>
            <span class="diff-content">{{ rolesDiff(evt).added.join(', ') }}</span>
          </div>
          <div v-if="rolesDiff(evt).removed.length > 0" class="diff-row diff-before">
            <span class="diff-label">− Retirés :</span>
            <span class="diff-content">{{ rolesDiff(evt).removed.join(', ') }}</span>
          </div>
          <div
            v-if="rolesDiff(evt).added.length === 0 && rolesDiff(evt).removed.length === 0 && evt.content"
            class="detail-row-body"
          >{{ evt.content }}</div>
        </div>
      </template>

      <template v-else-if="evt.event_type === 'nickname_changed'">
        <div class="diff-block">
          <div v-if="profileDiff(evt).before" class="diff-row diff-before">
            <span class="diff-label">Ancien pseudo :</span>
            <span class="diff-content">{{ profileDiff(evt).before }}</span>
          </div>
          <div v-if="profileDiff(evt).after" class="diff-row diff-after">
            <span class="diff-label">Nouveau :</span>
            <span class="diff-content">{{ profileDiff(evt).after }}</span>
          </div>
        </div>
      </template>

      <template v-else-if="evt.event_type === 'avatar_changed'">
        <div class="avatar-diff">
          <div v-if="safeImageUrl(avatarDiff(evt).before)" class="avatar-cell">
            <span class="diff-label">Avant</span>
            <img :src="safeImageUrl(avatarDiff(evt).before) ?? ''" alt="ancien avatar" class="avatar-thumb" />
          </div>
          <span v-if="safeImageUrl(avatarDiff(evt).before) && safeImageUrl(avatarDiff(evt).after)" class="diff-arrow">→</span>
          <div v-if="safeImageUrl(avatarDiff(evt).after)" class="avatar-cell">
            <span class="diff-label">Après</span>
            <img :src="safeImageUrl(avatarDiff(evt).after) ?? ''" alt="nouvel avatar" class="avatar-thumb" />
          </div>
          <span v-if="!avatarDiff(evt).before && !avatarDiff(evt).after && evt.content" class="detail-row-body">{{ evt.content }}</span>
        </div>
      </template>

      <template v-else>
        <div v-if="evt.content" class="detail-row-body">{{ evt.content }}</div>
      </template>
    </div>

    <PaginationBar
      v-if="filteredActivity.length > 0"
      :current-page="activityPage"
      :total-pages="activityTotalPages"
      :total-items="filteredActivity.length"
      :per-page="activityPerPage"
      @update:current-page="activityPage = $event"
      @update:per-page="(n) => { activityPerPage = n; activityPage = 1; }"
    />
  </div>
</template>

<style scoped>
.section { margin-bottom: 20px; }
.section h3 { margin: 0 0 10px 0; font-size: 14px; font-weight: 600; }
.empty-small { color: var(--text-secondary); text-align: center; padding: 20px; font-size: 13px; }

.detail-row {
  background: var(--bg-secondary);
  border-radius: 8px;
  padding: 10px 14px;
  margin-bottom: 6px;
}
.detail-row-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 8px;
  margin-bottom: 4px;
}
.header-badges {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-left: auto;
  flex-wrap: wrap;
  justify-content: flex-end;
}
.detail-date {
  font-size: 11px;
  color: var(--text-secondary);
  font-family: "JetBrains Mono", "Cascadia Code", monospace;
  flex-shrink: 0;
}
.detail-row-body { font-size: 13px; color: var(--text-primary); white-space: pre-wrap; word-break: break-word; }

.diff-block { display: flex; flex-direction: column; gap: 4px; font-size: 13px; margin-top: 4px; }
.diff-row {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  padding: 6px 10px;
  border-radius: 6px;
  border-left: 3px solid transparent;
}
.diff-before {
  background: color-mix(in srgb, var(--danger) 8%, transparent);
  border-left-color: color-mix(in srgb, var(--danger) 70%, transparent);
}
.diff-after {
  background: color-mix(in srgb, var(--success) 8%, transparent);
  border-left-color: color-mix(in srgb, var(--success) 70%, transparent);
}
.diff-label {
  font-size: 11px;
  font-weight: 700;
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.3px;
  flex-shrink: 0;
  min-width: 90px;
}
.diff-content { color: var(--text-primary); white-space: pre-wrap; word-break: break-word; }
.diff-missing { color: var(--text-secondary); font-style: italic; }

.avatar-diff { display: flex; align-items: center; gap: 12px; margin-top: 6px; }
.avatar-cell { display: flex; flex-direction: column; align-items: center; gap: 4px; }
.avatar-thumb {
  width: 56px;
  height: 56px;
  border-radius: 50%;
  border: 1px solid var(--border);
  object-fit: cover;
}
.diff-arrow { font-size: 20px; color: var(--text-secondary); }

.channel-id { margin-left: 4px; font-size: 10px; opacity: 0.7; }

.activity-stats {
  display: flex;
  flex-wrap: wrap;
  gap: 8px 16px;
  padding: 10px 12px;
  margin-bottom: 12px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  font-size: 12px;
  color: var(--text-secondary);
}
.activity-stats strong { color: var(--text-primary); font-weight: 700; margin-right: 2px; }
.activity-channel {
  font-size: 11px;
  color: var(--text-secondary);
  font-family: "JetBrains Mono", monospace;
  background: var(--bg-hover);
  padding: 1px 6px;
  border-radius: var(--radius-sm);
}

.activity-filters { display: flex; flex-wrap: wrap; gap: 12px; align-items: center; margin-bottom: 12px; }
.activity-filter-group { display: flex; align-items: center; gap: 6px; flex-wrap: wrap; }

.activity-chip {
  padding: 5px 12px;
  border-radius: var(--radius-sm);
  background: var(--bg-card);
  border: 1px solid var(--border);
  color: var(--text-secondary);
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  transition: all var(--transition-fast);
}
.activity-chip:hover { color: var(--text-primary); background: var(--bg-hover); }
.activity-chip.active { background: var(--accent); border-color: var(--accent); color: white; }

.activity-date-label {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  font-size: 11px;
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.6px;
}
.activity-date-input {
  padding: 8px 12px;
  border: 1px solid var(--border);
  border-radius: 6px;
  background: var(--bg-card);
  color: var(--text-primary);
  font-size: 13px;
}
.activity-date-input:focus { outline: none; border-color: var(--accent); }

.activity-reset {
  padding: 4px 10px;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--text-secondary);
  font-size: 11px;
  font-weight: 600;
  cursor: pointer;
}
.activity-reset:hover { color: var(--danger); border-color: var(--danger); }
</style>
