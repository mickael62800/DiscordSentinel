<script setup lang="ts">
import type { DossierNote } from "../../../types";
import { useFormatDate } from "../../../composables/useFormatDate";

defineProps<{ notes: DossierNote[] }>();

const { formatShortDateTime: fmt } = useFormatDate();
</script>

<template>
  <div v-if="notes && notes.length > 0" class="section">
    <h3>Notes ({{ notes.length }})</h3>
    <div v-for="(note, i) in notes" :key="i" class="detail-row">
      <div class="detail-row-header">
        <span class="detail-date">{{ note.created_at ? fmt(String(note.created_at)) : '' }}</span>
        <span class="note-author">{{ note.author_name }}</span>
      </div>
      <div class="detail-row-body">{{ note.content }}</div>
    </div>
  </div>
</template>

<style scoped>
.section { margin-bottom: 20px; }
.section h3 { margin: 0 0 10px 0; font-size: 14px; font-weight: 600; }

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
.detail-date {
  font-size: 11px;
  color: var(--text-secondary);
  font-family: "JetBrains Mono", "Cascadia Code", monospace;
  flex-shrink: 0;
}
.detail-row-body { font-size: 13px; color: var(--text-primary); white-space: pre-wrap; word-break: break-word; }
.note-author { font-size: 12px; font-weight: 600; color: var(--accent); }
</style>
