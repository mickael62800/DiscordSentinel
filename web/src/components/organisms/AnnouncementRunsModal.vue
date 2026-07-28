<script setup lang="ts">
import type { ScheduledAnnouncement, AnnouncementRun } from "@/services/announcementsService";
import AppModal from "../atoms/AppModal.vue";
import { useFormatDate } from "@/composables/useFormatDate";

const { formatDateTimeShort } = useFormatDate();

defineProps<{
  target: ScheduledAnnouncement | null;
  runs: AnnouncementRun[];
}>();

const emit = defineEmits<{ close: [] }>();

function fmtDate(iso: string | null): string {
  if (!iso) return "—";
  return formatDateTimeShort(iso);
}
</script>

<template>
  <AppModal
    :visible="!!target"
    :title="target ? `📜 Historique — ${target.name}` : ''"
    size="md"
    @close="emit('close')"
  >
    <table v-if="runs.length > 0" class="data-table">
      <thead>
        <tr>
          <th>Date</th>
          <th>Statut</th>
          <th>Salons</th>
          <th>Erreur</th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="r in runs" :key="r.id">
          <td class="small mono">{{ fmtDate(r.ran_at) }}</td>
          <td>
            <span class="badge" :class="`status-${r.status}`">{{ r.status }}</span>
          </td>
          <td class="small">
            {{ r.channels_posted.filter((c) => c.success).length }}/{{ r.channels_posted.length }} OK
          </td>
          <td class="small muted">{{ r.error ?? "—" }}</td>
        </tr>
      </tbody>
    </table>
    <p v-else class="muted">Aucun envoi pour le moment.</p>
  </AppModal>
</template>

<style scoped>
.muted { color: var(--text-secondary); }
.small { font-size: 12px; }
.mono { font-family: "JetBrains Mono", monospace; }

.data-table { width: 100%; border-collapse: collapse; font-size: 13px; }
.data-table th, .data-table td {
  padding: 10px 12px;
  border-bottom: 1px solid var(--border);
}
.data-table th {
  text-align: left;
  font-size: 11px;
  text-transform: uppercase;
  color: var(--text-secondary);
  letter-spacing: 0.5px;
}

.badge {
  display: inline-block;
  padding: 2px 6px;
  border-radius: 4px;
  background: var(--bg-secondary);
  color: var(--text-secondary);
  font-size: 10px;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}
.badge.status-success { background: rgba(46, 204, 113, 0.15); color: #2ecc71; }
.badge.status-partial { background: rgba(241, 196, 15, 0.15); color: #f1c40f; }
.badge.status-error { background: rgba(231, 76, 60, 0.15); color: #e74c3c; }
.badge.status-pending { background: rgba(138, 150, 168, 0.15); color: var(--text-secondary); }
</style>
