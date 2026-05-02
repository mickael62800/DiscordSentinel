<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { aiDatasetService, type DatasetMessage } from "@/services/aiDatasetService";
import { useGuildSelector } from "@/composables/useGuildSelector";
import { useToast } from "@/composables/useToast";

type Label = "skip" | "safe" | "severe";

const { selectedGuildId } = useGuildSelector();
const { success, error: showError } = useToast();

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

// Map id -> label (state local, conserve durant la pagination)
const labels = ref<Record<string, Label>>({});

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

function getLabel(id: string): Label {
  return labels.value[id] ?? "skip";
}
function setLabel(id: string, lbl: Label) {
  labels.value = { ...labels.value, [id]: lbl };
}

function markAllVisible(lbl: Label) {
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

// ── CSV ──
function csvEscape(s: string): string {
  // Echappement CSV standard : double les guillemets, encadre si necessaire
  if (s.includes("\"") || s.includes(",") || s.includes("\n") || s.includes("\r")) {
    return `"${s.replace(/"/g, '""')}"`;
  }
  return s;
}
function buildCsv(rows: { text: string; label: string }[]): string {
  const lines = ["text,label"];
  for (const r of rows) {
    lines.push(`${csvEscape(r.text)},${r.label}`);
  }
  return lines.join("\n") + "\n";
}
function downloadCsv(filename: string, content: string) {
  const blob = new Blob([content], { type: "text/csv;charset=utf-8" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = filename;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  URL.revokeObjectURL(url);
}

// Cache des messages chargés cumulés (pour exporter aussi ceux des pages précédentes)
const labeledCache = ref<Record<string, DatasetMessage>>({});
watch(items, (list) => {
  for (const m of list) labeledCache.value[m.id] = m;
});

async function exportAndClean() {
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
    if (safeRows.length > 0) downloadCsv(`dataset-safe-${stamp}.csv`, buildCsv(safeRows));
    if (severeRows.length > 0) downloadCsv(`dataset-severe-${stamp}.csv`, buildCsv(severeRows));

    // Suppression BDD
    const r = await aiDatasetService.bulkDelete(selectedGuildId.value!, idsToDelete);
    success(`${r.deleted} messages exportés et supprimés. (${safeRows.length} safe, ${severeRows.length} severe)`);

    // Reset etat local
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

function truncate(s: string, n: number): string {
  return s.length > n ? s.slice(0, n) + "…" : s;
}
function fmtDate(s: string): string {
  return new Date(s).toLocaleString("fr-FR");
}

onMounted(fetchData);
watch(selectedGuildId, () => {
  offset.value = 0;
  fetchData();
});
</script>

<template>
  <div class="dataset-page">
    <!-- Bloque l'usage en mobile : page d'export AI = desktop only -->
    <div class="mobile-only-block">
      <div class="mobile-block-card">
        <div class="mobile-block-icon">🖥️</div>
        <h2>Disponible sur desktop uniquement</h2>
        <p>
          La collecte et l'export du dataset IA nécessitent un grand écran
          (tableau dense, sélection multi-lignes, export CSV).
        </p>
        <p class="muted">Ouvre cette page depuis ton ordinateur pour continuer.</p>
      </div>
    </div>

    <div class="page-header">
      <h1>📚 Dataset IA — collecte de messages</h1>
      <p class="muted">
        Sélectionne les messages stockés et étiquette-les manuellement. À l'export, deux fichiers CSV
        (<code>safe</code> et <code>severe</code>) sont téléchargés et les messages exportés sont
        supprimés de la base.
      </p>
    </div>

    <!-- Filtres -->
    <section class="card filters">
      <div class="filter">
        <label>Channel ID</label>
        <input v-model="filterChannel" placeholder="(facultatif)" />
      </div>
      <div class="filter">
        <label>Du</label>
        <input v-model="filterFrom" type="datetime-local" />
      </div>
      <div class="filter">
        <label>Au</label>
        <input v-model="filterTo" type="datetime-local" />
      </div>
      <div class="filter">
        <label>Longueur min.</label>
        <input v-model.number="minLen" type="number" min="0" />
      </div>
      <div class="filter">
        <label>Par page</label>
        <select v-model.number="limit">
          <option :value="100">100</option>
          <option :value="200">200</option>
          <option :value="500">500</option>
          <option :value="1000">1000</option>
        </select>
      </div>
      <div class="filter actions">
        <button class="btn" :disabled="loading" @click="offset = 0; fetchData()">🔍 Rechercher</button>
      </div>
    </section>

    <!-- Compteurs + bouton export -->
    <section class="card stats-bar">
      <div class="stat"><span class="lbl">Affichés</span><span class="val">{{ items.length }} / {{ total }}</span></div>
      <div class="stat safe"><span class="lbl">✅ Safe</span><span class="val">{{ counts.safe }}</span></div>
      <div class="stat severe"><span class="lbl">⚠️ Severe</span><span class="val">{{ counts.severe }}</span></div>
      <div class="stat"><span class="lbl">↩ Skip</span><span class="val">{{ counts.skip }}</span></div>
      <div class="grow"></div>
      <button class="btn ghost" @click="markAllVisible('skip')">Tout skip (page)</button>
      <button class="btn ghost" @click="markAllVisible('safe')">Tout safe (page)</button>
      <button class="btn primary" :disabled="exporting || counts.total === 0" @click="exportAndClean">
        {{ exporting ? "Export…" : `📥 Exporter ${counts.total} & nettoyer` }}
      </button>
    </section>

    <!-- Liste messages -->
    <section class="card">
      <div v-if="loading" class="muted">Chargement…</div>
      <div v-else-if="items.length === 0" class="muted">Aucun message correspondant aux filtres.</div>
      <table v-else class="msg-table">
        <thead>
          <tr>
            <th class="lbl-col">Étiquette</th>
            <th>Message</th>
            <th class="meta-col">Channel</th>
            <th class="meta-col">Date</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="m in items" :key="m.id" :class="`row-${getLabel(m.id)}`">
            <td class="lbl-cell">
              <button class="seg" :class="{ active: getLabel(m.id) === 'safe' }" @click="setLabel(m.id, 'safe')" title="Safe">✅</button>
              <button class="seg" :class="{ active: getLabel(m.id) === 'severe' }" @click="setLabel(m.id, 'severe')" title="Severe">⚠️</button>
              <button class="seg" :class="{ active: getLabel(m.id) === 'skip' }" @click="setLabel(m.id, 'skip')" title="Skip">↩</button>
            </td>
            <td class="msg-cell">{{ truncate(m.content, 400) }}</td>
            <td class="small muted">{{ m.channel_name ?? m.channel_id ?? '—' }}</td>
            <td class="small muted">{{ fmtDate(m.created_at) }}</td>
          </tr>
        </tbody>
      </table>

      <div v-if="items.length > 0" class="pagination">
        <button class="btn" :disabled="offset === 0 || loading" @click="prevPage">← Précédent</button>
        <span class="muted">{{ offset + 1 }} – {{ Math.min(offset + items.length, total) }} sur {{ total }}</span>
        <button class="btn" :disabled="offset + limit >= total || loading" @click="nextPage">Suivant →</button>
      </div>
    </section>
  </div>
</template>

<style scoped>
.dataset-page { padding: 16px; }
.page-header h1 { margin: 0 0 4px; }
.muted { color: var(--text-secondary); font-size: 12px; }
.card {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 12px;
  padding: 16px;
  margin-bottom: 16px;
}

.filters { display: flex; flex-wrap: wrap; gap: 12px; align-items: flex-end; }
.filter { display: flex; flex-direction: column; gap: 4px; }
.filter label { font-size: 11px; color: var(--text-secondary); text-transform: uppercase; }
.filter input, .filter select {
  padding: 6px 10px;
  border-radius: 6px;
  border: 1px solid var(--border);
  background: var(--bg-secondary);
  color: var(--text-primary);
  font-size: 12px;
}

.stats-bar { display: flex; gap: 16px; align-items: center; flex-wrap: wrap; }
.stat { display: flex; flex-direction: column; }
.stat .lbl { font-size: 10px; text-transform: uppercase; color: var(--text-secondary); }
.stat .val { font-size: 20px; font-weight: 700; }
.stat.safe .val { color: var(--success, #2ecc71); }
.stat.severe .val { color: var(--danger); }
.grow { flex: 1; }

.btn {
  padding: 7px 14px;
  border-radius: 8px;
  border: 1px solid var(--border);
  background: var(--bg-secondary);
  color: var(--text-primary);
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
}
.btn:hover:not(:disabled) { border-color: var(--accent); color: var(--accent); }
.btn:disabled { opacity: 0.5; cursor: not-allowed; }
.btn.primary { background: var(--accent); color: white; border-color: var(--accent); }
.btn.primary:hover:not(:disabled) { filter: brightness(1.1); color: white; }
.btn.ghost { background: transparent; }

.msg-table { width: 100%; border-collapse: collapse; font-size: 12px; }
.msg-table th, .msg-table td {
  padding: 8px 10px;
  border-bottom: 1px solid color-mix(in srgb, var(--border) 60%, transparent);
  vertical-align: top;
}
.msg-table th {
  text-align: left;
  font-size: 10px;
  text-transform: uppercase;
  color: var(--text-secondary);
}
.lbl-col { width: 130px; }
.meta-col { width: 180px; }
.lbl-cell { white-space: nowrap; }
.msg-cell { word-break: break-word; line-height: 1.5; }
.small { font-size: 11px; }

.seg {
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  padding: 4px 8px;
  border-radius: 6px;
  cursor: pointer;
  margin-right: 2px;
  font-size: 14px;
}
.seg.active { background: var(--accent); border-color: var(--accent); color: white; }

tr.row-safe { background: color-mix(in srgb, var(--success, #2ecc71) 6%, transparent); }
tr.row-severe { background: color-mix(in srgb, var(--danger) 7%, transparent); }

.pagination {
  display: flex;
  justify-content: center;
  align-items: center;
  gap: 16px;
  margin-top: 16px;
}

/* Mobile : on cache toute la page sauf le bloc d'avertissement.
   La page est trop dense pour etre utilisable en mobile (export CSV =
   desktop only). */
.mobile-only-block { display: none; }

@media (max-width: 768px) {
  .mobile-only-block {
    display: flex;
    align-items: center;
    justify-content: center;
    min-height: 60vh;
  }
  .dataset-page > :not(.mobile-only-block) {
    display: none !important;
  }
  .mobile-block-card {
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 12px;
    padding: 32px 20px;
    text-align: center;
    max-width: 360px;
  }
  .mobile-block-icon {
    font-size: 48px;
    margin-bottom: 12px;
  }
  .mobile-block-card h2 {
    margin: 0 0 12px;
    font-size: 18px;
    color: var(--text-primary);
  }
  .mobile-block-card p {
    margin: 0 0 8px;
    font-size: 13px;
    color: var(--text-secondary);
    line-height: 1.5;
  }
}
</style>
