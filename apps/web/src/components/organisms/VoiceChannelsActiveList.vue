<script setup lang="ts">
import { useVoiceChannels } from "../../composables/useVoiceChannels";
import { useFormatDate } from "../../composables/useFormatDate";
import { useConfirm } from "../../composables/useConfirm";
import { usePagination } from "../../composables/usePagination";
import AppBadge from "../atoms/AppBadge.vue";
import ErrorState from "../atoms/ErrorState.vue";
import PaginationBar from "../molecules/PaginationBar.vue";

const emit = defineEmits<{ select: [channelId: string] }>();

const { formatShortDateTime: fmt } = useFormatDate();
const { confirm: confirmDialog } = useConfirm();

const {
  filteredChannels,
  loading,
  error,
  filterKind,
  closing,
  cleaningAll,
  fetchChannels,
  closeChannel,
  closeAllDisplayed,
} = useVoiceChannels();

const { currentPage, perPage, totalItems, totalPages, paginatedItems: paginatedChannels } =
  usePagination(filteredChannels);

async function handleCloseChannel(channelId: string, channelName: string) {
  const ok = await confirmDialog({
    title: "Fermer le salon",
    message: `Fermer "${channelName}" ?\n\nLa ligne sera marquee comme fermee en BDD et ne s'affichera plus ici. Aucune action cote Discord.`,
  });
  if (!ok) return;
  await closeChannel(channelId);
}

async function handleCleanupAll() {
  const ok = await confirmDialog({
    title: "Nettoyer tous les salons affiches",
    message:
      `Fermer ${filteredChannels.value.length} salon(s) actuellement affiches ?\n\n` +
      "Utile pour nettoyer les lignes fantomes laissees par un bot qui a crash " +
      "ou redemarre. Aucune action cote Discord, uniquement en BDD.",
  });
  if (!ok) return;
  await closeAllDisplayed();
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
  <div>
    <div class="filter-row">
      <select v-model="filterKind" class="filter-select">
        <option value="all">Tous les types</option>
        <option value="public">Public</option>
        <option value="private">Prive</option>
      </select>
      <button
        v-if="filteredChannels.length > 0"
        class="cleanup-btn"
        :disabled="cleaningAll"
        title="Ferme tous les salons affiches en BDD (nettoyage des fantomes)"
        @click="handleCleanupAll"
      >
        {{ cleaningAll ? "Nettoyage…" : `Nettoyer tout (${filteredChannels.length})` }}
      </button>
    </div>

    <ErrorState v-if="error" :message="error" :retryable="true" @retry="fetchChannels" />
    <div v-else-if="loading" class="loading">Chargement...</div>
    <div v-else-if="filteredChannels.length === 0" class="empty">Aucun salon vocal temporaire actif</div>
    <table v-else class="data-table">
      <thead>
        <tr>
          <th>Nom</th>
          <th>Proprietaire</th>
          <th>Type</th>
          <th>Visibilite</th>
          <th>Verrouille</th>
          <th>File d'attente</th>
          <th>Creation</th>
          <th></th>
        </tr>
      </thead>
      <tbody>
        <tr
          v-for="ch in paginatedChannels"
          :key="ch.id"
          class="clickable"
          @click="emit('select', ch.channel_id)"
        >
          <td>{{ ch.channel_name }}</td>
          <td>{{ ch.owner_name }}</td>
          <td><AppBadge :label="ch.kind" :variant="kindVariant(ch.kind)" /></td>
          <td>{{ ch.visibility }}</td>
          <td>{{ ch.locked ? 'Oui' : 'Non' }}</td>
          <td>{{ ch.queue_enabled ? 'Oui' : 'Non' }}</td>
          <td>{{ fmt(ch.created_at) }}</td>
          <td @click.stop>
            <button
              class="close-row-btn"
              :disabled="closing === ch.channel_id"
              title="Fermer ce salon en BDD"
              @click="handleCloseChannel(ch.channel_id, ch.channel_name)"
            >
              {{ closing === ch.channel_id ? "…" : "Fermer" }}
            </button>
          </td>
        </tr>
      </tbody>
    </table>

    <PaginationBar
      :current-page="currentPage"
      :total-pages="totalPages"
      :total-items="totalItems"
      :per-page="perPage"
      @update:current-page="currentPage = $event"
      @update:per-page="perPage = $event"
    />
  </div>
</template>

<style scoped>
.filter-row {
  margin-bottom: 16px;
  display: flex;
  align-items: center;
  gap: 12px;
  flex-wrap: wrap;
}

.filter-select {
  padding: 8px 12px;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--bg-secondary);
  color: var(--text-primary);
  font-size: 13px;
}

.cleanup-btn {
  margin-left: auto;
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

.data-table {
  width: 100%;
  border-collapse: collapse;
}
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
