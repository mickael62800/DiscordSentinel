import { computed, ref, watch } from "vue";
import { aiDatasetService, type DatasetMessage } from "@/services/aiDatasetService";
import { useGuildSelector } from "./useGuildSelector";
import { useToast } from "./useToast";

export type DatasetLabel = "skip" | "safe" | "severe";

// Singleton module-scoped : un cache partage entre Filters / StatsBar / Table
// pour que la pagination et les labels restent coherents entre organisms.
const { selectedGuildId } = useGuildSelector();

const items = ref<DatasetMessage[]>([]);
const total = ref(0);
const loading = ref(false);
const exporting = ref(false);

const filterChannel = ref("");
const filterFrom = ref("");
const filterTo = ref("");
const minLen = ref(2);
const limit = ref(200);
const offset = ref(0);

const labels = ref<Record<string, DatasetLabel>>({});
const labeledCache = ref<Record<string, DatasetMessage>>({});

const counts = computed(() => {
  let safe = 0, severe = 0, skip = 0;
  for (const v of Object.values(labels.value)) {
    if (v === "safe") safe++;
    else if (v === "severe") severe++;
    else if (v === "skip") skip++;
  }
  return { safe, severe, skip, total: safe + severe };
});

async function fetchData() {
  const { error: showError } = useToast();
  if (!selectedGuildId.value) return;
  loading.value = true;
  try {
    const r = await aiDatasetService.listMessages(selectedGuildId.value, {
      channel_id: filterChannel.value || undefined,
      from: filterFrom.value || undefined,
      to: filterTo.value || undefined,
      min_length: minLen.value,
      limit: limit.value,
      offset: offset.value,
    });
    items.value = r.items;
    total.value = r.total;
  } catch (e: any) {
    showError(`Erreur chargement : ${e?.message ?? e}`);
  } finally {
    loading.value = false;
  }
}

function getLabel(id: string): DatasetLabel {
  return labels.value[id] ?? "skip";
}
function setLabel(id: string, lbl: DatasetLabel) {
  labels.value = { ...labels.value, [id]: lbl };
}
function markAllVisible(lbl: DatasetLabel) {
  const next = { ...labels.value };
  for (const it of items.value) next[it.id] = lbl;
  labels.value = next;
}

function nextPage() {
  if (offset.value + limit.value < total.value) {
    offset.value += limit.value;
    fetchData();
  }
}
function prevPage() {
  if (offset.value > 0) {
    offset.value = Math.max(0, offset.value - limit.value);
    fetchData();
  }
}

function csvEscape(s: string): string {
  if (s.includes("\"") || s.includes(",") || s.includes("\n") || s.includes("\r")) {
    return `"${s.replace(/"/g, '""')}"`;
  }
  return s;
}
function buildCsv(rows: { text: string; label: string }[]): string {
  const lines = ["text,label"];
  for (const r of rows) lines.push(`${csvEscape(r.text)},${r.label}`);
  return lines.join("\n") + "\n";
}
function downloadCsv(filename: string, content: string): boolean {
  try {
    const blob = new Blob([content], { type: "text/csv;charset=utf-8" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = filename;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    // Revocation differee : revoquer immediatement peut annuler le
    // telechargement dans certains navigateurs (le blob n'est plus lisible
    // avant que l'ecriture disque soit terminee).
    setTimeout(() => URL.revokeObjectURL(url), 40_000);
    return true;
  } catch {
    return false;
  }
}

watch(items, (list) => {
  for (const m of list) labeledCache.value[m.id] = m;
});

async function exportAndClean() {
  const { success, error: showError } = useToast();
  const labeled = Object.entries(labels.value).filter(([, v]) => v !== "skip");
  if (labeled.length === 0) {
    showError("Aucun message labelisé.");
    return;
  }
  if (!confirm(`Exporter ${labeled.length} messages (${counts.value.safe} safe, ${counts.value.severe} severe) puis les supprimer de la BDD ?`)) {
    return;
  }
  exporting.value = true;
  try {
    const safeRows: { text: string; label: string }[] = [];
    const severeRows: { text: string; label: string }[] = [];
    const idsToDelete: string[] = [];
    for (const [id, lbl] of labeled) {
      const m = labeledCache.value[id];
      if (!m) continue;
      idsToDelete.push(id);
      const row = { text: m.content, label: lbl };
      if (lbl === "safe") safeRows.push(row);
      else if (lbl === "severe") severeRows.push(row);
    }

    const stamp = new Date().toISOString().replace(/[:.]/g, "-");
    // Anti-perte de donnees : la suppression est IRREVERSIBLE. On genere
    // d'abord les CSV et on ABANDONNE avant toute suppression si une
    // generation echoue (blob/URL non creables).
    const safeOk =
      safeRows.length === 0 || downloadCsv(`dataset-safe-${stamp}.csv`, buildCsv(safeRows));
    const severeOk =
      severeRows.length === 0 ||
      downloadCsv(`dataset-severe-${stamp}.csv`, buildCsv(severeRows));
    if (!safeOk || !severeOk) {
      showError("Echec de generation des CSV — suppression annulee, aucun message supprime.");
      return;
    }

    // Le navigateur ne garantit pas que le fichier a bien ete enregistre
    // (onglet en arriere-plan, blocage de telechargement...). On exige une
    // confirmation explicite APRES le declenchement du telechargement, avant
    // la suppression irreversible.
    if (
      !confirm(
        `CSV generes (${safeRows.length} safe, ${severeRows.length} severe). ` +
          `Verifie qu'ils sont bien dans tes telechargements. ` +
          `Supprimer maintenant ${idsToDelete.length} messages de la BDD ? (IRREVERSIBLE)`,
      )
    ) {
      showError("Suppression annulee — les messages sont conserves.");
      return;
    }

    const r = await aiDatasetService.bulkDelete(selectedGuildId.value!, idsToDelete);
    success(`${r.deleted} messages exportés et supprimés. (${safeRows.length} safe, ${severeRows.length} severe)`);

    labels.value = {};
    labeledCache.value = {};
    offset.value = 0;
    await fetchData();
  } catch (e: any) {
    showError(`Erreur export/clean : ${e?.message ?? e}`);
  } finally {
    exporting.value = false;
  }
}

watch(selectedGuildId, () => {
  offset.value = 0;
  fetchData();
}, { immediate: true });

export function useAiDataset() {
  return {
    items, total, loading, exporting,
    filterChannel, filterFrom, filterTo, minLen, limit, offset,
    labels, counts,
    fetchData, getLabel, setLabel, markAllVisible,
    nextPage, prevPage, exportAndClean,
  };
}
