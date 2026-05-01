<script setup lang="ts">
import { computed, ref } from "vue";
import { useLogs } from "@/composables/useLogs";
import { useFormatDate } from "@/composables/useFormatDate";
import { useConfirm } from "@/composables/useConfirm";
import { levelVariant } from "@/utils/variants";
import AppBadge from "@/components/atoms/AppBadge.vue";

interface Props {
  title: string;
  category: string;
  /** Limite de lignes affichees par colonne (defaut 50). */
  maxItems?: number;
}
const props = withDefaults(defineProps<Props>(), { maxItems: 50 });

const { formatShortDateTime: fmt } = useFormatDate();
const { confirm } = useConfirm();
const { filteredLogs, loading, filterLevel, clearLogs } = useLogs(props.category);

const visible = computed(() => filteredLogs.value.slice(0, props.maxItems));

const expandedId = ref<string | number | null>(null);
function toggle(id: string | number) {
  expandedId.value = expandedId.value === id ? null : id;
}

async function handleClear() {
  const ok = await confirm({ message: `Supprimer tous les journaux ${props.title.toLowerCase()} ?` });
  if (!ok) return;
  await clearLogs();
}
</script>

<template>
  <div class="logs-column">
    <header class="col-head">
      <h3>{{ title }}</h3>
      <div class="col-actions">
        <select v-model="filterLevel" class="level-select">
          <option value="all">Tous</option>
          <option value="info">Info</option>
          <option value="warn">Warn</option>
          <option value="error">Error</option>
        </select>
        <button class="clear-icon" title="Vider" @click="handleClear">🗑</button>
      </div>
    </header>

    <div v-if="loading" class="loading-mini">Chargement…</div>
    <div v-else-if="visible.length === 0" class="empty-mini">Aucun log.</div>
    <ul v-else class="log-list">
      <li
        v-for="(log, i) in visible"
        :key="(log as Record<string, unknown>).id as string ?? i"
        :class="['log-item', `lvl-${(log as Record<string, unknown>).level}`]"
        @click="toggle((log as Record<string, unknown>).id as string ?? i)"
      >
        <div class="log-line1">
          <AppBadge
            :label="String((log as Record<string, unknown>).level)"
            :variant="levelVariant(String((log as Record<string, unknown>).level))"
          />
          <span class="log-source">{{ (log as Record<string, unknown>).bot ?? "—" }}</span>
          <span class="log-time">{{ fmt(String((log as Record<string, unknown>).timestamp)) }}</span>
        </div>
        <div class="log-msg">{{ (log as Record<string, unknown>).message }}</div>
        <pre
          v-if="expandedId === ((log as Record<string, unknown>).id ?? i)
                && (log as Record<string, unknown>).details
                && Object.keys((log as Record<string, unknown>).details as object ?? {}).length > 0"
          class="log-details"
        >{{ JSON.stringify((log as Record<string, unknown>).details, null, 2) }}</pre>
      </li>
    </ul>

    <footer v-if="filteredLogs.length > visible.length" class="col-foot">
      {{ filteredLogs.length - visible.length }} ligne(s) plus anciennes masquées
    </footer>
  </div>
</template>

<style scoped>
.logs-column {
  display: flex;
  flex-direction: column;
  min-height: 0;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 10px;
  overflow: hidden;
}
.col-head {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 10px 12px;
  background: var(--bg-secondary);
  border-bottom: 1px solid var(--border);
}
.col-head h3 { margin: 0; font-size: 13px; font-weight: 700; color: var(--text-primary); text-transform: uppercase; letter-spacing: 0.5px; }
.col-actions { display: flex; gap: 6px; align-items: center; }
.level-select {
  padding: 4px 8px;
  font-size: 11px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  color: var(--text-primary);
  border-radius: 5px;
  cursor: pointer;
}
.clear-icon {
  background: transparent;
  border: 1px solid var(--border);
  border-radius: 5px;
  width: 26px;
  height: 26px;
  cursor: pointer;
  font-size: 12px;
  color: var(--text-secondary);
}
.clear-icon:hover { color: var(--danger); border-color: var(--danger); }

.log-list {
  list-style: none;
  margin: 0;
  padding: 0;
  overflow-y: auto;
  max-height: 65vh;
}
.log-item {
  padding: 8px 12px;
  border-bottom: 1px solid color-mix(in srgb, var(--border) 60%, transparent);
  cursor: pointer;
  transition: background-color 0.15s;
}
.log-item:hover { background: color-mix(in srgb, var(--accent) 5%, transparent); }
.log-item.lvl-warn { border-left: 3px solid var(--warning, #f59e0b); }
.log-item.lvl-error { border-left: 3px solid var(--danger, #ef4444); }

.log-line1 {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 11px;
  margin-bottom: 4px;
}
.log-source {
  font-family: "JetBrains Mono", monospace;
  color: var(--text-secondary);
  font-weight: 600;
}
.log-time { margin-left: auto; color: var(--text-secondary); font-family: "JetBrains Mono", monospace; }

.log-msg {
  font-size: 12px;
  color: var(--text-primary);
  line-height: 1.4;
  word-break: break-word;
}
.log-details {
  margin: 6px 0 0 0;
  padding: 6px 8px;
  background: var(--bg-secondary);
  border-radius: 4px;
  font-size: 10px;
  color: var(--text-secondary);
  max-height: 200px;
  overflow: auto;
  white-space: pre-wrap;
}

.loading-mini, .empty-mini {
  padding: 30px 12px;
  text-align: center;
  color: var(--text-secondary);
  font-style: italic;
  font-size: 12px;
}

.col-foot {
  padding: 8px 12px;
  text-align: center;
  font-size: 11px;
  color: var(--text-secondary);
  background: var(--bg-secondary);
  border-top: 1px solid var(--border);
}
</style>
