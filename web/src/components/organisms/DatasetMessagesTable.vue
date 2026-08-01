<script setup lang="ts">
import AppButton from "../atoms/AppButton.vue";
import { useAiDataset } from "@/composables/useAiDataset";
import { useFormatDate } from "@/composables/useFormatDate";

const { items, total, limit, offset, loading, getLabel, setLabel, nextPage, prevPage } = useAiDataset();
const { formatDateTimeShort: fmtDate } = useFormatDate();

function truncate(s: string, n: number): string {
  return s.length > n ? s.slice(0, n) + "…" : s;
}
</script>

<template>
  <section class="card">
    <div v-if="loading" class="muted">Chargement…</div>
    <div v-else-if="items.length === 0" class="muted">Aucun message correspondant aux filtres.</div>
    <table v-else class="msg-table">
      <thead>
        <tr>
          <th class="lbl-col">Étiquette</th>
          <th>Message</th>
          <th class="meta-col">Channel</th>
          <th class="meta-col">Date</th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="m in items" :key="m.id" :class="`row-${getLabel(m.id)}`">
          <td class="lbl-cell">
            <button class="seg" :class="{ active: getLabel(m.id) === 'safe' }" @click="setLabel(m.id, 'safe')" title="Safe">✅</button>
            <button class="seg" :class="{ active: getLabel(m.id) === 'severe' }" @click="setLabel(m.id, 'severe')" title="Severe">⚠️</button>
            <button class="seg" :class="{ active: getLabel(m.id) === 'skip' }" @click="setLabel(m.id, 'skip')" title="Skip">↩</button>
          </td>
          <td class="msg-cell">{{ truncate(m.content, 400) }}</td>
          <td class="small muted">{{ m.channel_name ?? m.channel_id ?? '—' }}</td>
          <td class="small muted">{{ fmtDate(m.created_at) }}</td>
        </tr>
      </tbody>
    </table>

    <div v-if="items.length > 0" class="pagination">
      <AppButton variant="ghost" :disabled="offset === 0 || loading" @click="prevPage">← Précédent</AppButton>
      <span class="muted">{{ offset + 1 }} – {{ Math.min(offset + items.length, total) }} sur {{ total }}</span>
      <AppButton variant="ghost" :disabled="offset + limit>= total || loading" @click="nextPage">Suivant →</AppButton>
    </div>
  </section>
</template>

<style scoped>
.card {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  padding: 16px;
  margin-bottom: 16px;
}
.muted { color: var(--text-secondary); font-size: 12px; }
.msg-table { width: 100%; border-collapse: collapse; font-size: 12px; }
.msg-table th, .msg-table td {
  padding: 8px 10px;
  border-bottom: 1px solid color-mix(in srgb, var(--border) 60%, transparent);
  vertical-align: top;
}
.msg-table th {
  text-align: left;
  font-size: 10px;
  text-transform: uppercase;
  color: var(--text-secondary);
}
.lbl-col { width: 130px; }
.meta-col { width: 180px; }
.lbl-cell { white-space: nowrap; }
.msg-cell { word-break: break-word; line-height: 1.5; }
.small { font-size: 11px; }
.seg {
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  padding: 4px 8px;
  border-radius: var(--radius-sm);
  cursor: pointer;
  margin-right: 2px;
  font-size: 14px;
}
.seg.active { background: var(--accent); border-color: var(--accent); color: white; }
tr.row-safe { background: color-mix(in srgb, var(--success, var(--success)) 6%, transparent); }
tr.row-severe { background: color-mix(in srgb, var(--danger) 7%, transparent); }
.pagination {
  display: flex; justify-content: center; align-items: center;
  gap: 16px; margin-top: 16px;
}
.btn {
  padding: 7px 14px;
  border-radius: var(--radius-md);
  border: 1px solid var(--border);
  background: var(--bg-secondary);
  color: var(--text-primary);
  font-size: 12px; font-weight: 600; cursor: pointer;
}
.btn:hover:not(:disabled) { border-color: var(--accent); color: var(--accent); }
.btn:disabled { opacity: 0.5; cursor: not-allowed; }
</style>
