<script setup lang="ts">
import AppSelect from "@/components/atoms/AppSelect.vue";
import { ref } from "vue";
import { useAuditLogs } from "@/composables/useAuditLogs";
import { useGuildSelector } from "@/composables/useGuildSelector";
import { useConfirm } from "@/composables/useConfirm";
import { useToast } from "@/composables/useToast";
import { useComponentVisibility } from "@/composables/useComponentVisibility";
import { auditLogsService } from "@/services/auditLogsService";
import { eventLabel } from "@/utils/variants";

const { logs, eventTypes, searchQuery, filterEventType, fetchLogs } = useAuditLogs();
const { selectedGuildId } = useGuildSelector();
const { confirm: confirmDialog } = useConfirm();
const { success: toastOk, error: toastErr } = useToast();
const { visible } = useComponentVisibility();

const purging = ref(false);

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
  <div class="filters">
    <input
      v-model="searchQuery"
      type="text"
      class="search-input"
      placeholder="Rechercher par nom, salon..."
    />
    <AppSelect v-model="filterEventType" class="event-select">
      <option value="">Tous les évènements</option>
      <option v-for="t in eventTypes" :key="t" :value="t">{{ eventLabel(t) }}</option>
    </AppSelect>
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
</template>

<style scoped>
.filters { display: flex; gap: 12px; margin-bottom: 20px; }
.search-input {
  flex: 1;
  padding: 8px 12px;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: var(--bg-card);
  color: var(--text-primary);
  font-size: 13px;
}
.search-input::placeholder { color: var(--text-secondary); }
.search-input:focus {
  outline: none;
  border-color: var(--accent);
  box-shadow: var(--focus-ring);
}
.event-select {
  padding: 8px 12px;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: var(--bg-card);
  color: var(--text-primary);
  font-size: 13px;
  cursor: pointer;
  min-width: 200px;
}
.event-select:focus { outline: none; border-color: var(--accent); }
.purge-btn {
  background: transparent;
  color: var(--danger);
  border: 1px solid var(--danger);
  border-radius: var(--radius-sm);
  padding: 8px 14px;
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  white-space: nowrap;
  transition: all var(--transition-fast);
}
.purge-btn:hover:not(:disabled) { background: var(--danger); color: white; }
.purge-btn:disabled { opacity: 0.5; cursor: not-allowed; }
@media (max-width: 768px) {
  .filters { flex-direction: column; gap: 8px; }
  .filters > * { width: 100%; }
}
</style>
