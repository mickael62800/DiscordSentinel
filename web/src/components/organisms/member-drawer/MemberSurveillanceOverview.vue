<script setup lang="ts">
import type { UserActivity, UserDossier } from "../../../types";
import {
  formatMemberDate as formatDate,
  countByCategory,
  voiceHours,
  attachmentCounts,
  automodCount,
  burstCount,
  watchSplitStats,
} from "../../../utils/memberActivity";

const props = defineProps<{
  activity: UserActivity[];
  dossier: UserDossier | null;
}>();

function cByCat(days: number, cat: "text" | "vocal" | "other") { return countByCategory(props.activity, days, cat); }
function vHours(days: number) { return voiceHours(props.activity, days); }
function attCounts() { return attachmentCounts(props.activity); }
function autoCount() { return automodCount(props.dossier); }
function bursts() { return burstCount(props.activity); }
function splitStats() { return watchSplitStats(props.dossier); }
</script>

<template>
  <div v-if="activity && activity.length > 0" class="section watch-summary">
    <h3>📊 Vue d'ensemble</h3>
    <div class="watch-stats-grid">
      <div class="watch-stat-card">
        <span class="watch-stat-label">Messages</span>
        <div class="watch-stat-multi">
          <span><strong>{{ cByCat(0, 'text') }}</strong> total</span>
          <span class="muted">{{ cByCat(30, 'text') }} · 30j</span>
          <span class="muted">{{ cByCat(7, 'text') }} · 7j</span>
        </div>
      </div>
      <div class="watch-stat-card">
        <span class="watch-stat-label">Heures vocales</span>
        <div class="watch-stat-multi">
          <span><strong>{{ vHours(0) }}h</strong> total</span>
          <span class="muted">{{ vHours(30) }}h · 30j</span>
          <span class="muted">{{ vHours(7) }}h · 7j</span>
        </div>
      </div>
      <div class="watch-stat-card">
        <span class="watch-stat-label">Pièces jointes</span>
        <div class="watch-stat-multi">
          <span>📷 <strong>{{ attCounts().images }}</strong></span>
          <span>🎬 <strong>{{ attCounts().videos }}</strong></span>
          <span>📎 <strong>{{ attCounts().files }}</strong></span>
          <span>🔗 <strong>{{ attCounts().links }}</strong></span>
        </div>
      </div>
      <div class="watch-stat-card">
        <span class="watch-stat-label">Modération</span>
        <div class="watch-stat-multi">
          <span><strong>{{ dossier?.infractions.length ?? 0 }}</strong> infractions</span>
          <span class="muted">🤖 {{ autoCount() }} automod</span>
          <span class="muted">⚡ {{ bursts() }} burst{{ bursts() > 1 ? 's' : '' }} (10msg/60s)</span>
        </div>
      </div>
      <div v-if="splitStats()" class="watch-stat-card">
        <span class="watch-stat-label">Sous surveillance depuis</span>
        <div class="watch-stat-multi">
          <span><strong>{{ formatDate(dossier?.user.first_seen_at as string ?? null) }}</strong></span>
          <span class="muted">Avant : {{ splitStats()?.beforeIncidents ?? 0 }} incident(s)</span>
          <span class="muted">Depuis : {{ splitStats()?.afterIncidents ?? 0 }} incident(s)</span>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.section { margin-bottom: 20px; }
.section h3 { margin: 0 0 10px 0; font-size: 14px; font-weight: 600; }

.watch-summary { margin-bottom: 16px; }
.watch-stats-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
  gap: 12px;
  margin-top: 8px;
}
.watch-stat-card {
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 10px 14px;
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.watch-stat-label {
  font-size: 11px;
  font-weight: 700;
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.4px;
}
.watch-stat-multi { display: flex; flex-direction: column; gap: 2px; font-size: 13px; }
.watch-stat-multi .muted { color: var(--text-secondary); font-size: 11px; }
</style>
