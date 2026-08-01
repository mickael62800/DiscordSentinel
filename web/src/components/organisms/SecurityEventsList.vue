<script setup lang="ts">
import { ref } from "vue";
import { useSecurity } from "@/composables/useSecurity";
import { useGuildSelector } from "@/composables/useGuildSelector";
import { useConfirm } from "@/composables/useConfirm";
import { useToast } from "@/composables/useToast";
import { useRealtimeRefresh } from "@/composables/useRealtimeRefresh";
import { useSearch } from "@/composables/useSearch";
import { securityService } from "@/services/securityService";
import type { SecurityEvent } from "@/types";
import AppBadge from "@/components/atoms/AppBadge.vue";
import LoadingState from "@/components/atoms/LoadingState.vue";
import ErrorState from "@/components/atoms/ErrorState.vue";
import EmptyState from "@/components/atoms/EmptyState.vue";
import { severityVariant } from "@/utils/variants";
import { useFormatDate } from "@/composables/useFormatDate";
import { useComponentVisibility } from "@/composables/useComponentVisibility";

const { visible } = useComponentVisibility();
const { formatShortDateTime: fmt } = useFormatDate();
const { events, loading, error, fetchEvents } = useSecurity();
const { selectedGuildId } = useGuildSelector();
const { confirm: confirmDialog } = useConfirm();
const { success: toastOk, error: toastErr } = useToast();
const purging = ref(false);

useRealtimeRefresh(["security_event"], fetchEvents);

const { search, filtered: filteredEvents } = useSearch<SecurityEvent>(
  events,
  ["event_type", "severity", "description", "created_at", (e) => e.user_ids?.join(" ")],
);

async function handlePurgeAll() {
  if (!selectedGuildId.value) return;
  const ok1 = await confirmDialog({
    title: "Nettoyer les evenements de securite",
    message:
      `Supprimer definitivement les ${events.value.length} evenement(s) de securite ?\n\n` +
      "Les utilisateurs ajoutes automatiquement en surveillance par ces evenements seront aussi retires.",
  });
  if (!ok1) return;
  const ok2 = await confirmDialog({
    title: "Confirmation finale",
    message: "Cette action est IRREVERSIBLE. Confirmer la suppression ?",
  });
  if (!ok2) return;
  purging.value = true;
  try {
    const res = await securityService.purge(selectedGuildId.value);
    toastOk(`${res.deleted_events} evenement(s) et ${res.deleted_watches} surveillance(s) supprimes.`);
    await fetchEvents();
  } catch (e) {
    console.error("Erreur purge security:", e);
    toastErr("Erreur lors du nettoyage.");
  } finally {
    purging.value = false;
  }
}

function eventIcon(type: string): string {
  switch (type) {
    case "raid_detected": return "R";
    case "suspicious_account": return "?";
    case "mass_ban": return "!";
    default: return "S";
  }
}
</script>

<template>
  <section class="section">
    <div class="section-head">
      <h2>Evenements</h2>
      <input
        v-model="search"
        type="text"
        placeholder="Rechercher..."
        class="search-input"
      />
      <button
        v-if="events.length > 0 && visible('db.purge.security_events')"
        class="purge-btn"
        :disabled="purging"
        title="Supprime tous les evenements de securite et les surveillances auto (owner uniquement)"
        @click="handlePurgeAll"
      >
        {{ purging ? "Nettoyage…" : `Tout nettoyer (${events.length})` }}
      </button>
    </div>

    <ErrorState v-if="error" :message="error" :retryable="true" @retry="fetchEvents" />
    <LoadingState v-else-if="loading" />

    <div v-else class="events-list">
      <div v-for="event in filteredEvents" :key="event.id" class="card event-card">
        <div :class="['event-icon', `icon--${event.severity}`]">
          {{ eventIcon(event.event_type) }}
        </div>
        <div class="event-content">
          <div class="event-header">
            <span class="event-type">{{ event.event_type.replace("_", " ") }}</span>
            <AppBadge :label="event.severity" :variant="severityVariant(event.severity)" />
            <span class="event-time">{{ fmt(event.created_at) }}</span>
          </div>
          <p class="event-description">{{ event.description }}</p>
          <div v-if="event.user_ids?.length > 0" class="event-users">
            <span class="users-label">Utilisateurs concernes :</span>
            <span v-for="uid in event.user_ids" :key="uid" class="user-chip">{{ uid }}</span>
          </div>
        </div>
      </div>

      <EmptyState v-if="filteredEvents.length === 0" message="Aucun evenement de securite" />
    </div>
  </section>
</template>

<style scoped>
.section { margin-bottom: 32px; }
.section-head {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 6px;
}
.section-head h2 {
  font-size: 18px;
  font-weight: 700;
  color: var(--text-primary);
  margin: 0;
}
.search-input {
  margin-left: auto;
  padding: 8px 12px;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: var(--bg-card);
  color: var(--text-primary);
  font-size: 13px;
  min-width: 240px;
}
.search-input:focus { border-color: var(--accent); outline: none; }
.purge-btn {
  background: transparent;
  color: var(--danger);
  border: 1px solid var(--danger);
  border-radius: var(--radius-sm);
  padding: 7px 14px;
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  transition: all var(--transition-fast);
}
.purge-btn:hover:not(:disabled) { background: var(--danger); color: white; }
.purge-btn:disabled { opacity: 0.5; cursor: not-allowed; }
.events-list {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(340px, 1fr));
  gap: 16px;
}
.event-card {
  padding: 16px 18px;
  display: flex;
  gap: 14px;
  transition: border-color var(--transition-fast), transform var(--transition-fast);
}
.event-card:hover { border-color: var(--accent); transform: translateY(-2px); }
.event-icon {
  width: 40px; height: 40px;
  border-radius: var(--radius-md);
  display: flex;
  align-items: center;
  justify-content: center;
  font-weight: 700;
  font-size: 16px;
  color: white;
  flex-shrink: 0;
}
.icon--critical { background-color: var(--danger); }
.icon--high { background-color: var(--warning); }
.icon--medium { background-color: var(--info); }
.icon--low { background-color: var(--bg-hover); color: var(--text-secondary); }
.event-content { flex: 1; }
.event-header {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 8px;
}
.event-type {
  font-weight: 600;
  font-size: 14px;
  text-transform: capitalize;
}
.event-time {
  margin-left: auto;
  font-size: 11px;
  color: var(--text-secondary);
  font-family: "JetBrains Mono", "Cascadia Code", monospace;
}
.event-description {
  font-size: 13px;
  color: var(--text-secondary);
  line-height: 1.5;
  margin-bottom: 8px;
}
.event-users {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-wrap: wrap;
}
.users-label { font-size: 11px; color: var(--text-secondary); }
.user-chip {
  font-size: 11px;
  padding: 2px 8px;
  border-radius: var(--radius-sm);
  background-color: var(--bg-hover);
  color: var(--text-secondary);
  font-family: monospace;
}
@media (max-width: 640px) {
  .search-input { min-width: 0; width: 100%; margin-left: 0; }
}
</style>
