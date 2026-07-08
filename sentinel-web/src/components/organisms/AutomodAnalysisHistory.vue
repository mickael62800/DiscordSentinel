<script setup lang="ts">
import { ref, computed, onMounted, watch, onUnmounted } from "vue";
import { infractionsService } from "@/services/infractionsService";
import { useRealtimeStore } from "@/stores/realtimeStore";
import type { UnlistenFn } from "@/api/events-api";
import type { Infraction } from "@/types";
import { useConfirm } from "@/composables/useConfirm";

const { confirm } = useConfirm();

// ⚠️ Vue de debug temporaire : permet d'analyser pourquoi un message est
// detecte ou non par l'automod (score IA brut, raison, action). A supprimer
// une fois le tuning termine.

const props = defineProps<{
  guildId: string | null;
}>();

const items = ref<Infraction[]>([]);
const loading = ref(false);
const errorMsg = ref<string>("");
const autoRefresh = ref(true);
const wiping = ref(false);
const realtime = useRealtimeStore();
let unlisten: UnlistenFn | null = null;

const detections = computed(() =>
  items.value.filter((i) => i.source === "detection"),
);

async function refresh() {
  if (!props.guildId) {
    items.value = [];
    return;
  }
  loading.value = true;
  errorMsg.value = "";
  try {
    items.value = await infractionsService.getAll(props.guildId);
  } catch (e) {
    errorMsg.value = e instanceof Error ? e.message : String(e);
  } finally {
    loading.value = false;
  }
}

async function wipe() {
  if (!props.guildId) return;
  if (
    !(await confirm({
      title: "Vider l'historique d'analyse",
      message:
        "Cette action supprime TOUTES les infractions de la guilde en base de données (irréversible). Continuer ?",
    }))
  ) {
    return;
  }
  wiping.value = true;
  errorMsg.value = "";
  try {
    await infractionsService.purgeAll(props.guildId);
    items.value = [];
  } catch (e) {
    errorMsg.value = e instanceof Error ? e.message : String(e);
  } finally {
    wiping.value = false;
  }
}

// Live via WebSocket : on s'abonne a l'event `infraction_new` (publie par
// l'API a chaque action automod) et on rafraichit, au lieu d'un polling fixe.
async function startLive() {
  stopLive();
  unlisten = await realtime.onEvent("infraction_new", (data) => {
    const p = data as { guild_id?: string };
    // Rafraichit si l'event concerne la guild affichee (ou s'il n'est pas
    // estampille guild — fallback prudent).
    if (!props.guildId || !p?.guild_id || p.guild_id === props.guildId) {
      refresh();
    }
  });
}
function stopLive() {
  if (unlisten) {
    unlisten();
    unlisten = null;
  }
}

watch(autoRefresh, (v) => (v ? startLive() : stopLive()));
watch(() => props.guildId, refresh);

onMounted(() => {
  refresh();
  if (autoRefresh.value) startLive();
});
onUnmounted(stopLive);

function fmtScore(s: number | undefined): string {
  if (s === undefined || s === null) return "—";
  return s.toFixed(2);
}

function fmtTime(iso: string): string {
  try {
    return new Date(iso).toLocaleTimeString("fr-FR", { hour12: false });
  } catch {
    return iso;
  }
}

function actionClass(action: string | undefined): string {
  switch ((action ?? "").toLowerCase()) {
    case "ban":
      return "act-ban";
    case "mute":
      return "act-mute";
    case "delete":
      return "act-delete";
    case "warn":
      return "act-warn";
    case "none":
      return "act-none";
    default:
      return "";
  }
}

// Code couleur selon score brut, aligne sur les seuils par defaut :
// warn=2, delete=4, mute=6, ban=9.
function scoreClass(s: number | undefined): string {
  if (s === undefined || s === null) return "";
  if (s >= 9) return "score-ban";
  if (s >= 6) return "score-mute";
  if (s >= 4) return "score-delete";
  if (s >= 2) return "score-warn";
  return "score-none";
}
</script>

<template>
  <section class="analysis-history">
    <div class="header">
      <div>
        <h3>Historique d'analyse IA <span class="debug-badge">debug</span></h3>
        <p class="subtitle">
          Les 100 derniers messages analyses par l'automod (table
          <code>infractions</code>). Score = total brut (regex + IA + tension).
          Seuils par defaut : warn≥2 · delete≥4 · mute≥6 · ban≥9.
        </p>
      </div>
      <div class="actions">
        <span class="count">{{ detections.length }} message{{ detections.length > 1 ? "s" : "" }}</span>
        <label class="toggle-auto">
          <input type="checkbox" v-model="autoRefresh" />
          Live
        </label>
        <button class="btn-refresh" :disabled="loading" @click="refresh">
          {{ loading ? "..." : "Rafraichir" }}
        </button>
        <button class="btn-wipe" :disabled="wiping || !props.guildId" @click="wipe">
          {{ wiping ? "..." : "Vider l'historique" }}
        </button>
      </div>
    </div>

    <div v-if="errorMsg" class="err">{{ errorMsg }}</div>

    <div v-if="!loading && detections.length === 0" class="empty">
      Aucun message analyse pour cette guild. Envoie un message dans un salon
      surveille puis clique sur Rafraichir.
    </div>

    <div v-else class="table-wrap">
      <table>
        <thead>
          <tr>
            <th>Heure</th>
            <th>Utilisateur</th>
            <th>Message</th>
            <th class="num">Score</th>
            <th>Action</th>
            <th>Raison IA</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="item in detections" :key="item.id">
            <td class="ts">{{ fmtTime(item.created_at) }}</td>
            <td class="user">{{ item.username }}</td>
            <td class="content">{{ item.content ?? "—" }}</td>
            <td class="num" :class="scoreClass(item.score)">{{ fmtScore(item.score) }}</td>
            <td class="action">
              <span class="action-pill" :class="actionClass(item.infraction_type)">
                {{ item.infraction_type }}
              </span>
            </td>
            <td class="reason">{{ item.reason || "—" }}</td>
          </tr>
        </tbody>
      </table>
    </div>
  </section>
</template>

<style scoped>
.analysis-history {
  margin-top: 32px;
  padding: 20px;
  background: #1a1d24;
  border: 1px dashed #4a5568;
  border-radius: 8px;
}

.header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: 16px;
  margin-bottom: 16px;
}

.header h3 {
  margin: 0 0 4px 0;
  font-size: 16px;
  color: #e2e8f0;
}

.debug-badge {
  display: inline-block;
  font-size: 10px;
  background: #d97706;
  color: #fff;
  padding: 2px 6px;
  border-radius: 3px;
  margin-left: 6px;
  font-weight: bold;
  vertical-align: middle;
}

.subtitle {
  margin: 0;
  font-size: 12px;
  color: #94a3b8;
  max-width: 720px;
  line-height: 1.4;
}

.subtitle code {
  background: #0f1115;
  padding: 1px 5px;
  border-radius: 3px;
  font-size: 11px;
}

.actions {
  display: flex;
  gap: 12px;
  align-items: center;
  white-space: nowrap;
}

.toggle-auto {
  font-size: 12px;
  color: #94a3b8;
  display: flex;
  gap: 4px;
  align-items: center;
}

.count {
  font-size: 11px;
  color: #94a3b8;
  font-family: monospace;
}

.btn-refresh,
.btn-wipe {
  border: none;
  padding: 6px 14px;
  border-radius: 4px;
  font-size: 12px;
  cursor: pointer;
  color: #fff;
}

.btn-refresh {
  background: #2563eb;
}

.btn-wipe {
  background: #dc2626;
  font-weight: 600;
}

.btn-refresh:disabled,
.btn-wipe:disabled {
  background: #475569;
  cursor: not-allowed;
  font-weight: normal;
}

.err {
  background: #7f1d1d;
  color: #fef2f2;
  padding: 8px 12px;
  border-radius: 4px;
  margin-bottom: 12px;
  font-size: 12px;
}

.empty {
  text-align: center;
  padding: 32px;
  color: #64748b;
  font-size: 13px;
}

.table-wrap {
  overflow-x: auto;
  background: #0f1115;
  border-radius: 6px;
}

table {
  width: 100%;
  border-collapse: collapse;
  font-size: 12px;
}

th {
  text-align: left;
  padding: 8px 10px;
  background: #1e2128;
  color: #94a3b8;
  font-weight: 600;
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  border-bottom: 1px solid #334155;
  position: sticky;
  top: 0;
}

td {
  padding: 8px 10px;
  border-bottom: 1px solid #1e2128;
  color: #cbd5e1;
  vertical-align: top;
}

tbody tr:hover {
  background: rgba(255, 255, 255, 0.03);
}

td.ts {
  color: #64748b;
  font-family: monospace;
  font-size: 11px;
  white-space: nowrap;
}

td.user {
  color: #93c5fd;
  font-weight: 500;
  white-space: nowrap;
  max-width: 140px;
  overflow: hidden;
  text-overflow: ellipsis;
}

td.content {
  min-width: 280px;
  max-width: 520px;
  word-break: break-word;
  white-space: pre-wrap;
  font-family: monospace;
  font-size: 13px;
  color: #f1f5f9;
  background: rgba(255, 255, 255, 0.03);
}

td.content:empty::before,
td.content:has(> :empty)::before {
  content: "(vide)";
  color: #64748b;
  font-style: italic;
}

td.reason {
  max-width: 280px;
  font-size: 11px;
  color: #94a3b8;
  word-break: break-word;
}

td.num {
  font-family: monospace;
  text-align: right;
  font-weight: 600;
  white-space: nowrap;
}

.score-none { color: #64748b; }
.score-warn { color: #facc15; }
.score-delete { color: #fb923c; }
.score-mute { color: #f87171; }
.score-ban { color: #ef4444; font-weight: 700; }

.action-pill {
  display: inline-block;
  padding: 2px 8px;
  border-radius: 3px;
  font-size: 10px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  background: #334155;
  color: #cbd5e1;
}

.act-none { background: #1e293b; color: #64748b; }
.act-warn { background: #713f12; color: #fde68a; }
.act-delete { background: #7c2d12; color: #fed7aa; }
.act-mute { background: #7f1d1d; color: #fecaca; }
.act-ban { background: #450a0a; color: #fecaca; }
</style>
