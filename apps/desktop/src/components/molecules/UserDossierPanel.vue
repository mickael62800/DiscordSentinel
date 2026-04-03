<script setup lang="ts">
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useFormatDate } from "../../composables/useFormatDate";
import AppBadge from "../atoms/AppBadge.vue";
import DataTable from "../organisms/DataTable.vue";
import ActivityTimeline from "./ActivityTimeline.vue";
import type { TableColumn, WatchedUser, UserActivity } from "../../types";
import { actionVariant, severityVariant } from "../../utils/variants";

const props = defineProps<{
  user: WatchedUser | null;
  dossier: any | null;
  dossierLoading: boolean;
  activities: UserActivity[];
  activitiesLoading: boolean;
}>();

const emit = defineEmits<{
  close: [];
  removed: [];
}>();

const removing = ref(false);

async function removeFromWatch() {
  if (!props.user) return;
  if (!confirm(`Retirer ${props.user.username} de la surveillance ?`)) return;
  removing.value = true;
  try {
    await invoke("remove_watched_user", {
      guildId: props.user.guild_id,
      userId: props.user.user_id,
    });
    emit("removed");
    emit("close");
  } catch (e) {
    console.error("Erreur suppression surveillance:", e);
  } finally {
    removing.value = false;
  }
}

const { formatShortDateTime: fmt } = useFormatDate();

function riskLabel(level: string): string {
  switch (level) {
    case "critical": return "Critique";
    case "high": return "Eleve";
    case "medium": return "Moyen";
    case "low": return "Faible";
    default: return level;
  }
}

function totalInfractions(u: WatchedUser): number {
  return u.total_warns + u.total_mutes + u.total_bans;
}

function conductPercent(u: WatchedUser): number | null {
  if (u.conduct_points === null || u.max_conduct_points === null) return null;
  return Math.round((u.conduct_points / u.max_conduct_points) * 100);
}

const dossierInfractionColumns: TableColumn[] = [
  { key: "infraction_type", label: "Type" },
  { key: "reason", label: "Raison" },
  { key: "moderator", label: "Moderateur" },
  { key: "created_at", label: "Date" },
];

const dossierActionColumns: TableColumn[] = [
  { key: "action_type", label: "Action" },
  { key: "reason", label: "Raison" },
  { key: "target_name", label: "Cible" },
];

const dossierConductColumns: TableColumn[] = [
  { key: "delta", label: "Points" },
  { key: "reason", label: "Raison" },
  { key: "points_after", label: "Apres" },
  { key: "created_at", label: "Date" },
];
</script>

<template>
  <!-- Panneau dossier -->
  <div v-if="user" class="dossier-panel">
    <div class="dossier-header">
      <div class="dossier-title">
        <h2>Dossier : {{ user.username }}</h2>
        <AppBadge :label="riskLabel(user.risk_level)" :variant="severityVariant(user.risk_level)" />
      </div>
      <div class="dossier-actions">
        <button class="remove-watch-btn" :disabled="removing" @click="removeFromWatch">
          {{ removing ? 'Suppression...' : 'Retirer surveillance' }}
        </button>
        <button class="close-btn" @click="emit('close')">&times;</button>
      </div>
    </div>

    <div class="dossier-summary">
      <div class="summary-card">
        <span class="summary-value">{{ user.user_id }}</span>
        <span class="summary-label">ID Discord</span>
      </div>
      <div class="summary-card">
        <span class="summary-value">{{ user.guild_name }}</span>
        <span class="summary-label">Serveur</span>
      </div>
      <div class="summary-card">
        <span class="summary-value">{{ totalInfractions(user) }}</span>
        <span class="summary-label">Infractions</span>
      </div>
      <div class="summary-card">
        <span class="summary-value">{{ user.security_events_count }}</span>
        <span class="summary-label">Evt Securite</span>
      </div>
      <div v-if="conductPercent(user) !== null" class="summary-card">
        <span :class="['summary-value', { 'conduct-low': (conductPercent(user) ?? 0) < 30 }]">
          {{ user.conduct_points }} / {{ user.max_conduct_points }}
        </span>
        <span class="summary-label">Points de conduite</span>
      </div>
    </div>

    <div v-if="dossierLoading" class="loading">Chargement du dossier...</div>

    <template v-else-if="dossier">
      <!-- Infractions -->
      <section class="dossier-section">
        <h3>Infractions ({{ dossier.infractions.length }})</h3>
        <DataTable
          :columns="dossierInfractionColumns"
          :rows="(dossier.infractions as unknown as Record<string, unknown>[])"
          empty-message="Aucune infraction"
        >
          <template #cell-infraction_type="{ value }">
            <AppBadge :label="String(value)" :variant="actionVariant(String(value))" />
          </template>
          <template #cell-created_at="{ value }">
            <span class="mono">{{ fmt(String(value)) }}</span>
          </template>
        </DataTable>
      </section>

      <!-- Actions de moderation -->
      <section class="dossier-section">
        <h3>Actions de moderation ({{ dossier.moderation_actions.length }})</h3>
        <DataTable
          :columns="dossierActionColumns"
          :rows="(dossier.moderation_actions as unknown as Record<string, unknown>[])"
          empty-message="Aucune action"
        >
          <template #cell-action_type="{ value }">
            <AppBadge :label="String(value)" :variant="actionVariant(String(value))" />
          </template>
        </DataTable>
      </section>

      <!-- Evenements de securite -->
      <section v-if="dossier.security_events.length > 0" class="dossier-section">
        <h3>Evenements de securite ({{ dossier.security_events.length }})</h3>
        <div class="security-events">
          <div v-for="evt in dossier.security_events" :key="evt.id" class="security-event-item">
            <AppBadge :label="evt.severity" :variant="severityVariant(evt.severity)" />
            <span class="event-type">{{ evt.event_type.replace("_", " ") }}</span>
            <span class="event-desc">{{ evt.description }}</span>
            <span class="mono event-date">{{ fmt(evt.created_at) }}</span>
          </div>
        </div>
      </section>

      <!-- Historique conduite -->
      <section v-if="dossier.conduct_log.length > 0" class="dossier-section">
        <h3>Historique de conduite ({{ dossier.conduct_log.length }})</h3>
        <DataTable
          :columns="dossierConductColumns"
          :rows="(dossier.conduct_log as unknown as Record<string, unknown>[])"
          empty-message="Aucun historique"
        >
          <template #cell-delta="{ value }">
            <span :class="['delta', Number(value) < 0 ? 'delta-neg' : 'delta-pos']">
              {{ Number(value) > 0 ? '+' : '' }}{{ value }}
            </span>
          </template>
          <template #cell-created_at="{ value }">
            <span class="mono">{{ fmt(String(value)) }}</span>
          </template>
        </DataTable>
      </section>

      <!-- Timeline d'activite -->
      <section class="dossier-section">
        <h3>Timeline d'activite ({{ activities.length }})</h3>
        <ActivityTimeline :activities="activities" :loading="activitiesLoading" />
      </section>
    </template>
  </div>

  <!-- Placeholder quand aucun user selectionne -->
  <div v-else class="dossier-placeholder">
    <div class="placeholder-content">
      <svg class="placeholder-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
        <path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z" />
        <circle cx="12" cy="12" r="3" />
      </svg>
      <p>Selectionnez un utilisateur pour consulter son dossier</p>
    </div>
  </div>
</template>

<style scoped>
.dossier-panel {
  flex: 1;
  background-color: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 12px;
  padding: 24px;
  overflow-y: auto;
  max-height: calc(100vh - 200px);
}

.dossier-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 20px;
}

.dossier-title {
  display: flex;
  align-items: center;
  gap: 12px;
}

.dossier-title h2 {
  font-size: 18px;
  margin: 0;
}

.dossier-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}

.remove-watch-btn {
  background: var(--danger-bg);
  color: var(--danger);
  border: 1px solid var(--danger);
  border-radius: 8px;
  padding: 6px 14px;
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.15s;
}

.remove-watch-btn:hover {
  background: var(--danger);
  color: white;
}

.remove-watch-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.close-btn {
  width: 32px;
  height: 32px;
  background: none;
  border-radius: 8px;
  font-size: 20px;
  color: var(--text-secondary);
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
}

.close-btn:hover {
  background-color: var(--bg-hover);
  color: var(--text-primary);
}

.dossier-summary {
  display: flex;
  gap: 12px;
  flex-wrap: wrap;
  margin-bottom: 24px;
}

.summary-card {
  background-color: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: 10px;
  padding: 12px 16px;
  display: flex;
  flex-direction: column;
  gap: 4px;
  min-width: 120px;
}

.summary-value {
  font-weight: 700;
  font-size: 14px;
  color: var(--text-primary);
  word-break: break-all;
}

.summary-value.conduct-low {
  color: var(--danger);
}

.summary-label {
  font-size: 10px;
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.3px;
}

.dossier-section {
  margin-bottom: 24px;
}

.dossier-section h3 {
  font-size: 14px;
  font-weight: 600;
  margin-bottom: 12px;
  color: var(--text-primary);
}

.security-events {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.security-event-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 14px;
  background-color: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: 8px;
  font-size: 13px;
}

.event-type {
  font-weight: 600;
  text-transform: capitalize;
}

.event-desc {
  flex: 1;
  color: var(--text-secondary);
}

.event-date {
  font-size: 11px;
  color: var(--text-secondary);
}

.delta {
  font-weight: 700;
  font-family: "JetBrains Mono", "Cascadia Code", monospace;
}

.delta-neg { color: var(--danger); }
.delta-pos { color: var(--success); }

.dossier-placeholder {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  background-color: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 12px;
  min-height: 400px;
}

.placeholder-content {
  text-align: center;
  color: var(--text-secondary);
}

.placeholder-icon {
  width: 48px;
  height: 48px;
  margin-bottom: 12px;
  opacity: 0.4;
}

.placeholder-content p {
  font-size: 14px;
}

.loading {
  color: var(--text-secondary);
  padding: 40px;
  text-align: center;
}

.mono {
  font-family: "JetBrains Mono", "Cascadia Code", monospace;
}
</style>
