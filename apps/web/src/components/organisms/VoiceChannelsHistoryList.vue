<script setup lang="ts">
import AppSelect from "@/components/atoms/AppSelect.vue";
import { ref, computed } from "vue";
import { useVoiceChannels } from "../../composables/useVoiceChannels";
import { useFormatDate } from "../../composables/useFormatDate";
import { useConfirm } from "../../composables/useConfirm";
import { usePagination } from "../../composables/usePagination";
import { useComponentVisibility } from "@/composables/useComponentVisibility";
import AppBadge from "../atoms/AppBadge.vue";
import PaginationBar from "../molecules/PaginationBar.vue";

const emit = defineEmits<{ select: [channelId: string] }>();

const { visible } = useComponentVisibility();
const { formatShortDateTime: fmt } = useFormatDate();
const { confirm: confirmDialog } = useConfirm();

const {
  historyChannels,
  historyLoading,
  purging,
  purgingAll,
  purgeChannel,
  purgeAllHistory,
} = useVoiceChannels();

const historyFilterKind = ref<"all" | "public" | "private">("all");
const historySearch = ref("");
const historyFrom = ref("");
const historyTo = ref("");

const filteredHistory = computed(() => {
  const q = historySearch.value.trim().toLowerCase();
  const from = historyFrom.value ? new Date(historyFrom.value).getTime() : null;
  const to = historyTo.value ? new Date(historyTo.value).getTime() + 86_400_000 : null;
  return historyChannels.value.filter((c) => {
    if (historyFilterKind.value !== "all" && c.kind !== historyFilterKind.value) return false;
    if (q && !c.channel_name.toLowerCase().includes(q) && !c.owner_name.toLowerCase().includes(q)) return false;
    const created = new Date(c.created_at).getTime();
    if (from !== null && created < from) return false;
    if (to !== null && created >= to) return false;
    return true;
  });
});

function resetHistoryFilters() {
  historyFilterKind.value = "all";
  historySearch.value = "";
  historyFrom.value = "";
  historyTo.value = "";
}

const {
  currentPage: historyPage,
  perPage: historyPerPage,
  totalItems: historyTotal,
  totalPages: historyTotalPages,
  paginatedItems: paginatedHistory,
} = usePagination(filteredHistory);

async function handlePurgeAll() {
  const ok = await confirmDialog({
    title: "Vider l'historique",
    message:
      `Supprimer definitivement les ${historyChannels.value.length} salon(s) de l'historique ?\n\n` +
      "Toutes les lignes fermees et leurs timelines seront effacees de la BDD. Action irreversible.",
  });
  if (!ok) return;
  await purgeAllHistory();
}

async function handlePurge(channelId: string, channelName: string) {
  const ok = await confirmDialog({
    title: "Supprimer de l'historique",
    message: `Supprimer definitivement "${channelName}" ?\n\nLa ligne et sa timeline seront effacees de la BDD. Cette action est irreversible.`,
  });
  if (!ok) return;
  await purgeChannel(channelId);
}

function kindVariant(kind: string): "info" | "warning" | "default" {
  switch (kind) {
    case "public": return "info";
    case "private": return "warning";
    default: return "default";
  }
}
</script>

<template>
  <section class="history-section">
    <div class="history-head">
      <div>
        <h2 class="history-title">
          Historique
          <span class="history-count">{{ historyChannels.length }}</span>
        </h2>
        <p class="history-subtitle">Salons vocaux fermes / archives</p>
      </div>
      <button
        v-if="historyChannels.length > 0 && visible('db.purge.voice_history')"
        class="cleanup-btn"
        :disabled="purgingAll"
        title="Supprime definitivement tout l'historique en BDD (owner uniquement)"
        @click="handlePurgeAll"
      >
        {{ purgingAll ? "Suppression…" : `Tout supprimer (${historyChannels.length})` }}
      </button>
    </div>

    <div class="filter-row history-filters">
      <AppSelect v-model="historyFilterKind" class="filter-select">
        <option value="all">Tous les types</option>
        <option value="public">Public</option>
        <option value="private">Prive</option>
      </AppSelect>
      <input
        v-model="historySearch"
        type="search"
        placeholder="Rechercher nom ou proprietaire…"
        class="filter-input"
      />
      <input v-model="historyFrom" type="date" class="filter-input" title="Date de debut" />
      <input v-model="historyTo" type="date" class="filter-input" title="Date de fin" />
      <button class="reset-btn" type="button" @click="resetHistoryFilters">Reinitialiser</button>
    </div>

    <div v-if="historyLoading" class="loading">Chargement de l'historique...</div>
    <div v-else-if="historyChannels.length === 0" class="empty">
      Aucun salon dans l'historique
    </div>
    <div v-else-if="filteredHistory.length === 0" class="empty">
      Aucun salon ne correspond aux filtres
    </div>
    <table v-else class="data-table history-table">
      <thead>
        <tr>
          <th>Nom</th>
          <th>Proprietaire</th>
          <th>Type</th>
          <th>Creation</th>
          <th></th>
        </tr>
      </thead>
      <tbody>
        <tr
          v-for="ch in paginatedHistory"
          :key="ch.id"
          class="clickable"
          @click="emit('select', ch.channel_id)"
        >
          <td>{{ ch.channel_name }}</td>
          <td>{{ ch.owner_name }}</td>
          <td><AppBadge :label="ch.kind" :variant="kindVariant(ch.kind)" /></td>
          <td>{{ fmt(ch.created_at) }}</td>
          <td @click.stop>
            <button
              v-if="visible('db.purge.voice_channel')"
              class="close-row-btn"
              :disabled="purging === ch.channel_id"
              title="Supprimer definitivement cette ligne (owner uniquement)"
              @click="handlePurge(ch.channel_id, ch.channel_name)"
            >
              {{ purging === ch.channel_id ? "…" : "Supprimer" }}
            </button>
          </td>
        </tr>
      </tbody>
    </table>
    <PaginationBar
      v-if="filteredHistory.length > 0"
      :current-page="historyPage"
      :total-pages="historyTotalPages"
      :total-items="historyTotal"
      :per-page="historyPerPage"
      @update:current-page="historyPage = $event"
      @update:per-page="historyPerPage = $event"
    />
  </section>
</template>

<style scoped>
.history-section {
  margin-top: 40px;
  padding-top: 24px;
  border-top: 1px solid var(--border);
}

.history-head {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: 16px;
  margin-bottom: 16px;
}

.history-title {
  display: flex;
  align-items: center;
  gap: 10px;
  font-size: 18px;
  font-weight: 700;
  color: var(--text-primary);
  margin: 0 0 4px;
}

.history-count {
  font-size: 12px;
  font-weight: 600;
  padding: 2px 10px;
  border-radius: 999px;
  background-color: var(--bg-hover);
  color: var(--text-secondary);
}

.history-subtitle {
  color: var(--text-secondary);
  font-size: 12px;
  margin: 0 0 16px;
}

.filter-row {
  margin-bottom: 16px;
  display: flex;
  align-items: center;
  gap: 12px;
  flex-wrap: wrap;
}

.filter-select,
.filter-input {
  padding: 8px 12px;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--bg-secondary);
  color: var(--text-primary);
  font-size: 13px;
}

.filter-input { min-width: 180px; }

.reset-btn {
  background: transparent;
  color: var(--text-secondary);
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 7px 14px;
  font-size: 12px;
  cursor: pointer;
}
.reset-btn:hover { background: var(--bg-hover); color: var(--text-primary); }

.cleanup-btn {
  background: transparent;
  color: var(--danger);
  border: 1px solid var(--danger);
  border-radius: 6px;
  padding: 7px 14px;
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  transition: all var(--transition-fast);
}
.cleanup-btn:hover:not(:disabled) { background: var(--danger); color: white; }
.cleanup-btn:disabled { opacity: 0.5; cursor: not-allowed; }

.close-row-btn {
  background: transparent;
  color: var(--danger);
  border: 1px solid var(--danger);
  border-radius: 6px;
  padding: 4px 10px;
  font-size: 11px;
  font-weight: 600;
  cursor: pointer;
  transition: all var(--transition-fast);
  white-space: nowrap;
}
.close-row-btn:hover:not(:disabled) { background: var(--danger); color: white; }
.close-row-btn:disabled { opacity: 0.4; cursor: not-allowed; }

.data-table { width: 100%; border-collapse: collapse; }
.data-table th, .data-table td {
  padding: 10px 14px;
  text-align: left;
  border-bottom: 1px solid var(--border);
  font-size: 13px;
}
.data-table th {
  color: var(--text-secondary);
  font-weight: 600;
  font-size: 12px;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.history-table { opacity: 0.85; }
.history-table tbody tr:hover { opacity: 1; }

.clickable { cursor: pointer; }
.clickable:hover { background: var(--bg-hover); }

.loading, .empty {
  text-align: center;
  padding: 40px;
  color: var(--text-secondary);
}

@media (max-width: 768px) {
  .data-table { font-size: 12px; }
  .data-table th, .data-table td { padding: 6px 8px !important; }
}
</style>
