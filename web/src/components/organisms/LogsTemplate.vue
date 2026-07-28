<script setup lang="ts">
import AppInput from "@/components/atoms/AppInput.vue";
import { computed, ref } from "vue";
import { useLogs } from "../../composables/useLogs";
import { useRealtimeRefresh } from "../../composables/useRealtimeRefresh";
import { usePagination } from "../../composables/usePagination";
import { useFormatDate } from "../../composables/useFormatDate";
import { useConfirm } from "../../composables/useConfirm";
import { levelVariant } from "../../utils/variants";
import type { LogEntry, TableColumn } from "../../types";
import FilterBar from "../molecules/FilterBar.vue";
import DataTable from "../organisms/DataTable.vue";
import AppBadge from "../atoms/AppBadge.vue";
import LoadingState from "../atoms/LoadingState.vue";
import PaginationBar from "../molecules/PaginationBar.vue";

const props = withDefaults(defineProps<{
  title: string;
  category: string;
  columns: TableColumn[];
  emptyMessage: string;
  showSourceFilter?: boolean;
  sourceLabel?: string;
  showClearButton?: boolean;
  clearConfirmMessage?: string;
  /** Cache le titre h1 (utile quand le parent a deja un titre/onglets). */
  hideTitle?: boolean;
  /** Active la colonne + filtre Type (extrait depuis details/message). */
  showTypeColumn?: boolean;
}>(), {
  showSourceFilter: false,
  sourceLabel: "Toutes les sources",
  showClearButton: false,
  clearConfirmMessage: "Supprimer tous les journaux ?",
  hideTitle: false,
  showTypeColumn: false,
});

const { formatShortDateTime: fmt } = useFormatDate();
const { confirm } = useConfirm();
const { filteredLogs, sources, loading, filterLevel, filterBot, dateFrom, dateTo, search, fetchLogs, clearLogs } = useLogs(props.category);
useRealtimeRefresh(["log_entry_created"], fetchLogs, { debounceMs: 2000 });

const hasDetailsColumn = computed(() => props.columns.some((c) => c.key === "details"));
const hasTypeColumn = computed(() => props.columns.some((c) => c.key === "type"));

/** Derive un type d'event depuis details (event_type / kind) ou depuis le
 *  message via mots-cles. Retourne "general" en dernier recours. */
function inferType(log: LogEntry): string {
  const details = log.details ?? {};
  const fromDetails = (details.event_type ?? details.kind ?? details.event) as string | undefined;
  if (fromDetails && typeof fromDetails === "string" && fromDetails.trim()) return fromDetails;

  const msg = String(log.message ?? "").toLowerCase();
  if (/(joined|join the|a rejoint)/.test(msg)) return "member.join";
  if (/(left|a quitt[eé])/.test(msg)) return "member.leave";
  if (/(banned|banni)/.test(msg)) return "member.ban";
  if (/(kicked|expuls)/.test(msg)) return "member.kick";
  if (/(timeout|mute)/.test(msg)) return "member.timeout";
  if (/(role.*added|added.*role|role ajoute|role assigne)/.test(msg)) return "role.add";
  if (/(role.*removed|removed.*role|role retire|role enleve)/.test(msg)) return "role.remove";
  if (/(channel.*created|created.*channel|salon cr[eé][eé])/.test(msg)) return "channel.create";
  if (/(channel.*deleted|deleted.*channel|salon supprim)/.test(msg)) return "channel.delete";
  if (/(channel.*update|salon modifi)/.test(msg)) return "channel.update";
  if (/(message.*deleted|message supprim)/.test(msg)) return "message.delete";
  if (/(message.*edit|message modifi)/.test(msg)) return "message.edit";
  if (/(voice|vocal)/.test(msg)) return "voice";
  if (/(emoji|sticker)/.test(msg)) return "emoji";
  if (/(invite|invitation)/.test(msg)) return "invite";
  if (/(thread)/.test(msg)) return "thread";
  return "general";
}

const filterType = ref<string>("all");

/** Liste des types presents pour peupler le dropdown du filtre. */
const types = computed<string[]>(() => {
  const set = new Set<string>();
  for (const log of filteredLogs.value) {
    set.add(inferType(log));
  }
  return Array.from(set).sort();
});

/** Logs filtres aussi sur le type quand le filtre est actif. */
const filteredAndTyped = computed(() => {
  if (filterType.value === "all") return filteredLogs.value;
  return filteredLogs.value.filter((log) => inferType(log) === filterType.value);
});

const { currentPage, perPage, totalItems, totalPages, paginatedItems: paginatedLogs } = usePagination(filteredAndTyped, 50);

const filters = computed(() => {
  const list = [
    {
      modelValue: filterLevel.value,
      options: [
        { value: "all", label: "Tous les niveaux" },
        { value: "info", label: "Info" },
        { value: "warn", label: "Avertissement" },
        { value: "error", label: "Erreur" },
      ],
    },
  ];
  // Filtre type (active via showTypeColumn) : place avant le filtre source
  // pour rester dans le meme groupe que niveau.
  if (props.showTypeColumn && types.value.length >= 2) {
    list.push({
      modelValue: filterType.value,
      options: [
        { value: "all", label: "Tous les types" },
        ...types.value.map((t) => ({ value: t, label: t })),
      ],
    });
  }
  // Affiche le filtre source uniquement s'il y a au moins 2 sources distinctes.
  // Avec 0 ou 1 source, le dropdown "Toutes les sources" n'apporte rien.
  if (props.showSourceFilter && sources.value.length >= 2) {
    list.push({
      modelValue: filterBot.value,
      options: [
        { value: "all", label: props.sourceLabel },
        ...sources.value.map((s) => ({ value: s, label: s })),
      ],
    });
  }
  return list;
});

function onFilterUpdate(index: number, value: string) {
  if (index === 0) { filterLevel.value = value; return; }
  // Si le filtre type est actif, il occupe l'index 1 ; sinon c'est le filtre source.
  const typeActive = props.showTypeColumn && types.value.length >= 2;
  if (index === 1) {
    if (typeActive) filterType.value = value;
    else filterBot.value = value;
    return;
  }
  if (index === 2) filterBot.value = value;
}

async function handleClear() {
  const ok = await confirm({ message: props.clearConfirmMessage });
  if (!ok) return;
  await clearLogs();
}
</script>

<template>
  <div class="logs">
    <h1 v-if="!hideTitle">{{ title }}</h1>

    <AppInput v-model="search" type="text" placeholder="Rechercher dans tous les champs..." class="search-global" />

    <div class="filters-row">
      <FilterBar :filters="filters" @update:filter="onFilterUpdate" />
      <div class="date-filters">
        <label>Du <input type="date" v-model="dateFrom" class="date-input" /></label>
        <label>Au <input type="date" v-model="dateTo" class="date-input" /></label>
      </div>
      <button v-if="showClearButton" class="clear-btn" @click="handleClear">Tout supprimer</button>
    </div>

    <LoadingState v-if="loading" />

    <DataTable
      v-else
      :columns="columns"
      :rows="(paginatedLogs as unknown as Record<string, unknown>[])"
      :empty-message="emptyMessage"
    >
      <template #cell-timestamp="{ value }">
        <span class="mono">{{ fmt(String(value)) }}</span>
      </template>
      <template #cell-level="{ value }">
        <AppBadge :label="String(value)" :variant="levelVariant(String(value))" />
      </template>
      <template v-if="hasTypeColumn" #cell-type="{ row }">
        <AppBadge :label="inferType(row as unknown as LogEntry)" variant="info" />
      </template>
      <template v-if="hasDetailsColumn" #cell-details="{ value }">
        <slot name="details" :value="value">
          <span v-if="value && typeof value === 'object' && Object.keys(value as object).length > 0" class="mono details-text">
            {{ Object.entries(value as Record<string, unknown>).map(([k, v]) => `${k}: ${v}`).join(' | ') }}
          </span>
        </slot>
      </template>
    </DataTable>

    <PaginationBar
      v-if="!loading && filteredLogs.length > 0"
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
.logs h1 { margin-bottom: 24px; }
.search-global { width: 100%; padding: 10px 14px; margin-bottom: 12px; border: 1px solid var(--border); border-radius: 8px; background: var(--bg-card); color: var(--text-primary); font-size: 14px; outline: none; }
.search-global:focus { border-color: var(--accent); }
.search-global::placeholder { color: var(--text-secondary); opacity: 0.6; }
.filters-row { display: flex; align-items: flex-start; gap: 16px; flex-wrap: wrap; margin-bottom: 16px; }
.date-filters { display: flex; gap: 12px; align-items: center; }
.date-filters label { display: flex; align-items: center; gap: 6px; font-size: 13px; color: var(--text-secondary); }
.date-input { padding: 6px 10px; border: 1px solid var(--border); border-radius: 6px; background: var(--bg-card); color: var(--text-primary); font-size: 13px; font-family: monospace; }
.date-input:focus { outline: none; border-color: var(--accent); }
.clear-btn { margin-left: auto; padding: 8px 16px; background: var(--danger); color: white; border: none; border-radius: 6px; font-size: 13px; font-weight: 600; cursor: pointer; }
.clear-btn:hover { opacity: 0.85; }
.details-text { font-size: 11px; color: var(--text-secondary); }
</style>
