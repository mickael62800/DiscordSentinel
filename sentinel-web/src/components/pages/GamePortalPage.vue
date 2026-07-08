<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch } from "vue";
import {
  gamePortalService,
  type GameServer,
  type GameTemplate,
} from "@/services/gamePortalService";
import { useGuildSelector } from "@/composables/useGuildSelector";
import { useRealtimeStore } from "@/stores/realtimeStore";
import type { UnlistenFn } from "@/api/events-api";
import { useToast } from "@/composables/useToast";
import { useConfirm } from "@/composables/useConfirm";
import { useAuth } from "@/composables/useAuth";
import GameServerConfigModal from "@/components/molecules/GameServerConfigModal.vue";
import GameServerSessionsModal from "@/components/molecules/GameServerSessionsModal.vue";
import GamePortalServersPanel from "@/components/organisms/GamePortalServersPanel.vue";
import GamePortalCatalogPanel from "@/components/organisms/GamePortalCatalogPanel.vue";
import GamePortalRolesPanel from "@/components/organisms/GamePortalRolesPanel.vue";
import GamePortalConsolePanel, { type LogLine } from "@/components/organisms/GamePortalConsolePanel.vue";
import GameServerCreateModal from "@/components/organisms/GameServerCreateModal.vue";
import { botConfigService } from "@/services/botConfigService";

const GAME_PORTAL_BOT = "game-portal";
const DEFAULT_POOL_TOTAL_MB = 8192;

const { selectedGuildId } = useGuildSelector();
const realtime = useRealtimeStore();
const { success, error: toastError } = useToast();
const { confirm } = useConfirm();
const { user } = useAuth();

const templates = ref<GameTemplate[]>([]);
const servers = ref<GameServer[]>([]);
const loading = ref(false);
const busy = ref<string | null>(null);

// Modales
const configModalOpen = ref(false);
const configModalServer = ref<GameServer | null>(null);
const configModalDetail = ref<{ template: GameTemplate | null; config: Record<string, string> }>({
  template: null,
  config: {},
});
const sessionsModalOpen = ref(false);
const sessionsModalServer = ref<GameServer | null>(null);

// Modal de creation (remplace les window.prompt)
const createModalOpen = ref(false);
const createModalTemplate = ref<GameTemplate | null>(null);
const suggestedName = ref("");
// Pool RAM total du composant (config bot game-portal, cle max_memory_total_mb).
const poolTotalMb = ref(DEFAULT_POOL_TOTAL_MB);
const poolRemainingMb = computed(
  () =>
    poolTotalMb.value -
    servers.value.reduce((sum, s) => sum + (s.allocated_memory_mb ?? 0), 0),
);

const selectedServerId = ref<string | null>(null);
const selectedServer = computed(() =>
  servers.value.find((s) => s.id === selectedServerId.value) ?? null,
);

const slotsUsed = computed(() => servers.value.length);
const runningCount = computed(
  () => servers.value.filter((s) => s.status === "running").length,
);

// ── Console ──
const logs = ref<LogLine[]>([]);
const cmd = ref("");

function pushLog(line: LogLine) {
  logs.value.push(line);
  if (logs.value.length > 500) logs.value.splice(0, logs.value.length - 500);
}
function nowHHMMSS() {
  return new Date().toTimeString().slice(0, 8);
}

// ── Fetchers ──
// Lit le pool RAM total du composant game-portal via la config bot
// (cle max_memory_total_mb). Defaut 8192 Mo si absente/invalide.
async function fetchPoolTotal() {
  if (!selectedGuildId.value) return;
  try {
    const cfgs = await botConfigService.getGuildConfig(selectedGuildId.value);
    const raw = cfgs.find(
      (c) => c.bot_name === GAME_PORTAL_BOT && c.config_key === "max_memory_total_mb",
    )?.config_value;
    const parsed = raw != null ? Number.parseInt(raw, 10) : NaN;
    poolTotalMb.value = Number.isNaN(parsed) ? DEFAULT_POOL_TOTAL_MB : parsed;
  } catch {
    poolTotalMb.value = DEFAULT_POOL_TOTAL_MB;
  }
}

async function fetchAll() {
  if (!selectedGuildId.value) return;
  loading.value = true;
  try {
    const [tpl, srv] = await Promise.all([
      gamePortalService.listTemplates(selectedGuildId.value),
      gamePortalService.listServers(selectedGuildId.value),
      fetchPoolTotal(),
    ]);
    templates.value = tpl;
    servers.value = srv;
    if (
      !selectedServerId.value ||
      !srv.some((s) => s.id === selectedServerId.value)
    ) {
      selectedServerId.value = srv[0]?.id ?? null;
    }
  } catch (e) {
    toastError(`Erreur chargement: ${e instanceof Error ? e.message : String(e)}`);
  } finally {
    loading.value = false;
  }
}

async function fetchLogs(serverId: string) {
  const srv = servers.value.find((s) => s.id === serverId);
  if (!srv || srv.status === "created" || !srv.started_at) {
    pushLog({
      time: nowHHMMSS(),
      source: serverId.slice(0, 8),
      level: "sys",
      text: "Aucun log : serveur jamais démarré.",
    });
    return;
  }
  try {
    const lines = await gamePortalService.getLogs(serverId, 100);
    logs.value = lines.map((raw) => parseLogLine(raw, serverId));
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    if (msg.includes("409") || msg.includes("container_id")) {
      pushLog({
        time: nowHHMMSS(),
        source: serverId.slice(0, 8),
        level: "sys",
        text: "Aucun log disponible (container pas encore créé).",
      });
      return;
    }
    pushLog({
      time: nowHHMMSS(),
      source: serverId.slice(0, 8),
      level: "error",
      text: `Erreur logs: ${msg}`,
    });
  }
}

function parseLogLine(raw: string, serverId: string): LogLine {
  const tsMatch = raw.match(/^(\S+T\S+)\s+(.*)$/);
  const time = tsMatch
    ? new Date(tsMatch[1]!).toLocaleTimeString("fr-FR")
    : nowHHMMSS();
  const text = tsMatch ? tsMatch[2]! : raw;
  let level: LogLine["level"] = "info";
  if (/\b(WARN|warning)\b/i.test(text)) level = "warn";
  else if (/\b(ERROR|FATAL|fail)\b/i.test(text)) level = "error";
  return { time, source: serverId.slice(0, 8), level, text };
}

// ── Actions ──
async function toggleServer(s: GameServer) {
  busy.value = s.id;
  const actorId = user.value?.id;
  try {
    if (s.status === "running") {
      await gamePortalService.stopServer(s.id, actorId);
      success(`${s.name} arrêté`);
    } else if (
      s.status === "stopped" ||
      s.status === "created" ||
      s.status === "error"
    ) {
      pushLog({
        time: nowHHMMSS(),
        source: s.id.slice(0, 8),
        level: "sys",
        text: "Démarrage…",
      });
      await gamePortalService.startServer(s.id, actorId);
      success(`${s.name} démarré`);
    } else {
      toastError(`Transition impossible depuis ${s.status}`);
      return;
    }
    await fetchAll();
  } catch (e) {
    toastError(`Echec: ${e instanceof Error ? e.message : String(e)}`);
  } finally {
    busy.value = null;
  }
}

// Ouvre le modal de creation (remplace les anciens window.prompt nom + RAM).
function launchTemplate(t: GameTemplate) {
  if (!selectedGuildId.value) {
    toastError("Sélectionne une guild d'abord.");
    return;
  }
  if (!user.value?.id) {
    toastError("Authentification Discord requise.");
    return;
  }
  createModalTemplate.value = t;
  suggestedName.value = `${t.name}-${servers.value.length + 1}`;
  createModalOpen.value = true;
}

async function submitCreate(payload: { name: string; memoryMb: number }) {
  const t = createModalTemplate.value;
  if (!t || !selectedGuildId.value) return;
  const actorId = user.value?.id;
  if (!actorId) {
    toastError("Authentification Discord requise.");
    return;
  }
  busy.value = t.id;
  try {
    const created = await gamePortalService.createServer(selectedGuildId.value, {
      template_slug: t.slug,
      name: payload.name,
      owner_user_id: actorId,
      memory_mb: payload.memoryMb,
    });
    success(`${payload.name} créé. Démarrage…`);
    createModalOpen.value = false;
    selectedServerId.value = created.id;
    pushLog({
      time: nowHHMMSS(),
      source: created.id.slice(0, 8),
      level: "sys",
      text: `docker create ${t.slug}`,
    });
    await gamePortalService.startServer(created.id, actorId);
    await fetchAll();
  } catch (e) {
    toastError(`Echec création: ${e instanceof Error ? e.message : String(e)}`);
  } finally {
    busy.value = null;
  }
}

async function openConfigModal(s: GameServer) {
  busy.value = s.id;
  try {
    const detail = await gamePortalService.getServer(s.id);
    const tpl = templates.value.find((t) => t.id === s.template_id) ?? null;
    configModalServer.value = s;
    configModalDetail.value = { template: tpl, config: detail.config };
    configModalOpen.value = true;
  } catch (e) {
    toastError(`Échec : ${e instanceof Error ? e.message : String(e)}`);
  } finally {
    busy.value = null;
  }
}

function openSessionsModal(s: GameServer) {
  sessionsModalServer.value = s;
  sessionsModalOpen.value = true;
}

async function deleteServer(s: GameServer) {
  const ok = await confirm({
    title: "Supprimer le serveur",
    message: `Supprimer définitivement "${s.name}" ?\n\nLe container Docker et le volume (monde) seront supprimés. Cette action est irréversible.`,
  });
  if (!ok) return;
  busy.value = s.id;
  try {
    await gamePortalService.deleteServer(s.id, user.value?.id);
    success(`${s.name} supprimé`);
    if (selectedServerId.value === s.id) selectedServerId.value = null;
    await fetchAll();
  } catch (e) {
    toastError(`Echec suppression: ${e instanceof Error ? e.message : String(e)}`);
  } finally {
    busy.value = null;
  }
}

async function sendCommand() {
  const text = cmd.value.trim();
  if (!text || !selectedServer.value) return;
  const sid = selectedServer.value.id;
  pushLog({ time: nowHHMMSS(), source: sid.slice(0, 8), level: "sys", text: `> ${text}` });
  cmd.value = "";
  try {
    const resp = await gamePortalService.executeCommand(sid, text, user.value?.id);
    pushLog({
      time: nowHHMMSS(),
      source: sid.slice(0, 8),
      level: "info",
      text: resp.response || "(pas de réponse)",
    });
  } catch (e) {
    pushLog({
      time: nowHHMMSS(),
      source: sid.slice(0, 8),
      level: "error",
      text: `RCON: ${e instanceof Error ? e.message : String(e)}`,
    });
  }
}

// ── Temps reel + poll de secours ──
// Les events `game_server_*` (publies par l'API) declenchent un refetch
// immediat (reactivite cross-client). Le poll reste en secours car les
// transitions de statut (start -> running) et les crashes sont asynchrones
// (Docker/worker) et ne sont pas couverts par des events.
let pollTick: number | undefined;
const unlisteners: UnlistenFn[] = [];

function onServerEvent(data: unknown) {
  const p = data as { guild_id?: string };
  if (!selectedGuildId.value || !p?.guild_id || p.guild_id === selectedGuildId.value) {
    fetchAll();
  }
}

onMounted(async () => {
  await fetchAll();
  pollTick = window.setInterval(fetchAll, 10_000);
  for (const evt of [
    "game_server_created",
    "game_server_started",
    "game_server_stopped",
    "game_server_deleted",
  ]) {
    unlisteners.push(await realtime.onEvent(evt, onServerEvent));
  }
});
onUnmounted(() => {
  if (pollTick) window.clearInterval(pollTick);
  for (const u of unlisteners) u();
  unlisteners.length = 0;
});

watch(selectedGuildId, fetchAll);
watch(selectedServerId, async (sid) => {
  logs.value = [];
  if (sid) await fetchLogs(sid);
});
</script>

<template>
  <div class="portal page--constrained">
    <header class="topbar">
      <div class="brand">
        <span class="logo">🎮</span>
        <div>
          <h1 class="page-title">Game Portal</h1>
          <p class="sub">Gestionnaire de serveurs de jeux Docker</p>
        </div>
      </div>
      <div class="kpis">
        <div class="kpi"><span class="kpi-val">{{ runningCount }}</span><span class="kpi-lbl">en ligne</span></div>
        <div class="kpi"><span class="kpi-val">{{ slotsUsed }}</span><span class="kpi-lbl">serveurs</span></div>
        <div class="kpi ok"><span class="kpi-val">●</span><span class="kpi-lbl">api connectée</span></div>
      </div>
    </header>

    <div v-if="!selectedGuildId" class="empty" style="padding: 40px; text-align: center;">
      Sélectionne une guild dans la barre latérale.
    </div>

    <main v-else class="grid">
      <GamePortalServersPanel
        :servers="servers"
        :templates="templates"
        :loading="loading"
        :busy="busy"
        :selected-server-id="selectedServerId"
        @select="(id) => (selectedServerId = id)"
        @toggle="toggleServer"
        @open-config="openConfigModal"
        @open-sessions="openSessionsModal"
        @remove="deleteServer"
      />

      <GamePortalCatalogPanel
        :templates="templates"
        :busy="busy"
        @launch="launchTemplate"
      />

      <GamePortalConsolePanel
        v-model:cmd="cmd"
        :selected-server="selectedServer"
        :logs="logs"
        @send="sendCommand"
      />
    </main>

    <GamePortalRolesPanel
      v-if="selectedGuildId"
      class="roles-full"
      :templates="templates"
      :guild-id="selectedGuildId ?? null"
    />

    <GameServerConfigModal
      :open="configModalOpen"
      :server-id="configModalServer?.id ?? null"
      :server-name="configModalServer?.name ?? ''"
      :template="configModalDetail.template"
      :initial-config="configModalDetail.config"
      :actor-id="user?.id"
      @close="configModalOpen = false"
      @saved="fetchAll"
    />

    <GameServerSessionsModal
      :open="sessionsModalOpen"
      :server-id="sessionsModalServer?.id ?? null"
      :server-name="sessionsModalServer?.name ?? ''"
      @close="sessionsModalOpen = false"
    />

    <GameServerCreateModal
      :open="createModalOpen"
      :template="createModalTemplate"
      :pool-remaining-mb="poolRemainingMb"
      :suggested-name="suggestedName"
      :busy="busy === createModalTemplate?.id"
      @close="createModalOpen = false"
      @submit="submitCreate"
    />
  </div>
</template>

<style scoped>
.portal {
  color: var(--text-primary);
  display: flex;
  flex-direction: column;
  gap: var(--space-lg);
}

.topbar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: var(--space-lg);
  padding: var(--space-lg) var(--space-xl);
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  flex-wrap: wrap;
}

.brand { display: flex; gap: var(--space-md); align-items: center; }
.logo {
  width: 48px; height: 48px;
  display: grid; place-items: center;
  font-size: 24px;
  background: linear-gradient(135deg, var(--accent), var(--accent-alt));
  border-radius: var(--radius-md);
  box-shadow: var(--shadow-sm);
}
.brand h1 { margin: 0; font-size: 18px; font-weight: 700; }
.sub { margin: 2px 0 0; color: var(--text-secondary); font-size: 12px; }

.kpis { display: flex; gap: var(--space-sm); flex-wrap: wrap; }
.kpi {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  padding: var(--space-sm) var(--space-md);
  display: flex; flex-direction: column; align-items: center;
  min-width: 90px;
}
.kpi-val { font-weight: 700; font-size: 16px; color: var(--text-primary); }
.kpi-lbl {
  font-size: 10px;
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.5px;
  margin-top: 2px;
}
.kpi.ok .kpi-val { color: var(--success); }

.grid {
  display: grid;
  grid-template-columns: minmax(320px, 380px) 1fr minmax(380px, 480px);
  gap: var(--space-lg);
  /* Hauteur fixe pour forcer le scroll interne des panels */
  height: calc(100vh - 240px);
  min-height: 500px;
}

/* Panneau "Role par jeu" : pleine largeur sous la grille (hors des 3 colonnes). */
.roles-full {
  margin-top: var(--space-lg);
}

.empty {
  color: var(--text-secondary);
  text-align: center;
  padding: var(--space-xl);
  font-size: 13px;
}

@media (max-width: 1400px) {
  .grid {
    grid-template-columns: minmax(300px, 360px) 1fr;
    grid-template-rows: 1fr 1fr;
  }
  .grid > :nth-child(1) { grid-column: 1; grid-row: 1; }
  .grid > :nth-child(2) { grid-column: 2; grid-row: 1 / span 2; }
  .grid > :nth-child(3) { grid-column: 1; grid-row: 2; }
}

@media (max-width: 900px) {
  .grid {
    grid-template-columns: 1fr;
    grid-template-rows: auto;
  }
  .grid > * { grid-column: 1; grid-row: auto; min-height: 400px; }
  .topbar { flex-direction: column; align-items: stretch; }
  .kpis { justify-content: center; }
}
</style>
