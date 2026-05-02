<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, nextTick, watch } from "vue";

type ServerStatus = "running" | "starting" | "stopped" | "error";

interface GameTemplate {
  id: string;
  name: string;
  category: string;
  image: string;
  accent: string;
  icon: string;
  description: string;
}

interface RunningServer {
  id: string;
  templateId: string;
  name: string;
  status: ServerStatus;
  players: string;
  port: number;
  uptime: string;
  cpu: number;
  ram: number;
}

interface LogLine {
  time: string;
  source: string;
  level: "info" | "warn" | "error" | "sys";
  text: string;
}

const MAX_SLOTS = 10;

const templates: GameTemplate[] = [
  { id: "minecraft", name: "Minecraft", category: "Survie", image: "itzg/minecraft-server", accent: "#5cb85c", icon: "⛏️", description: "Survie vanilla, modpacks Forge/Fabric supportés." },
  { id: "valheim", name: "Valheim", category: "Survie", image: "lloesche/valheim-server", accent: "#d4a017", icon: "🪓", description: "Survie viking coopérative jusqu'à 10 joueurs." },
  { id: "rust", name: "Rust", category: "Survie PvP", image: "didstopia/rust-server", accent: "#cd412b", icon: "🔧", description: "Survie hardcore PvP, wipes hebdomadaires." },
  { id: "ark", name: "ARK: Survival", category: "Survie", image: "hermsi/ark-server", accent: "#3a7ca5", icon: "🦖", description: "Survie dinosaures, mods Steam Workshop." },
  { id: "palworld", name: "Palworld", category: "Survie", image: "thijsvanloef/palworld", accent: "#7d5fff", icon: "🐾", description: "Survie créatures, jusqu'à 32 joueurs." },
  { id: "terraria", name: "Terraria", category: "Aventure", image: "ryshe/terraria", accent: "#46b1c9", icon: "🌳", description: "Bac à sable 2D, exploration et boss." },
  { id: "factorio", name: "Factorio", category: "Gestion", image: "factoriotools/factorio", accent: "#f39c12", icon: "⚙️", description: "Automatisation et logistique industrielle." },
  { id: "csgo", name: "CS2", category: "FPS", image: "joedwards32/cs2", accent: "#e67e22", icon: "🎯", description: "Serveur compétitif Counter-Strike 2." },
];

const servers = ref<RunningServer[]>([
  { id: "srv-01", templateId: "minecraft", name: "Survie-Amis", status: "running", players: "4/20", port: 25565, uptime: "2j 14h", cpu: 23, ram: 58 },
  { id: "srv-02", templateId: "valheim", name: "Vikings-FR", status: "running", players: "2/10", port: 2456, uptime: "6h 12m", cpu: 11, ram: 32 },
  { id: "srv-03", templateId: "terraria", name: "Hardmode", status: "starting", players: "0/8", port: 7777, uptime: "0m", cpu: 4, ram: 9 },
]);

const selectedServerId = ref<string>("srv-01");
const selectedServer = computed(() =>
  servers.value.find((s) => s.id === selectedServerId.value) ?? null,
);

const slotsUsed = computed(() => servers.value.length);
const slotsFree = computed(() => MAX_SLOTS - slotsUsed.value);
const runningCount = computed(() => servers.value.filter((s) => s.status === "running").length);

const templateById = (id: string) => templates.find((t) => t.id === id);

const logs = ref<LogLine[]>([
  { time: "14:02:11", source: "docker", level: "sys", text: "docker engine connecté (v25.0.3)" },
  { time: "14:02:14", source: "srv-01", level: "info", text: "Démarrage container itzg/minecraft-server…" },
  { time: "14:02:21", source: "srv-01", level: "info", text: "Loading properties" },
  { time: "14:02:24", source: "srv-01", level: "info", text: "Default game type: SURVIVAL" },
  { time: "14:02:29", source: "srv-01", level: "info", text: "Done (5.1s)! For help, type \"help\"" },
  { time: "14:05:02", source: "srv-02", level: "info", text: "Valheim server listening on 2456" },
  { time: "14:11:48", source: "srv-03", level: "warn", text: "World file not found, generating new world…" },
]);

const consoleEl = ref<HTMLElement | null>(null);
const cmd = ref("");

function pushLog(line: LogLine) {
  logs.value.push(line);
  if (logs.value.length > 300) logs.value.splice(0, logs.value.length - 300);
}

function nowHHMMSS() {
  return new Date().toTimeString().slice(0, 8);
}

function sendCommand() {
  const text = cmd.value.trim();
  if (!text || !selectedServer.value) return;
  pushLog({ time: nowHHMMSS(), source: selectedServer.value.id, level: "sys", text: `> ${text}` });
  cmd.value = "";
}

function toggleServer(s: RunningServer) {
  if (s.status === "running") {
    s.status = "stopped";
    pushLog({ time: nowHHMMSS(), source: s.id, level: "sys", text: "Container stoppé" });
  } else {
    s.status = "starting";
    pushLog({ time: nowHHMMSS(), source: s.id, level: "info", text: "Démarrage…" });
    setTimeout(() => {
      s.status = "running";
      pushLog({ time: nowHHMMSS(), source: s.id, level: "info", text: "Container prêt" });
    }, 1500);
  }
}

function launchTemplate(t: GameTemplate) {
  if (slotsFree.value <= 0) {
    pushLog({ time: nowHHMMSS(), source: "portal", level: "error", text: `Slots pleins (${MAX_SLOTS}/${MAX_SLOTS})` });
    return;
  }
  const id = `srv-${String(servers.value.length + 1).padStart(2, "0")}`;
  const s: RunningServer = {
    id, templateId: t.id, name: `${t.name}-${id}`, status: "starting",
    players: "0/-", port: 25000 + servers.value.length, uptime: "0m", cpu: 0, ram: 0,
  };
  servers.value.push(s);
  selectedServerId.value = id;
  pushLog({ time: nowHHMMSS(), source: id, level: "sys", text: `docker run ${t.image}` });
  setTimeout(() => {
    s.status = "running";
    pushLog({ time: nowHHMMSS(), source: id, level: "info", text: "Serveur en ligne" });
  }, 2000);
}

let tick: number | undefined;
onMounted(() => {
  tick = window.setInterval(() => {
    const running = servers.value.filter((s) => s.status === "running");
    if (running.length === 0) return;
    const s = running[Math.floor(Math.random() * running.length)];
    s.cpu = Math.max(2, Math.min(95, s.cpu + (Math.random() * 10 - 5)));
    s.ram = Math.max(5, Math.min(95, s.ram + (Math.random() * 6 - 3)));
    const samples = ["Player joined the game", "Saving world…", "Keepalive ok", "Chunk generated", "Tick took 38ms"];
    pushLog({ time: nowHHMMSS(), source: s.id, level: "info", text: samples[Math.floor(Math.random() * samples.length)] });
  }, 2500);
});
onUnmounted(() => { if (tick) window.clearInterval(tick); });

watch(() => logs.value.length, () => {
  nextTick(() => { if (consoleEl.value) consoleEl.value.scrollTop = consoleEl.value.scrollHeight; });
});
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
        <div class="kpi"><span class="kpi-val">{{ slotsUsed }}/{{ MAX_SLOTS }}</span><span class="kpi-lbl">slots utilisés</span></div>
        <div class="kpi ok"><span class="kpi-val">●</span><span class="kpi-lbl">docker connecté</span></div>
      </div>
    </header>

    <main class="grid">
      <section class="panel servers">
        <div class="panel-head">
          <h2>Serveurs actifs</h2>
          <div class="slots-bar" :title="`${slotsUsed} / ${MAX_SLOTS}`">
            <div v-for="i in MAX_SLOTS" :key="i" class="slot" :class="{ filled: i <= slotsUsed }" />
          </div>
        </div>
        <div class="server-list">
          <div v-for="s in servers" :key="s.id" class="server"
               :class="{ active: s.id === selectedServerId }" @click="selectedServerId = s.id">
            <div class="server-icon" :style="{ background: templateById(s.templateId)?.accent }">
              {{ templateById(s.templateId)?.icon }}
            </div>
            <div class="server-info">
              <div class="server-name">
                {{ s.name }}
                <span class="status" :class="s.status">{{ s.status }}</span>
              </div>
              <div class="server-meta">
                {{ templateById(s.templateId)?.name }} · port {{ s.port }} · {{ s.players }} · up {{ s.uptime }}
              </div>
              <div class="bars">
                <div class="bar"><span :style="{ width: s.cpu + '%' }" /></div>
                <div class="bar"><span :style="{ width: s.ram + '%' }" /></div>
              </div>
            </div>
            <button class="btn-icon" @click.stop="toggleServer(s)">
              {{ s.status === 'running' ? '⏹' : '▶' }}
            </button>
          </div>
          <div v-if="servers.length === 0" class="empty">Aucun serveur lancé</div>
        </div>
      </section>

      <section class="panel catalog">
        <div class="panel-head">
          <h2>Catalogue de jeux</h2>
          <span class="hint">{{ slotsFree }} slot(s) libre(s)</span>
        </div>
        <div class="game-grid">
          <article v-for="t in templates" :key="t.id" class="game-card" :style="{ '--accent': t.accent }">
            <div class="game-icon">{{ t.icon }}</div>
            <div class="game-body">
              <div class="game-title">{{ t.name }} <span class="cat">{{ t.category }}</span></div>
              <p class="game-desc">{{ t.description }}</p>
              <code class="img">{{ t.image }}</code>
            </div>
            <button class="btn-launch" :disabled="slotsFree <= 0" @click="launchTemplate(t)">Lancer</button>
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
  --bg: #0e1218; --bg-2: #161c26; --bg-3: #1f2733; --br: #2a3342;
  --tx: #e6ecf3; --tx-2: #8a96a8;
  --ok: #2ecc71; --warn: #f1c40f; --err: #e74c3c; --acc: #4a9eff;
  min-height: 100vh;
  background: radial-gradient(1200px 600px at 10% -10%, #1b2433 0%, var(--bg) 60%);
  color: var(--tx);
  font-family: 'Inter', system-ui, sans-serif;
  padding: 20px; box-sizing: border-box;
}
.topbar {
  display: flex; justify-content: space-between; align-items: center;
  padding: 14px 20px; background: var(--bg-2);
  border: 1px solid var(--br); border-radius: 14px; margin-bottom: 18px;
}
.brand { display: flex; gap: 14px; align-items: center; }
.logo {
  width: 48px; height: 48px; display: grid; place-items: center; font-size: 26px;
  background: linear-gradient(135deg, #4a9eff, #7d5fff); border-radius: 12px;
}
.brand h1 { margin: 0; font-size: 20px; }
.sub { margin: 2px 0 0; color: var(--tx-2); font-size: 13px; }
.kpis { display: flex; gap: 10px; }
.kpi {
  background: var(--bg-3); border: 1px solid var(--br); border-radius: 10px;
  padding: 8px 14px; display: flex; flex-direction: column; align-items: center; min-width: 90px;
}
.kpi-val { font-weight: 700; font-size: 18px; }
.kpi-lbl { font-size: 11px; color: var(--tx-2); text-transform: uppercase; letter-spacing: 0.5px; }
.kpi.ok .kpi-val { color: var(--ok); }
.grid {
  display: grid;
  grid-template-columns: minmax(320px, 380px) 1fr minmax(380px, 480px);
  grid-template-rows: 1fr;
  gap: 18px;
  height: calc(100vh - 200px);
}
.panel {
  background: var(--bg-2); border: 1px solid var(--br); border-radius: 14px;
  padding: 16px; display: flex; flex-direction: column;
}
.panel-head { display: flex; justify-content: space-between; align-items: center; margin-bottom: 12px; }
.panel-head h2 { margin: 0; font-size: 15px; letter-spacing: 0.3px; }
.hint { color: var(--tx-2); font-size: 12px; font-weight: 400; }
.servers { grid-column: 1; grid-row: 1; min-height: 0; }
.catalog { grid-column: 2; grid-row: 1; min-height: 0; }
.console { grid-column: 3; grid-row: 1; min-height: 0; }
.slots-bar { display: flex; gap: 4px; }
.slot { width: 14px; height: 8px; border-radius: 3px; background: var(--bg-3); border: 1px solid var(--br); }
.slot.filled { background: var(--acc); border-color: var(--acc); }
.server-list { display: flex; flex-direction: column; gap: 8px; overflow: auto; flex: 1; min-height: 0; }
.server {
  display: flex; gap: 12px; align-items: center; padding: 10px; border-radius: 10px;
  background: var(--bg-3); border: 1px solid transparent; cursor: pointer; transition: all 0.15s;
}
.server:hover { border-color: var(--br); }
.server.active { border-color: var(--acc); box-shadow: 0 0 0 1px var(--acc) inset; }
.server-icon {
  width: 40px; height: 40px; border-radius: 8px; display: grid; place-items: center;
  font-size: 20px; flex-shrink: 0;
}
.server-info { flex: 1; min-width: 0; }
.server-name { font-weight: 600; font-size: 14px; display: flex; gap: 8px; align-items: center; }
.server-meta { color: var(--tx-2); font-size: 12px; margin-top: 2px; }
.status {
  font-size: 10px; padding: 2px 6px; border-radius: 4px;
  text-transform: uppercase; letter-spacing: 0.5px; font-weight: 700;
}
.status.running { background: rgba(46, 204, 113, 0.15); color: var(--ok); }
.status.starting { background: rgba(241, 196, 15, 0.15); color: var(--warn); }
.status.stopped { background: rgba(138, 150, 168, 0.15); color: var(--tx-2); }
.status.error { background: rgba(231, 76, 60, 0.15); color: var(--err); }
.bars { display: flex; gap: 4px; margin-top: 6px; }
.bar { flex: 1; height: 3px; background: var(--bg); border-radius: 2px; overflow: hidden; }
.bar span { display: block; height: 100%; background: var(--acc); transition: width 0.4s; }
.bar:nth-child(2) span { background: #7d5fff; }
.btn-icon {
  width: 36px; height: 36px; border-radius: 8px;
  background: var(--bg); color: var(--tx); border: 1px solid var(--br);
  cursor: pointer; font-size: 14px;
}
.btn-icon:hover { background: var(--acc); border-color: var(--acc); }
.empty { color: var(--tx-2); text-align: center; padding: 20px; font-size: 13px; }
.game-grid {
  display: grid; grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
  gap: 14px; overflow: auto; flex: 1; min-height: 0; padding-right: 4px; align-content: start;
}
.game-card {
  background: var(--bg-3); border: 1px solid var(--br); border-radius: 12px;
  padding: 14px; display: flex; flex-direction: column; gap: 10px;
  position: relative; overflow: hidden; border-top: 3px solid var(--accent);
}
.game-card::before {
  content: ''; position: absolute; inset: 0;
  background: radial-gradient(300px 100px at 50% -50%, var(--accent), transparent 70%);
  opacity: 0.15; pointer-events: none;
}
.game-icon { font-size: 28px; }
.game-title { font-weight: 700; font-size: 15px; display: flex; gap: 8px; align-items: center; }
.cat {
  font-size: 10px; font-weight: 500; background: var(--bg);
  padding: 2px 6px; border-radius: 4px; color: var(--tx-2);
  text-transform: uppercase; letter-spacing: 0.5px;
}
.game-desc { margin: 0; font-size: 12px; color: var(--tx-2); line-height: 1.4; flex: 1; }
.img {
  font-size: 11px; color: var(--tx-2); background: var(--bg);
  padding: 4px 6px; border-radius: 4px; display: block; word-break: break-all;
}
.btn-launch {
  background: var(--accent); color: #0e1218; border: none; border-radius: 8px;
  padding: 8px; font-weight: 700; cursor: pointer;
  text-transform: uppercase; letter-spacing: 0.5px; font-size: 12px;
}
.btn-launch:disabled { opacity: 0.4; cursor: not-allowed; }
.legend { display: flex; gap: 12px; font-size: 11px; color: var(--tx-2); align-items: center; }
.dot { display: inline-block; width: 8px; height: 8px; border-radius: 50%; margin-right: 4px; }
.dot.info { background: var(--acc); }
.dot.warn { background: var(--warn); }
.dot.error { background: var(--err); }
.console-out {
  flex: 1; background: #06080c; border: 1px solid var(--br); border-radius: 8px;
  padding: 10px; font-family: 'JetBrains Mono', 'Consolas', monospace;
  font-size: 12px; overflow: auto; min-height: 0;
}
.line { display: flex; gap: 8px; padding: 1px 0; }
.line .t { color: #4d5b70; }
.line .s { color: #7d5fff; min-width: 70px; }
.line .m { color: #c8d3e0; flex: 1; word-break: break-word; }
.line.warn .m { color: var(--warn); }
.line.error .m { color: var(--err); }
.line.sys .m { color: var(--ok); }
.console-in {
  display: flex; gap: 6px; align-items: center; margin-top: 10px;
  background: #06080c; border: 1px solid var(--br); border-radius: 8px; padding: 4px 10px;
}
.prompt { color: var(--ok); font-family: monospace; }
.console-in input {
  flex: 1; background: transparent; border: none; outline: none;
  color: var(--tx); font-family: monospace; font-size: 13px; padding: 8px 0;
}
.console-in button {
  background: var(--acc); color: white; border: none; border-radius: 6px;
  padding: 6px 14px; font-weight: 600; cursor: pointer; font-size: 12px;
}
.console-in button:disabled { opacity: 0.4; cursor: not-allowed; }
@media (max-width: 1400px) {
  .grid {
    grid-template-columns: minmax(300px, 360px) 1fr;
    grid-template-rows: 1fr 1fr;
    height: auto; min-height: calc(100vh - 200px);
  }
  .servers { grid-column: 1; grid-row: 1; }
  .catalog { grid-column: 2; grid-row: 1 / span 2; }
  .console { grid-column: 1; grid-row: 2; }
}
@media (max-width: 900px) {
  .grid { grid-template-columns: 1fr; grid-template-rows: auto; }
  .servers, .catalog, .console { grid-column: 1; grid-row: auto; min-height: 400px; }
}
</style>
