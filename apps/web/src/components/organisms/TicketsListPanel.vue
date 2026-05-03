<script setup lang="ts">
import { useTickets } from "../../composables/useTickets";
import { useRealtimeRefresh } from "../../composables/useRealtimeRefresh";
import { usePagination } from "../../composables/usePagination";
import { useToast } from "../../composables/useToast";
import { useConfirm } from "../../composables/useConfirm";
import { statusVariant, priorityVariant } from "../../utils/variants";
import AppBadge from "../atoms/AppBadge.vue";
import ErrorState from "../atoms/ErrorState.vue";
import PaginationBar from "../molecules/PaginationBar.vue";

const emit = defineEmits<{ select: [ticketId: string] }>();

const { success, error: toastError } = useToast();
const { confirm: confirmDialog } = useConfirm();

const {
  filteredTickets,
  loading,
  error,
  filterStatus,
  filterPriority,
  filterAuthor,
  filterFrom,
  filterTo,
  hasActiveFilters,
  openCount,
  pendingCount,
  bulkDeleting,
  fetchTickets,
  bulkDelete,
  resetFilters,
} = useTickets();

useRealtimeRefresh(
  ["ticket_new", "ticket_message", "ticket_closed", "ticket_assigned", "ticket_status_updated", "ticket_channel_updated"],
  fetchTickets,
);

const { currentPage, perPage, totalItems, totalPages, paginatedItems: paginatedTickets } =
  usePagination(filteredTickets);

async function handleBulkDelete() {
  const filterSummary: string[] = [];
  if (filterAuthor.value.trim()) filterSummary.push(`auteur ≈ "${filterAuthor.value.trim()}"`);
  if (filterFrom.value) filterSummary.push(`a partir du ${filterFrom.value}`);
  if (filterTo.value) filterSummary.push(`jusqu'au ${filterTo.value}`);
  if (filterStatus.value !== "all") filterSummary.push(`statut = ${filterStatus.value}`);
  if (filterPriority.value !== "all") filterSummary.push(`priorite = ${filterPriority.value}`);

  const hasBackendFilter =
    filterAuthor.value.trim() !== "" || filterFrom.value !== "" || filterTo.value !== "";
  const scope = filterSummary.length > 0
    ? `les tickets filtres (${filterSummary.join(", ")})`
    : "TOUS les tickets de la BDD";
  const count = filteredTickets.value.length;

  const ok1 = await confirmDialog({
    title: "⚠️ Suppression en masse",
    message:
      `Supprimer ${scope} ?\n\n` +
      `Environ ${count} ticket(s) affiches seront supprimes avec leurs messages.\n\n` +
      "Cette action est IRREVERSIBLE.",
  });
  if (!ok1) return;
  const ok2 = await confirmDialog({
    title: "Derniere confirmation",
    message: `Vraiment supprimer ${count} ticket(s) ?`,
  });
  if (!ok2) return;

  try {
    const deleted = await bulkDelete({
      author_id: filterAuthor.value.trim() || null,
      from: filterFrom.value || null,
      to: filterTo.value || null,
      // Safety : backend exige all=true si aucun filtre backend (author/from/to).
      // Les filtres status/priority sont client-side only.
      all: !hasBackendFilter,
    });
    success(`${deleted} ticket(s) supprime(s) de la BDD.`);
  } catch (e) {
    console.error("Erreur suppression en masse tickets:", e);
    toastError("Erreur lors de la suppression en masse");
  }
}
</script>

<template>
  <div>
    <div class="tickets-header">
      <h1>Tickets</h1>
      <div class="tickets-stats">
        <span class="stat"><strong>{{ openCount }}</strong> ouvert(s)</span>
        <span class="stat"><strong>{{ pendingCount }}</strong> en attente</span>
        <span class="stat"><strong>{{ filteredTickets.length }}</strong> affiches</span>
      </div>
    </div>

    <div class="card tickets-toolbar">
      <div class="filter-grid">
        <div class="filter-field filter-search">
          <label>Auteur (ID ou nom)</label>
          <input
            v-model="filterAuthor"
            type="text"
            placeholder="Discord user id ou nom..."
          />
        </div>
        <div class="filter-field">
          <label>Statut</label>
          <select v-model="filterStatus">
            <option value="all">Tous</option>
            <option value="open">Ouvert</option>
            <option value="pending">En attente</option>
            <option value="closed">Ferme</option>
          </select>
        </div>
        <div class="filter-field">
          <label>Priorite</label>
          <select v-model="filterPriority">
            <option value="all">Toutes</option>
            <option value="urgent">Urgent</option>
            <option value="high">Elevee</option>
            <option value="medium">Moyenne</option>
            <option value="low">Faible</option>
          </select>
        </div>
        <div class="filter-field">
          <label>Du</label>
          <input v-model="filterFrom" type="date" />
        </div>
        <div class="filter-field">
          <label>Au</label>
          <input v-model="filterTo" type="date" />
        </div>
      </div>

      <div class="toolbar-actions">
        <button
          v-if="hasActiveFilters"
          class="reset-btn"
          @click="resetFilters"
        >
          Reinitialiser
        </button>
        <button
          class="bulk-delete-btn"
          :disabled="bulkDeleting"
          :title="hasActiveFilters ? 'Supprimer les tickets filtres' : 'Supprimer TOUS les tickets'"
          @click="handleBulkDelete"
        >
          {{
            bulkDeleting
              ? "Suppression…"
              : hasActiveFilters
                ? `Supprimer filtres (${filteredTickets.length})`
                : "Tout supprimer"
          }}
        </button>
      </div>
    </div>

    <ErrorState v-if="error" :message="error" :retryable="true" @retry="fetchTickets" />
    <div v-else-if="loading" class="loading">Chargement...</div>

    <div v-else class="card ticket-list">
      <div
        v-for="ticket in paginatedTickets"
        :key="ticket.id"
        class="ticket-row"
        @click="emit('select', ticket.id)"
      >
        <div class="ticket-main">
          <div class="ticket-title-line">
            <span class="ticket-title">{{ ticket.title }}</span>
            <AppBadge :label="ticket.priority" :variant="priorityVariant(ticket.priority)" />
          </div>
          <div class="ticket-meta">
            <span>{{ ticket.author_name }}</span>
            <span class="sep">dans</span>
            <span>{{ ticket.server }}</span>
            <span class="sep">-</span>
            <span class="category">{{ ticket.category }}</span>
          </div>
        </div>
        <div class="ticket-side">
          <AppBadge :label="ticket.status" :variant="statusVariant(ticket.status)" />
          <span class="ticket-messages">{{ ticket.messages_count }} msg.</span>
          <span class="ticket-date">{{ ticket.updated_at }}</span>
        </div>
      </div>

      <div v-if="filteredTickets.length === 0" class="empty">
        Aucun ticket correspondant aux filtres
      </div>
    </div>

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
.tickets-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 24px;
}
.tickets-header h1 { margin: 0; }

.tickets-stats {
  display: flex;
  gap: 16px;
  color: var(--text-secondary);
  font-size: 13px;
}

.tickets-toolbar {
  margin-bottom: var(--space-lg);
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.filter-grid {
  display: grid;
  grid-template-columns: 2fr 1fr 1fr 1fr 1fr;
  gap: 12px;
}
@media (max-width: 1200px) {
  .filter-grid { grid-template-columns: repeat(3, 1fr); }
  .filter-search { grid-column: 1 / -1; }
}
@media (max-width: 700px) {
  .filter-grid { grid-template-columns: 1fr; }
}

.filter-field { display: flex; flex-direction: column; gap: 4px; min-width: 0; }
.filter-field label {
  font-size: 11px;
  font-weight: 600;
  text-transform: uppercase;
  color: var(--text-secondary);
  letter-spacing: 0.3px;
}
.filter-field input,
.filter-field select {
  background-color: var(--bg-primary);
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 8px 10px;
  color: var(--text-primary);
  font-size: 13px;
  font-family: inherit;
  outline: none;
  color-scheme: dark;
}
.filter-field input:focus,
.filter-field select:focus { border-color: var(--accent); }

.toolbar-actions {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
  flex-wrap: wrap;
}

.reset-btn {
  background: transparent;
  color: var(--text-secondary);
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 7px 14px;
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  transition: all var(--transition-fast);
}
.reset-btn:hover { color: var(--text-primary); border-color: var(--accent); }

.bulk-delete-btn {
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
.bulk-delete-btn:hover:not(:disabled) {
  background: var(--danger);
  color: white;
}
.bulk-delete-btn:disabled { opacity: 0.4; cursor: not-allowed; }

.ticket-list {
  display: flex;
  flex-direction: column;
  padding: 0;
  overflow: hidden;
}
.ticket-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 16px 20px;
  border-bottom: 1px solid var(--border);
  cursor: pointer;
  transition: background-color var(--transition-fast);
}
.ticket-row:last-child { border-bottom: none; }
.ticket-row:hover { background-color: var(--bg-hover); }

.ticket-main { display: flex; flex-direction: column; gap: 6px; flex: 1; min-width: 0; }
.ticket-title-line { display: flex; align-items: center; gap: 10px; }
.ticket-title {
  font-weight: 600;
  font-size: 14px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.ticket-meta { display: flex; gap: 6px; font-size: 12px; color: var(--text-secondary); }
.ticket-meta .sep { opacity: 0.5; }
.category {
  background-color: var(--bg-hover);
  padding: 1px 6px;
  border-radius: 3px;
}

.ticket-side {
  display: flex;
  align-items: center;
  gap: 12px;
  flex-shrink: 0;
  margin-left: 20px;
}
.ticket-messages {
  font-size: 12px;
  color: var(--text-secondary);
  white-space: nowrap;
}
.ticket-date {
  font-size: 11px;
  color: var(--text-secondary);
  font-family: "JetBrains Mono", "Cascadia Code", monospace;
  white-space: nowrap;
}

.loading, .empty {
  text-align: center;
  color: var(--text-secondary);
  padding: 32px;
}
</style>
