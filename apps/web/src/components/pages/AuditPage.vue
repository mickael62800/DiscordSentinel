<script setup lang="ts">
import { ref } from "vue";
import { useAuditLogs } from "../../composables/useAuditLogs";
import { useRealtimeRefresh } from "../../composables/useRealtimeRefresh";
import { usePagination } from "../../composables/usePagination";
import { useGuildSelector } from "../../composables/useGuildSelector";
import { useConfirm } from "../../composables/useConfirm";
import { useToast } from "../../composables/useToast";
import AppBadge from "../atoms/AppBadge.vue";
import LoadingState from "../atoms/LoadingState.vue";
import EmptyState from "../atoms/EmptyState.vue";
import PaginationBar from "../molecules/PaginationBar.vue";
import AuditEventDetail from "../molecules/AuditEventDetail.vue";
import { eventVariant, eventLabel, eventIcon } from "../../utils/variants";
import { useFormatDate } from "../../composables/useFormatDate";
import { auditLogsService } from "@/services/auditLogsService";
import { useComponentVisibility } from "@/composables/useComponentVisibility";

const { visible } = useComponentVisibility();
const { formatShortDateTime: fmt } = useFormatDate();
const { logs, filteredLogs, eventTypes, loading, filterEventType, searchQuery, fetchLogs } = useAuditLogs();
const { selectedGuildId } = useGuildSelector();
const { confirm: confirmDialog } = useConfirm();
const { success: toastOk, error: toastErr } = useToast();
const purging = ref(false);

useRealtimeRefresh(["log_entry_created"], fetchLogs);
const { currentPage, perPage, totalItems, totalPages, paginatedItems: paginatedLogs } = usePagination(filteredLogs, 30);

async function handlePurgeAll() {
  if (!selectedGuildId.value) return;
  const ok1 = await confirmDialog({
    title: "Vider le journal d'audit",
    message:
      `Supprimer définitivement les ${logs.value.length} entrée(s) du journal d'audit ?\n\n` +
      "Toutes les traces (joins, leaves, modifications, modération) seront effacées de la BDD.",
  });
  if (!ok1) return;
  const ok2 = await confirmDialog({
    title: "Confirmation finale",
    message: "Cette action est IRRÉVERSIBLE. Confirmer la suppression totale ?",
  });
  if (!ok2) return;
  purging.value = true;
  try {
    const res = await auditLogsService.purge(selectedGuildId.value);
    toastOk(`${res.deleted} entrée(s) supprimée(s).`);
    await fetchLogs();
  } catch (e) {
    console.error("Erreur purge audit logs:", e);
    toastErr("Erreur lors du nettoyage du journal d'audit.");
  } finally {
    purging.value = false;
  }
}
</script>

<template>
  <div class="audit">
    <h1>Journal d'audit</h1>

    <div class="filters">
      <input
        v-model="searchQuery"
        type="text"
        class="search-input"
        placeholder="Rechercher par nom, salon..."
      />
      <select v-model="filterEventType" class="event-select">
        <option value="">Tous les évènements</option>
        <option v-for="t in eventTypes" :key="t" :value="t">
          {{ eventLabel(t) }}
        </option>
      </select>
      <button
        v-if="logs.length > 0 && visible('db.purge.audit_logs')"
        class="purge-btn"
        :disabled="purging"
        title="Supprime totalement le journal d'audit en BDD (owner uniquement)"
        @click="handlePurgeAll"
      >
        {{ purging ? "Nettoyage…" : `Tout supprimer (${logs.length})` }}
      </button>
    </div>

    <LoadingState v-if="loading" />

    <div v-else class="audit-list">
      <div v-for="log in paginatedLogs" :key="log.id" class="audit-entry">
        <div :class="['event-icon', `icon--${eventVariant(log.event_type)}`]">
          {{ eventIcon(log.event_type) }}
        </div>
        <div class="entry-content">
          <div class="entry-header">
            <AppBadge :label="eventLabel(log.event_type)" :variant="eventVariant(log.event_type)" />
            <span v-if="log.actor_name" class="actor">
              par <strong>{{ log.actor_name }}</strong>
            </span>
            <span v-if="log.target_name" class="target">
              sur <strong>{{ log.target_name }}</strong>
            </span>
            <span v-if="log.channel_name" class="channel">
              dans <strong>{{ log.channel_name }}</strong>
            </span>
            <span class="timestamp">{{ fmt(log.created_at) }}</span>
          </div>
          <AuditEventDetail :details="log.details" />
        </div>
      </div>

      <EmptyState v-if="filteredLogs.length === 0" message="Aucun evenement d'audit" />

      <PaginationBar
        v-if="filteredLogs.length > 0"
        :current-page="currentPage"
        :total-pages="totalPages"
        :total-items="totalItems"
        :per-page="perPage"
        @update:current-page="currentPage = $event"
        @update:per-page="perPage = $event"
      />
    </div>
  </div>
</template>

<style scoped>
.audit h1 {
  margin-bottom: 20px;
}

.filters {
  display: flex;
  gap: 12px;
  margin-bottom: 20px;
}

.search-input {
  flex: 1;
  padding: 8px 12px;
  border: 1px solid var(--border);
  border-radius: 6px;
  background: var(--bg-card);
  color: var(--text-primary);
  font-size: 13px;
}

.search-input::placeholder {
  color: var(--text-secondary);
}

.search-input:focus {
  outline: none;
  border-color: var(--accent);
  box-shadow: var(--focus-ring);
}

.event-select {
  padding: 8px 12px;
  border: 1px solid var(--border);
  border-radius: 6px;
  background: var(--bg-card);
  color: var(--text-primary);
  font-size: 13px;
  cursor: pointer;
  min-width: 200px;
}

.event-select:focus {
  outline: none;
  border-color: var(--accent);
}

.purge-btn {
  background: transparent;
  color: var(--danger);
  border: 1px solid var(--danger);
  border-radius: 6px;
  padding: 8px 14px;
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  white-space: nowrap;
  transition: all var(--transition-fast);
}

.purge-btn:hover:not(:disabled) {
  background: var(--danger);
  color: white;
}

.purge-btn:disabled { opacity: 0.5; cursor: not-allowed; }

.audit-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.audit-entry {
  background-color: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 10px;
  padding: 14px 18px;
  display: flex;
  gap: 14px;
  align-items: flex-start;
}

.event-icon {
  width: 32px;
  height: 32px;
  border-radius: 8px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-weight: 700;
  font-size: 14px;
  color: white;
  flex-shrink: 0;
}

.icon--danger { background-color: var(--danger); }
.icon--warning { background-color: var(--warning); color: #1a1b2e; }
.icon--success { background-color: var(--success); color: #1a1b2e; }
.icon--info { background-color: var(--info); }
.icon--default { background-color: var(--bg-hover); color: var(--text-secondary); }

.entry-content {
  flex: 1;
  min-width: 0;
}

.entry-header {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
  font-size: 13px;
}

.actor, .target, .channel {
  color: var(--text-secondary);
}

.actor strong, .target strong, .channel strong {
  color: var(--text-primary);
}

.timestamp {
  margin-left: auto;
  font-size: 11px;
  color: var(--text-secondary);
  font-family: "JetBrains Mono", "Cascadia Code", monospace;
  white-space: nowrap;
}

/* Les styles .entry-details / .detail-* / .avatar-* vivent désormais dans
   molecules/AuditEventDetail.vue */

.loading, .empty {
  color: var(--text-secondary);
  padding: 40px;
  text-align: center;
}

/* Texte des entrees audit : autorise le wrap des longues IDs (snowflakes
   19 chiffres) pour eviter le debordement de la card. */
.entry-content,
.entry-content * {
  overflow-wrap: anywhere;
  word-break: break-word;
}

@media (max-width: 768px) {
  .filters {
    flex-direction: column;
    gap: 8px;
  }
  .filters > * {
    width: 100%;
  }
  .audit-entry {
    padding: 10px 12px;
    gap: 10px;
  }
  .event-icon {
    width: 28px;
    height: 28px;
    font-size: 13px;
  }
  /* Reduction taille texte des details */
  .audit-entry {
    font-size: 13px;
  }
}
</style>
