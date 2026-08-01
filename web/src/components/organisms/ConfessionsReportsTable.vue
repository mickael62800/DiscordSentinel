<script setup lang="ts">
import { useConfessions } from "@/composables/useConfessions";
import { useFormatDate } from "@/composables/useFormatDate";

const { reports, resolveReport } = useConfessions();
const { formatDateTimeShort } = useFormatDate();

function fmtDate(iso: string | null): string {
  if (!iso) return "—";
  return formatDateTimeShort(iso);
}

function truncate(s: string, n = 80): string {
  return s.length > n ? s.slice(0, n) + "…" : s;
}
</script>

<template>
  <table v-if="reports.length > 0" class="data-table">
    <thead>
      <tr>
        <th>Date</th>
        <th>Cible</th>
        <th>Reporter</th>
        <th>Raison</th>
        <th>Status</th>
        <th class="actions-h">Actions</th>
      </tr>
    </thead>
    <tbody>
      <tr v-for="r in reports" :key="r.id">
        <td class="small mono">{{ fmtDate(r.created_at) }}</td>
        <td class="small mono">
          <span v-if="r.confession_id">Confession {{ r.confession_id.slice(0, 8) }}…</span>
          <span v-else-if="r.reply_id">Reply {{ r.reply_id.slice(0, 8) }}…</span>
        </td>
        <td class="small mono">{{ r.reporter_user_id }}</td>
        <td class="small">{{ truncate(r.reason, 80) }}</td>
        <td><span class="badge" :class="`status-${r.status}`">{{ r.status }}</span></td>
        <td class="actions">
          <button class="btn-secondary xs" @click="resolveReport(r, 'resolved')" title="Résoudre">✓</button>
          <button class="btn-secondary xs" @click="resolveReport(r, 'dismissed')" title="Rejeter">×</button>
        </td>
      </tr>
    </tbody>
  </table>
  <p v-else class="empty">Aucun signalement en attente 🎉</p>
</template>

<style scoped>
.small { font-size: 12px; }
.mono { font-family: "JetBrains Mono", monospace; }
.empty { padding: 30px; text-align: center; color: var(--text-secondary); font-style: italic; }
.data-table { width: 100%; border-collapse: collapse; }
.data-table th, .data-table td { padding: 10px 12px; border-bottom: 1px solid var(--border); }
.data-table th { text-align: left; font-size: 11px; text-transform: uppercase; color: var(--text-secondary); letter-spacing: .5px; }
.data-table .actions-h, .data-table .actions { text-align: right; white-space: nowrap; }
.data-table .actions button { margin-left: 4px; }
.badge { display: inline-block; padding: 2px 6px; border-radius: var(--radius-sm); background: var(--bg-secondary); color: var(--text-secondary); font-size: 10px; margin-left: 6px; text-transform: uppercase; letter-spacing: .5px; }
.badge.status-pending { background: rgba(241, 196, 15, .15); color: var(--warning); }
.badge.status-resolved { background: rgba(46, 204, 113, .15); color: var(--success); }
.badge.status-dismissed { background: rgba(138, 150, 168, .15); color: var(--text-secondary); }
.btn-secondary { padding: 4px 8px; border-radius: var(--radius-sm); cursor: pointer; font-size: 11px; font-weight: 600; border: 1px solid var(--border); background: transparent; color: var(--text-primary); }
.btn-secondary:hover { background: var(--bg-hover); }
.xs { padding: 4px 8px; }
</style>
