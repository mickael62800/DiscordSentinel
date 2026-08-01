<script setup lang="ts">
import { useAutomod } from "@/composables/useAutomod";
import { useFormatDate } from "@/composables/useFormatDate";

const { detections, loading, userFilter, fetchDetections } = useAutomod();
const { formatDateTimeNumeric: formatDate } = useFormatDate();

function severityLabel(s: number): { label: string; color: string } {
  if (s >= 8) return { label: "Critique", color: "#E74C3C" };
  if (s >= 5) return { label: "Élevée", color: "#E67E22" };
  if (s >= 2) return { label: "Moyenne", color: "#F1C40F" };
  return { label: "Faible", color: "#7F8C8D" };
}
</script>

<template>
  <section class="card timeline">
    <div class="timeline-header">
      <h2>Timeline des détections</h2>
      <div class="filters">
        <input
          v-model="userFilter"
          placeholder="Filtrer par user ID"
          @keyup.enter="fetchDetections"
        />
        <button class="btn-secondary" @click="fetchDetections">Filtrer</button>
      </div>
    </div>

    <div v-if="loading" class="loading">Chargement…</div>
    <div v-else-if="detections.length === 0" class="empty">
      Aucune détection à afficher.
    </div>
    <table v-else class="detections-table">
      <thead>
        <tr>
          <th>Date</th>
          <th>Utilisateur</th>
          <th>Raison</th>
          <th>Sévérité</th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="d in detections" :key="d.id">
          <td>{{ formatDate(d.created_at) }}</td>
          <td>
            <strong>{{ d.username }}</strong>
            <small class="muted">{{ d.user_id }}</small>
          </td>
          <td class="reason">{{ d.reason }}</td>
          <td>
            <span
              class="severity-badge"
              :style="{ backgroundColor: severityLabel(d.score ?? 0).color }"
            >
              {{ severityLabel(d.score ?? 0).label }}
            </span>
            <small class="muted">{{ (d.score ?? 0).toFixed(1) }}</small>
          </td>
        </tr>
      </tbody>
    </table>
  </section>
</template>

<style scoped>
.card {
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  padding: 20px;
}
.card h2 { margin: 0; font-size: 1.1rem; }

.timeline-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 12px;
}

.filters { display: flex; gap: 8px; }
.filters input {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  padding: 6px 10px;
  color: inherit;
}

.btn-secondary {
  background: var(--accent);
  color: white;
  border: none;
  border-radius: var(--radius-sm);
  padding: 6px 14px;
  cursor: pointer;
}

.loading { padding: 32px; text-align: center; color: var(--text-secondary); }
.empty { color: var(--text-secondary); font-style: italic; }

.detections-table {
  width: 100%;
  border-collapse: collapse;
}
.detections-table th, .detections-table td {
  text-align: left;
  padding: 8px 10px;
  border-bottom: 1px solid var(--border);
  vertical-align: top;
}
.detections-table th {
  font-size: 0.85rem;
  font-weight: 600;
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.05em;
}
.detections-table small.muted {
  display: block;
  font-size: 0.75rem;
  color: var(--text-secondary);
  margin-top: 2px;
}
.muted { color: var(--text-secondary); }

.severity-badge {
  display: inline-block;
  padding: 2px 8px;
  border-radius: var(--radius-lg);
  color: white;
  font-size: 0.75rem;
  font-weight: 600;
}

.reason {
  max-width: 480px;
  word-break: break-word;
}

@media (max-width: 640px) {
  .timeline-header {
    flex-direction: column;
    align-items: stretch;
    gap: 10px;
  }
  .filters {
    flex-direction: column;
    width: 100%;
  }
  .filters input { width: 100%; }
}
</style>
