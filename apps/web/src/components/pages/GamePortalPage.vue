<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, nextTick, watch } from "vue";
import {
  gamePortalService,
  type GameServer,
  type GameTemplate,
} from "@/services/gamePortalService";
import { useGuildSelector } from "@/composables/useGuildSelector";
import { useToast } from "@/composables/useToast";
import { useConfirm } from "@/composables/useConfirm";
import { useAuth } from "@/composables/useAuth";
import { useComponentVisibility } from "@/composables/useComponentVisibility";

interface LogLine {
  time: string;
  source: string;
  level: "info" | "warn" | "error" | "sys";
  text: string;
}

const { selectedGuildId } = useGuildSelector();
const { success, error: toastError } = useToast();
const { confirm } = useConfirm();
const { user } = useAuth();
const { visible } = useComponentVisibility();

const templates = ref<GameTemplate[]>([]);
const servers = ref<GameServer[]>([]);
const loading = ref(false);
const busy = ref<string | null>(null);

const selectedServerId = ref<string | null>(null);
const selectedServer = computed(() =>
  servers.value.find((s) => s.id === selectedServerId.value) ?? null,
);

const slotsUsed = computed(() => servers.value.length);
const runningCount = computed(
  () => servers.value.filter((s) => s.status === "running").length,
);

const templateById = (id: string) => templates.value.find((t) => t.id === id);
const templateBySlug = (slug: string) =>
  templates.value.find((t) => t.slug === slug);

// ── Console ──────────────────────────────────────────────────────────
const logs = ref<LogLine[]>([]);
const consoleEl = ref<HTMLElement | null>(null);
const cmd = ref("");

function pushLog(line: LogLine) {
  logs.value.push(line);
  if (logs.value.length > 500) logs.value.splice(0, logs.value.length - 500);
}
function nowHHMMSS() {
  return new Date().toTimeString().slice(0, 8);
}

// ── Fetchers ────────────────────────────────────────────────────────
async function fetchAll() {
  if (!selectedGuildId.value) return;
  loading.value = true;
  try {
    const [tpl, srv] = await Promise.all([
      gamePortalService.listTemplates(selectedGuildId.value),
      gamePortalService.listServers(selectedGuildId.value),
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
  try {
    const lines = await gamePortalService.getLogs(serverId, 100);
    logs.value = lines.map((raw) => parseLogLine(raw, serverId));
  } catch (e) {
    pushLog({
      time: nowHHMMSS(),
      source: serverId.slice(0, 8),
      level: "error",
      text: `Erreur logs: ${e instanceof Error ? e.message : String(e)}`,
    });
  }
}

function parseLogLine(raw: string, serverId: string): LogLine {
  // Format Docker logs avec timestamp ISO devant.
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

// ── Actions ─────────────────────────────────────────────────────────
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

async function launchTemplate(t: GameTemplate) {
  if (!selectedGuildId.value) {
    toastError("Sélectionne une guild d'abord.");
    return;
  }
  const actorId = user.value?.id;
  if (!actorId) {
    toastError("Authentification Discord requise.");
    return;
  }
  // Nom suggere base sur le template + count
  const suggested = `${t.name}-${servers.value.length + 1}`;
  const name = window.prompt(
    `Nom du nouveau serveur ${t.name} ?`,
    suggested,
  );
  if (!name) return;
  busy.value = t.id;
  try {
    const created = await gamePortalService.createServer(
      selectedGuildId.value,
      {
        template_slug: t.slug,
        name,
        owner_user_id: actorId,
      },
    );
    success(`${name} créé. Démarrage…`);
    selectedServerId.value = created.id;
    pushLog({
      time: nowHHMMSS(),
      source: created.id.slice(0, 8),
      level: "sys",
      text: `docker create ${t.slug}`,
    });
    // Auto-start derriere create
    await gamePortalService.startServer(created.id, actorId);
    await fetchAll();
  } catch (e) {
    toastError(`Echec création: ${e instanceof Error ? e.message : String(e)}`);
  } finally {
    busy.value = null;
  }
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
    const resp = await gamePortalService.executeCommand(
      sid,
      text,
      user.value?.id,
    );
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

// ── Polling ─────────────────────────────────────────────────────────
let pollTick: number | undefined;
onMounted(async () => {
  await fetchAll();
  pollTick = window.setInterval(fetchAll, 10_000);
});
onUnmounted(() => {
  if (pollTick) window.clearInterval(pollTick);
});

watch(selectedGuildId, fetchAll);
watch(selectedServerId, async (sid) => {
  logs.value = [];
  if (sid) await fetchLogs(sid);
});
watch(
  () => logs.value.length,
  () => {
    nextTick(() => {
      if (consoleEl.value)
        consoleEl.value.scrollTop = consoleEl.value.scrollHeight;
    });
  },
);

// ── Helpers UI ──────────────────────────────────────────────────────
function formatUptime(server: GameServer): string {
  if (!server.started_at || server.status !== "running") return "—";
  const ms = Date.now() - new Date(server.started_at).getTime();
  const h = Math.floor(ms / 3_600_000);
  const m = Math.floor((ms % 3_600_000) / 60_000);
  if (h >= 24) {
    const d = Math.floor(h / 24);
    return `${d}j ${h % 24}h`;
  }
  return h > 0 ? `${h}h ${m}m` : `${m}m`;
}

function templateAccent(slug: string | undefined): string {
  if (!slug) return "var(--accent)";
  const t = templateBySlug(slug);
  return t?.accent_color ? `#${t.accent_color}` : "var(--accent)";
}

function templateIcon(slug: string | undefined): string {
  if (!slug) return "🎮";
  return templateBySlug(slug)?.icon ?? "🎮";
}
</script>

<template>
  <div class="portal">
    <header class="topbar">
      <div class="brand">
        <span class="logo">🎮</span>
        <div>
          <h1>Game Portal</h1>
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
      <section class="panel servers">
        <div class="panel-head">
          <h2>Serveurs actifs</h2>
          <span class="hint">{{ slotsUsed }} total</span>
        </div>
        <div class="server-list">
          <div v-for="s in servers" :key="s.id" class="server"
               :class="{ active: s.id === selectedServerId }" @click="selectedServerId = s.id">
            <div class="server-icon" :style="{ background: templateAccent(templateById(s.template_id)?.slug) }">
              {{ templateIcon(templateById(s.template_id)?.slug) }}
            </div>
            <div class="server-info">
              <div class="server-name">
                {{ s.name }}
                <span class="status" :class="s.status">{{ s.status }}</span>
              </div>
              <div class="server-meta">
                {{ templateById(s.template_id)?.name ?? '?' }} ·
                <template v-if="s.host_port">port {{ s.host_port }} · </template>
                {{ s.last_player_count }} joueur(s) · up {{ formatUptime(s) }}
              </div>
              <div v-if="s.last_error" class="server-error">⚠ {{ s.last_error }}</div>
            </div>
            <button
              class="btn-icon"
              :disabled="busy === s.id"
              :title="s.status === 'running' ? 'Arrêter' : 'Démarrer'"
              @click.stop="toggleServer(s)"
            >
              {{ s.status === 'running' ? '⏹' : '▶' }}
            </button>
            <button
              v-if="visible('game.server.delete')"
              class="btn-icon btn-icon-danger"
              :disabled="busy === s.id"
              title="Supprimer"
              @click.stop="deleteServer(s)"
            >
              🗑
            </button>
          </div>
          <div v-if="!loading && servers.length === 0" class="empty">Aucun serveur lancé</div>
          <div v-if="loading && servers.length === 0" class="empty">Chargement…</div>
        </div>
      </section>

      <section class="panel catalog">
        <div class="panel-head">
          <h2>Catalogue de jeux</h2>
          <span class="hint">{{ templates.length }} template(s)</span>
        </div>
        <div class="game-grid">
          <article v-for="t in templates" :key="t.id" class="game-card" :style="{ '--accent': '#' + (t.accent_color ?? '5865f2') }">
            <div class="game-icon">{{ t.icon ?? '🎮' }}</div>
            <div class="game-body">
              <div class="game-title">{{ t.name }} <span v-if="t.category" class="cat">{{ t.category }}</span></div>
              <p class="game-desc">{{ t.description ?? '' }}</p>
              <code class="img">{{ t.slug }}</code>
            </div>
            <button
              v-if="visible('game.server.create')"
              class="btn-launch"
              :disabled="busy === t.id"
              @click="launchTemplate(t)"
            >
              {{ busy === t.id ? "…" : "Lancer" }}
            </button>
          </article>
        </div>
      </section>

      <section class="panel console">
        <div class="panel-head">
          <h2>Console <span v-if="selectedServer" class="hint">— {{ selectedServer.name }}</span></h2>
          <div class="legend">
            <span class="dot info" /> info
            <span class="dot warn" /> warn
            <span class="dot error" /> error
          </div>
        </div>
        <div ref="consoleEl" class="console-out">
          <div v-for="(l, i) in logs" :key="i" class="line" :class="l.level">
            <span class="t">{{ l.time }}</span>
            <span class="s">[{{ l.source }}]</span>
            <span class="m">{{ l.text }}</span>
          </div>
        </div>
        <form class="console-in" @submit.prevent="sendCommand">
          <span class="prompt">$</span>
          <input v-model="cmd" type="text"
                 placeholder="Entrez une commande pour le serveur sélectionné…"
                 :disabled="!selectedServer || selectedServer.status !== 'running'" />
          <button type="submit" :disabled="!cmd.trim()">Envoyer</button>
        </form>
      </section>
    </main>
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
  display: flex; justify-content: space-between; align-items: center;
  gap: var(--space-lg);
  padding: var(--space-lg) var(--space-xl);
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  flex-wrap: wrap;
}
.brand { display: flex; gap: var(--space-md); align-items: center; }
.logo {
  width: 48px; height: 48px; display: grid; place-items: center; font-size: 24px;
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
  display: flex; flex-direction: column; align-items: center; min-width: 90px;
}
.kpi-val { font-weight: 700; font-size: 16px; color: var(--text-primary); }
.kpi-lbl { font-size: 10px; color: var(--text-secondary); text-transform: uppercase; letter-spacing: 0.5px; margin-top: 2px; }
.kpi.ok .kpi-val { color: var(--success); }
.grid {
  display: grid;
  grid-template-columns: minmax(320px, 380px) 1fr minmax(380px, 480px);
  gap: var(--space-lg);
  min-height: calc(100vh - 240px);
}
.panel {
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  padding: var(--space-lg);
  display: flex; flex-direction: column;
  min-height: 0;
}
.panel-head { display: flex; justify-content: space-between; align-items: center; gap: var(--space-md); margin-bottom: var(--space-md); }
.panel-head h2 { margin: 0; font-size: 14px; font-weight: 600; letter-spacing: 0.2px; }
.hint { color: var(--text-secondary); font-size: 12px; font-weight: 400; }
.slots-bar { display: flex; gap: 3px; }
.slot {
  width: 14px; height: 8px; border-radius: 3px;
  background: var(--bg-card); border: 1px solid var(--border);
}
.slot.filled { background: var(--accent); border-color: var(--accent); }
.server-list { display: flex; flex-direction: column; gap: var(--space-sm); overflow: auto; flex: 1; min-height: 0; }
.server {
  display: flex; gap: var(--space-md); align-items: center;
  padding: var(--space-md);
  border-radius: var(--radius-md);
  background: var(--bg-card);
  border: 1px solid transparent;
  cursor: pointer;
  transition: var(--transition-fast);
}
.server:hover { border-color: var(--border); background: var(--bg-hover); }
.server.active { border-color: var(--accent); background: var(--accent-bg); }
.server-icon {
  width: 40px; height: 40px;
  border-radius: var(--radius-sm);
  display: grid; place-items: center;
  font-size: 20px; flex-shrink: 0;
  box-shadow: var(--shadow-sm);
}
.server-info { flex: 1; min-width: 0; }
.server-name { font-weight: 600; font-size: 13px; display: flex; gap: var(--space-sm); align-items: center; color: var(--text-primary); }
.server-meta { color: var(--text-secondary); font-size: 11px; margin-top: 2px; }
.status {
  font-size: 9px; padding: 2px 6px;
  border-radius: var(--radius-sm);
  text-transform: uppercase; letter-spacing: 0.5px; font-weight: 700;
}
.status.running { background: var(--success-bg); color: var(--success); }
.status.starting { background: var(--warning-bg); color: var(--warning); }
.status.stopped { background: var(--muted-bg); color: var(--text-secondary); }
.status.error { background: var(--danger-bg); color: var(--danger); }
.bars { display: flex; gap: 4px; margin-top: 6px; }
.bar { flex: 1; height: 3px; background: var(--bg-primary); border-radius: 2px; overflow: hidden; }
.bar span { display: block; height: 100%; background: var(--accent); transition: width 0.4s; }
.bar:nth-child(2) span { background: var(--accent-alt); }
.btn-icon {
  width: 36px; height: 36px;
  border-radius: var(--radius-sm);
  background: var(--bg-primary); color: var(--text-primary);
  border: 1px solid var(--border);
  cursor: pointer; font-size: 14px;
  display: grid; place-items: center;
  transition: var(--transition-fast);
}
.btn-icon:hover:not(:disabled) { background: var(--accent); border-color: var(--accent); color: #fff; }
.btn-icon:disabled { opacity: 0.4; cursor: not-allowed; }
.btn-icon-danger { margin-left: 4px; }
.btn-icon-danger:hover:not(:disabled) { background: var(--danger); border-color: var(--danger); color: #fff; }
.server-error { font-size: 11px; color: var(--danger); margin-top: 4px; word-break: break-word; }
.empty { color: var(--text-secondary); text-align: center; padding: var(--space-xl); font-size: 13px; }
.game-grid {
  display: grid; grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
  gap: var(--space-md); overflow: auto; flex: 1; min-height: 0;
  padding-right: 4px; align-content: start;
}
.game-card {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  padding: var(--space-md);
  display: flex; flex-direction: column; gap: var(--space-sm);
  position: relative; overflow: hidden;
  border-top: 3px solid var(--accent);
  transition: var(--transition-fast);
}
.game-card:hover {
  transform: translateY(-2px);
  box-shadow: var(--shadow-md);
  border-color: var(--accent);
}
.game-card::before {
  content: ''; position: absolute; inset: 0;
  background: radial-gradient(300px 100px at 50% -50%, var(--accent), transparent 70%);
  opacity: 0.12; pointer-events: none;
}
.game-icon { font-size: 26px; }
.game-title { font-weight: 700; font-size: 14px; display: flex; gap: var(--space-sm); align-items: center; color: var(--text-primary); }
.cat {
  font-size: 9px; font-weight: 600;
  background: var(--bg-primary);
  padding: 2px 6px;
  border-radius: var(--radius-sm);
  color: var(--text-secondary);
  text-transform: uppercase; letter-spacing: 0.5px;
}
.game-desc { margin: 0; font-size: 12px; color: var(--text-secondary); line-height: 1.4; flex: 1; }
.img {
  font-size: 11px; color: var(--text-secondary);
  background: var(--bg-primary);
  padding: 4px 6px;
  border-radius: var(--radius-sm);
  display: block; word-break: break-all;
  font-family: 'JetBrains Mono', 'Cascadia Code', monospace;
}
.btn-launch {
  background: var(--accent); color: #fff; border: none;
  border-radius: var(--radius-sm);
  padding: 8px; font-weight: 700; cursor: pointer;
  text-transform: uppercase; letter-spacing: 0.5px; font-size: 11px;
  transition: var(--transition-fast);
}
.btn-launch:hover:not(:disabled) { background: var(--accent-hover); }
.btn-launch:disabled { opacity: 0.4; cursor: not-allowed; }
.legend { display: flex; gap: var(--space-md); font-size: 11px; color: var(--text-secondary); align-items: center; }
.dot { display: inline-block; width: 8px; height: 8px; border-radius: 50%; margin-right: 4px; }
.dot.info { background: var(--info); }
.dot.warn { background: var(--warning); }
.dot.error { background: var(--danger); }
.console-out {
  flex: 1; background: var(--bg-primary);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  padding: var(--space-md);
  font-family: 'JetBrains Mono', 'Cascadia Code', 'Consolas', monospace;
  font-size: 12px; overflow: auto; min-height: 0;
}
.line { display: flex; gap: var(--space-sm); padding: 1px 0; }
.line .t { color: var(--text-secondary); opacity: 0.6; }
.line .s { color: var(--accent-alt); min-width: 70px; }
.line .m { color: var(--text-primary); flex: 1; word-break: break-word; }
.line.warn .m { color: var(--warning); }
.line.error .m { color: var(--danger); }
.line.sys .m { color: var(--success); }
.console-in {
  display: flex; gap: var(--space-sm); align-items: center;
  margin-top: var(--space-md);
  background: var(--bg-primary);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  padding: 4px var(--space-md);
  transition: border-color var(--transition-fast);
}
.console-in:focus-within { border-color: var(--accent); }
.prompt { color: var(--success); font-family: 'JetBrains Mono', monospace; font-weight: 700; }
.console-in input {
  flex: 1; background: transparent; border: none; outline: none;
  color: var(--text-primary);
  font-family: 'JetBrains Mono', 'Cascadia Code', monospace;
  font-size: 13px; padding: 8px 0;
}
.console-in button {
  background: var(--accent); color: #fff; border: none;
  border-radius: var(--radius-sm);
  padding: 6px 14px; font-weight: 600; cursor: pointer; font-size: 12px;
  transition: var(--transition-fast);
}
.console-in button:hover:not(:disabled) { background: var(--accent-hover); }
.console-in button:disabled { opacity: 0.4; cursor: not-allowed; }
@media (max-width: 1400px) {
  .grid {
    grid-template-columns: minmax(300px, 360px) 1fr;
    grid-template-rows: 1fr 1fr;
  }
  .servers { grid-column: 1; grid-row: 1; }
  .catalog { grid-column: 2; grid-row: 1 / span 2; }
  .console { grid-column: 1; grid-row: 2; }
}
@media (max-width: 900px) {
  .grid { grid-template-columns: 1fr; grid-template-rows: auto; }
  .servers, .catalog, .console { grid-column: 1; grid-row: auto; min-height: 400px; }
  .topbar { flex-direction: column; align-items: stretch; }
  .kpis { justify-content: center; }
}
</style>
