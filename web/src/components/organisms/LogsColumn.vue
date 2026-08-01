<script setup lang="ts">
import AppSelect from "@/components/atoms/AppSelect.vue";
import { computed, ref, watch } from "vue";
import { useLogs } from "@/composables/useLogs";
import { useFormatDate } from "@/composables/useFormatDate";
import { useConfirm } from "@/composables/useConfirm";
import { levelVariant } from "@/utils/variants";
import AppBadge from "@/components/atoms/AppBadge.vue";

interface Props {
  title: string;
  category: string;
  /** Lignes par page (defaut 20). */
  pageSize?: number;
  /** Niveau impose par le filtre global de la page parent. Si fourni et
   *  different de 'all', remplace le filtre local de la colonne. */
  forceLevel?: "all" | "info" | "warn" | "error";
}
const props = withDefaults(defineProps<Props>(), { pageSize: 20, forceLevel: "all" });

const { formatShortDateTime: fmt } = useFormatDate();
const { confirm } = useConfirm();
const { filteredLogs, loading, filterLevel, clearLogs } = useLogs(props.category);

// Synchronise le filtre local avec le filtre global parent quand celui-ci
// change. Garde la possibilite de surcharger localement par colonne.
watch(
  () => props.forceLevel,
  (lv) => {
    if (lv) filterLevel.value = lv;
  },
  { immediate: true },
);

const expandedId = ref<string | number | null>(null);
function toggle(id: string | number) {
  expandedId.value = expandedId.value === id ? null : id;
}

// ── Pagination ──
const currentPage = ref(1);
const totalPages = computed(() => Math.max(1, Math.ceil(filteredLogs.value.length / props.pageSize)));
const visible = computed(() => {
  const start = (currentPage.value - 1) * props.pageSize;
  return filteredLogs.value.slice(start, start + props.pageSize);
});

function goToPage(p: number) {
  if (p < 1 || p > totalPages.value) return;
  currentPage.value = p;
}

async function handleClear() {
  const ok = await confirm({
    title: `Vider ${props.title}`,
    message: `Supprimer définitivement tous les journaux ${props.title.toLowerCase()} ?`,
  });
  if (!ok) return;
  await clearLogs();
  currentPage.value = 1;
}
</script>

<template>
  <div class="logs-column">
    <header class="col-head">
      <h3>{{ title }}</h3>
      <div class="col-actions">
        <AppSelect v-model="filterLevel" class="level-select">
          <option value="all">Tous</option>
          <option value="info">Info</option>
          <option value="warn">Warn</option>
          <option value="error">Error</option>
        </AppSelect>
        <button class="clear-btn" title="Vider tous les journaux" @click="handleClear">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M3 6h18" />
            <path d="M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
            <path d="M19 6l-1 14a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2L5 6" />
            <path d="M10 11v6" />
            <path d="M14 11v6" />
          </svg>
          <span>Vider</span>
        </button>
      </div>
    </header>

    <div v-if="loading" class="loading-mini">Chargement…</div>
    <div v-else-if="visible.length === 0" class="empty-mini">Aucun log.</div>
    <ul v-else class="log-list">
      <li
        v-for="(log, i) in visible"
        :key="log.id ?? i"
        :class="['log-item', `lvl-${log.level}`]"
        @click="toggle(log.id ?? i)"
      >
        <div class="log-line1">
          <AppBadge
            :label="log.level"
            :variant="levelVariant(log.level)"
          />
          <span class="log-source">{{ log.bot ?? "—" }}</span>
          <span class="log-time">{{ fmt(log.timestamp) }}</span>
        </div>
        <div class="log-msg">{{ log.message }}</div>
        <pre
          v-if="expandedId === (log.id ?? i)
                && log.details
                && Object.keys(log.details).length > 0"
          class="log-details"
        >{{ JSON.stringify(log.details, null, 2) }}</pre>
      </li>
    </ul>

    <footer v-if="!loading && filteredLogs.length > 0" class="col-foot">
      <button class="page-btn" :disabled="currentPage === 1" @click="goToPage(1)" title="Première page">«</button>
      <button class="page-btn" :disabled="currentPage === 1" @click="goToPage(currentPage - 1)">‹</button>
      <span class="page-info">{{ currentPage }} / {{ totalPages }}</span>
      <button class="page-btn" :disabled="currentPage === totalPages" @click="goToPage(currentPage + 1)">›</button>
      <button class="page-btn" :disabled="currentPage === totalPages" @click="goToPage(totalPages)" title="Dernière page">»</button>
      <span class="total-count">{{ filteredLogs.length }} log{{ filteredLogs.length > 1 ? "s" : "" }}</span>
    </footer>
  </div>
</template>

<style scoped>
.logs-column {
  display: flex;
  flex-direction: column;
  /* Hauteur calculee : viewport - en-tete page (env. 130px) - margin */
  height: calc(100vh - 200px);
  min-height: 400px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  overflow: hidden;
}
.col-head {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 10px 12px;
  background: var(--bg-secondary);
  border-bottom: 1px solid var(--border);
  flex-shrink: 0;
}
.col-head h3 { margin: 0; font-size: 13px; font-weight: 700; color: var(--text-primary); text-transform: uppercase; letter-spacing: 0.5px; }
.col-actions { display: flex; gap: 8px; align-items: center; }
.level-select {
  padding: 6px 10px;
  height: 30px;
  font-size: 12px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  color: var(--text-primary);
  border-radius: var(--radius-sm);
  cursor: pointer;
}
.level-select:hover { border-color: var(--accent); }

.clear-btn {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  height: 30px;
  padding: 0 12px;
  font-size: 12px;
  font-weight: 600;
  background: transparent;
  border: 1px solid color-mix(in srgb, var(--danger, #ef4444) 50%, var(--border));
  color: var(--danger, #ef4444);
  border-radius: var(--radius-sm);
  cursor: pointer;
  transition: background-color 0.15s, border-color 0.15s;
}
.clear-btn:hover {
  background: color-mix(in srgb, var(--danger, #ef4444) 12%, transparent);
  border-color: var(--danger, #ef4444);
}
.clear-btn:active { transform: translateY(1px); }
.clear-btn svg { width: 14px; height: 14px; }

.log-list {
  list-style: none;
  margin: 0;
  padding: 0;
  overflow-y: auto;
  flex: 1 1 auto;
  min-height: 0;
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
  border-radius: var(--radius-sm);
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
  flex: 1 1 auto;
  display: flex;
  align-items: center;
  justify-content: center;
}

.col-foot {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 4px;
  padding: 8px 10px;
  background: var(--bg-secondary);
  border-top: 1px solid var(--border);
  flex-shrink: 0;
  font-size: 11px;
}
.page-btn {
  min-width: 26px;
  height: 26px;
  padding: 0 6px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  color: var(--text-primary);
  border-radius: var(--radius-sm);
  cursor: pointer;
  font-size: 12px;
  font-weight: 600;
}
.page-btn:hover:not(:disabled) { border-color: var(--accent); color: var(--accent); }
.page-btn:disabled { opacity: 0.4; cursor: not-allowed; }
.page-info {
  padding: 0 6px;
  color: var(--text-secondary);
  font-family: "JetBrains Mono", monospace;
  min-width: 40px;
  text-align: center;
}
.total-count {
  margin-left: auto;
  color: var(--text-secondary);
  font-size: 10px;
}
</style>
