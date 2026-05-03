<script setup lang="ts">
import { ref, onMounted, watch } from "vue";
import { blackjackService, type BlackjackTable, type BlackjackTablePlayer } from "@/services/blackjackService";
import { useGuildSelector } from "../../composables/useGuildSelector";
import { useFormatDate } from "../../composables/useFormatDate";
import { useConfirm } from "../../composables/useConfirm";
import { useToast } from "../../composables/useToast";
import AppButton from "../atoms/AppButton.vue";

const { formatShortDateTime: fmt } = useFormatDate();
const { selectedGuildId } = useGuildSelector();
const { confirm } = useConfirm();
const { success, error: toastError } = useToast();

const tables = ref<BlackjackTable[]>([]);
const tablesLoading = ref(false);
const expandedTable = ref<string | null>(null);
const tablePlayers = ref<Record<string, BlackjackTablePlayer[]>>({});
const closingTable = ref<string | null>(null);

async function fetchTables() {
  if (!selectedGuildId.value) return;
  tablesLoading.value = true;
  try {
    tables.value = await blackjackService.listTables(selectedGuildId.value);
  } catch (e) {
    toastError(String(e));
  } finally {
    tablesLoading.value = false;
  }
}

defineExpose({ refresh: fetchTables });

async function toggleTable(id: string) {
  if (expandedTable.value === id) {
    expandedTable.value = null;
    return;
  }
  expandedTable.value = id;
  if (!tablePlayers.value[id]) {
    try {
      tablePlayers.value[id] = await blackjackService.listTablePlayers(id);
    } catch (e) {
      toastError(String(e));
    }
  }
}

async function closeTable(table: BlackjackTable) {
  const ok = await confirm({
    title: "Fermer la table",
    message: `Fermer la table de ${table.owner_name} (channel ${table.channel_id}) ?`,
  });
  if (!ok) return;
  closingTable.value = table.id;
  try {
    await blackjackService.closeTable(table.id);
    success("Table fermee.");
    await fetchTables();
  } catch (e) {
    toastError(String(e));
  } finally {
    closingTable.value = null;
  }
}

watch(selectedGuildId, fetchTables);
onMounted(fetchTables);
</script>

<template>
  <section class="tables-section">
    <div class="section-header">
      <h2>🎲 Tables multijoueur ouvertes</h2>
      <AppButton variant="secondary" @click="fetchTables" :disabled="tablesLoading">↻</AppButton>
    </div>
    <div v-if="tablesLoading" class="muted-text">Chargement…</div>
    <div v-else-if="tables.length === 0" class="muted-text">Aucune table ouverte.</div>
    <div v-else class="tables-list">
      <div
        v-for="t in tables"
        :key="t.id"
        class="table-card"
        :class="{ expanded: expandedTable === t.id }"
      >
        <div class="table-card-main" @click="toggleTable(t.id)">
          <div class="table-info">
            <strong>{{ t.owner_name }}</strong>
            <span class="muted">channel <code>{{ t.channel_id }}</code></span>
          </div>
          <div class="table-meta">
            <span class="muted">{{ fmt(t.created_at) }}</span>
            <span class="status-badge status-success">{{ t.status }}</span>
            <button
              class="danger-btn"
              :disabled="closingTable === t.id"
              @click.stop="closeTable(t)"
              title="Fermer la table"
            >{{ closingTable === t.id ? "…" : "🚫 Fermer" }}</button>
          </div>
        </div>
        <div v-if="expandedTable === t.id" class="table-card-detail">
          <h4>Joueurs</h4>
          <ul v-if="tablePlayers[t.id]?.length" class="players-list">
            <li v-for="p in tablePlayers[t.id]" :key="p.user_id">
              <strong>{{ p.user_name }}</strong>
              <code>{{ p.user_id }}</code>
              <span class="muted">{{ fmt(p.joined_at) }}</span>
            </li>
          </ul>
          <p v-else class="muted-text">Aucun joueur encore.</p>
          <div class="detail-row"><span class="detail-label">Table ID</span><code>{{ t.id }}</code></div>
          <div class="detail-row"><span class="detail-label">Owner ID</span><code>{{ t.owner_id }}</code></div>
        </div>
      </div>
    </div>
  </section>
</template>

<style scoped>
.tables-section {
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 12px;
  padding: 16px 20px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.section-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.section-header h2 { margin: 0; font-size: 1.1rem; }

.muted, .muted-text { color: var(--text-secondary); }

.tables-list { display: flex; flex-direction: column; gap: 8px; }

.table-card {
  border: 1px solid var(--border);
  border-radius: 8px;
  overflow: hidden;
}
.table-card-main {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 10px 14px;
  cursor: pointer;
  gap: 12px;
}
.table-card-main:hover {
  background: color-mix(in srgb, var(--accent) 4%, transparent);
}
.table-info { display: flex; flex-direction: column; gap: 2px; }
.table-meta { display: flex; align-items: center; gap: 10px; }

.table-card-detail {
  padding: 12px 16px;
  border-top: 1px dashed var(--border);
  background: color-mix(in srgb, var(--accent) 3%, var(--bg));
}
.table-card-detail h4 { margin: 0 0 8px; font-size: 0.85rem; }

.players-list { list-style: none; padding: 0; margin: 0 0 10px; }
.players-list li {
  display: flex; gap: 10px; align-items: baseline;
  padding: 4px 0;
  font-size: 0.85rem;
}
.players-list code {
  font-family: "JetBrains Mono", monospace;
  font-size: 0.72rem;
  color: var(--accent);
}

.detail-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 6px 0;
  font-size: 0.85rem;
  border-bottom: 1px solid color-mix(in srgb, var(--border) 50%, transparent);
}
.detail-row:last-child { border-bottom: none; }
.detail-label { color: var(--text-secondary); }
.detail-row code {
  font-family: "JetBrains Mono", monospace;
  font-size: 0.75rem;
  color: var(--accent);
  background: color-mix(in srgb, var(--accent) 8%, transparent);
  padding: 2px 6px;
  border-radius: 4px;
}

.status-badge {
  display: inline-block;
  padding: 4px 10px;
  border-radius: 12px;
  font-size: 0.72rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.3px;
}
.status-badge.status-success {
  background: color-mix(in srgb, var(--success) 20%, transparent);
  color: var(--success);
}

.danger-btn {
  background: transparent;
  color: var(--danger);
  border: 1px solid var(--danger);
  border-radius: 6px;
  padding: 6px 12px;
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  white-space: nowrap;
  transition: all var(--transition-fast);
}
.danger-btn:hover:not(:disabled) {
  background: var(--danger);
  color: white;
}
.danger-btn:disabled { opacity: 0.5; cursor: not-allowed; }
</style>
