<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { dockerService, type DockerContainer, type DockerImage, type DockerNetwork, type DockerOverview, type DockerVolume } from "@/services/dockerService";
import { useToast } from "@/composables/useToast";
import { useComponentVisibility } from "@/composables/useComponentVisibility";

const { visible } = useComponentVisibility();

const { success, error: showError } = useToast();

type Tab = "overview" | "containers" | "images" | "volumes" | "networks" | "prune";
const tab = ref<Tab>("overview");

const overview = ref<DockerOverview | null>(null);
const containers = ref<DockerContainer[]>([]);
const images = ref<DockerImage[]>([]);
const volumes = ref<DockerVolume[]>([]);
const networks = ref<DockerNetwork[]>([]);
const loading = ref(false);
const busy = ref(false);

const showOnlyDangling = ref(false);
const showOnlyUnused = ref(false);
const filterContainerState = ref<"all" | "running" | "stopped">("all");

// ── Logs modal ──
const logsOpen = ref(false);
const logsContainer = ref<DockerContainer | null>(null);
const logsContent = ref("");
const logsTail = ref(200);
const logsLoading = ref(false);

async function refreshTab() {
  loading.value = true;
  try {
    if (tab.value === "overview") overview.value = await dockerService.getOverview();
    else if (tab.value === "containers") containers.value = await dockerService.listContainers(true);
    else if (tab.value === "images") images.value = await dockerService.listImages();
    else if (tab.value === "volumes") volumes.value = await dockerService.listVolumes();
    else if (tab.value === "networks") networks.value = await dockerService.listNetworks();
    else if (tab.value === "prune") overview.value = await dockerService.getOverview();
  } catch (e: any) {
    console.error(e);
    showError(`Erreur Docker : ${e?.message ?? e}`);
  } finally {
    loading.value = false;
  }
}

function setTab(t: Tab) {
  tab.value = t;
  refreshTab();
}

let pollHandle: number | null = null;
function startPoll() {
  if (pollHandle !== null) return;
  pollHandle = window.setInterval(refreshTab, 120_000);
}
function stopPoll() {
  if (pollHandle !== null) {
    clearInterval(pollHandle);
    pollHandle = null;
  }
}
onMounted(() => {
  refreshTab();
  startPoll();
});
onUnmounted(stopPoll);

// ── Helpers ──
function fmtBytes(b: number | null | undefined): string {
  if (!b || b < 0) return "—";
  const u = ["B", "KB", "MB", "GB", "TB"];
  let v = b;
  let i = 0;
  while (v >= 1024 && i < u.length - 1) {
    v /= 1024;
    i++;
  }
  return `${v.toFixed(v < 10 && i > 0 ? 2 : 1)} ${u[i]}`;
}
function fmtTs(unix: number): string {
  if (!unix) return "—";
  return new Date(unix * 1000).toLocaleString("fr-FR");
}
function shortId(id: string): string {
  return id.replace(/^sha256:/, "").slice(0, 12);
}
function cleanName(n: string): string {
  return n.replace(/^\//, "");
}

const filteredContainers = computed(() => {
  if (filterContainerState.value === "running") return containers.value.filter((c) => c.state === "running");
  if (filterContainerState.value === "stopped") return containers.value.filter((c) => c.state !== "running");
  return containers.value;
});
const filteredImages = computed(() =>
  showOnlyDangling.value ? images.value.filter((i) => i.dangling || i.containers === 0) : images.value,
);
const filteredVolumes = computed(() =>
  showOnlyUnused.value ? volumes.value.filter((v) => !v.in_use) : volumes.value,
);

// ── Actions ──
async function doConfirm(msg: string): Promise<boolean> {
  return window.confirm(msg);
}

async function startCt(c: DockerContainer) {
  busy.value = true;
  try {
    await dockerService.startContainer(c.id);
    success(`Conteneur ${cleanName(c.names[0] ?? "")} démarré.`);
    await refreshTab();
  } catch (e: any) {
    showError(`Erreur start : ${e?.message ?? e}`);
  } finally {
    busy.value = false;
  }
}
async function stopCt(c: DockerContainer) {
  if (!(await doConfirm(`Arrêter ${cleanName(c.names[0] ?? c.id)} ?`))) return;
  busy.value = true;
  try {
    await dockerService.stopContainer(c.id);
    success("Conteneur arrêté.");
    await refreshTab();
  } catch (e: any) {
    showError(`Erreur stop : ${e?.message ?? e}`);
  } finally {
    busy.value = false;
  }
}
async function restartCt(c: DockerContainer) {
  busy.value = true;
  try {
    await dockerService.restartContainer(c.id);
    success("Conteneur redémarré.");
    await refreshTab();
  } catch (e: any) {
    showError(`Erreur restart : ${e?.message ?? e}`);
  } finally {
    busy.value = false;
  }
}
async function removeCt(c: DockerContainer) {
  const force = c.state === "running";
  if (!(await doConfirm(`Supprimer ${cleanName(c.names[0] ?? c.id)} ?${force ? " (force)" : ""}`))) return;
  busy.value = true;
  try {
    await dockerService.removeContainer(c.id, force, false);
    success("Conteneur supprimé.");
    await refreshTab();
  } catch (e: any) {
    showError(`Erreur delete : ${e?.message ?? e}`);
  } finally {
    busy.value = false;
  }
}

async function openLogs(c: DockerContainer) {
  logsContainer.value = c;
  logsOpen.value = true;
  await fetchLogs();
}
async function fetchLogs() {
  if (!logsContainer.value) return;
  logsLoading.value = true;
  try {
    const r = await dockerService.containerLogs(logsContainer.value.id, logsTail.value, true);
    logsContent.value = r.logs;
  } catch (e: any) {
    showError(`Erreur logs : ${e?.message ?? e}`);
  } finally {
    logsLoading.value = false;
  }
}
function closeLogs() {
  logsOpen.value = false;
  logsContent.value = "";
  logsContainer.value = null;
}

async function removeImg(img: DockerImage) {
  const tag = img.repo_tags[0] ?? shortId(img.id);
  if (!(await doConfirm(`Supprimer image ${tag} ?`))) return;
  busy.value = true;
  try {
    await dockerService.removeImage(img.id, false);
    success("Image supprimée.");
    await refreshTab();
  } catch (e: any) {
    showError(`Erreur : ${e?.message ?? e}`);
  } finally {
    busy.value = false;
  }
}
async function removeVol(v: DockerVolume) {
  if (!(await doConfirm(`Supprimer volume ${v.name} ?`))) return;
  busy.value = true;
  try {
    await dockerService.removeVolume(v.name, false);
    success("Volume supprimé.");
    await refreshTab();
  } catch (e: any) {
    showError(`Erreur : ${e?.message ?? e}`);
  } finally {
    busy.value = false;
  }
}

// ── Prune ──
async function pruneContainers() {
  if (!(await doConfirm("Supprimer tous les conteneurs arrêtés ?"))) return;
  busy.value = true;
  try {
    const r = await dockerService.pruneContainers();
    success(`${r.deleted.length} conteneurs supprimés · ${fmtBytes(r.space_reclaimed_bytes)} libérés.`);
    await refreshTab();
  } catch (e: any) { showError(`Erreur : ${e?.message ?? e}`); } finally { busy.value = false; }
}
async function pruneImages(all: boolean) {
  const msg = all ? "Supprimer toutes les images non utilisées ?" : "Supprimer les images dangling ?";
  if (!(await doConfirm(msg))) return;
  busy.value = true;
  try {
    const r = await dockerService.pruneImages(all);
    success(`${r.deleted.length} images supprimées · ${fmtBytes(r.space_reclaimed_bytes)} libérés.`);
    await refreshTab();
  } catch (e: any) { showError(`Erreur : ${e?.message ?? e}`); } finally { busy.value = false; }
}
async function pruneVolumes() {
  if (!(await doConfirm("⚠️ Supprimer tous les volumes orphelins ? Données potentiellement perdues."))) return;
  busy.value = true;
  try {
    const r = await dockerService.pruneVolumes();
    success(`${r.deleted.length} volumes supprimés · ${fmtBytes(r.space_reclaimed_bytes)} libérés.`);
    await refreshTab();
  } catch (e: any) { showError(`Erreur : ${e?.message ?? e}`); } finally { busy.value = false; }
}
async function pruneNetworks() {
  if (!(await doConfirm("Supprimer les réseaux non utilisés ?"))) return;
  busy.value = true;
  try {
    const r = await dockerService.pruneNetworks();
    success(`${r.deleted.length} réseaux supprimés.`);
    await refreshTab();
  } catch (e: any) { showError(`Erreur : ${e?.message ?? e}`); } finally { busy.value = false; }
}
async function pruneSystem(includeVolumes: boolean, allImages: boolean) {
  let msg = "Nettoyage système complet : conteneurs arrêtés + images";
  msg += allImages ? " (toutes inutilisées)" : " dangling";
  msg += " + réseaux";
  if (includeVolumes) msg += " + volumes orphelins ⚠️";
  msg += ". Continuer ?";
  if (!(await doConfirm(msg))) return;
  busy.value = true;
  try {
    const r = await dockerService.pruneSystem({ volumes: includeVolumes, allImages });
    success(`Nettoyage : ${fmtBytes(r.total_space_reclaimed_bytes)} libérés.`);
    await refreshTab();
  } catch (e: any) { showError(`Erreur : ${e?.message ?? e}`); } finally { busy.value = false; }
}
</script>

<template>
  <section class="docker-section">
    <div class="docker-header">
      <h2 class="section-title">🐳 Docker</h2>
      <div class="tabs">
        <button :class="{ active: tab === 'overview' }" @click="setTab('overview')">Vue d'ensemble</button>
        <button :class="{ active: tab === 'containers' }" @click="setTab('containers')">Conteneurs</button>
        <button :class="{ active: tab === 'images' }" @click="setTab('images')">Images</button>
        <button :class="{ active: tab === 'volumes' }" @click="setTab('volumes')">Volumes</button>
        <button :class="{ active: tab === 'networks' }" @click="setTab('networks')">Réseaux</button>
        <button :class="{ active: tab === 'prune' }" @click="setTab('prune')">🧹 Nettoyage</button>
      </div>
    </div>

    <div v-if="loading" class="muted">Chargement…</div>

    <!-- ── Overview ── -->
    <div v-else-if="tab === 'overview' && overview" class="overview-grid">
      <div class="ov-card">
        <div class="ov-label">Version Docker</div>
        <div class="ov-value">{{ overview.version }}</div>
        <div class="ov-sub">API {{ overview.api_version }} · {{ overview.os }}/{{ overview.arch }}</div>
        <div class="ov-sub">Kernel : {{ overview.kernel }}</div>
      </div>
      <div class="ov-card">
        <div class="ov-label">Conteneurs</div>
        <div class="ov-value">{{ overview.containers_running }} / {{ overview.containers_running + overview.containers_paused + overview.containers_stopped }}</div>
        <div class="ov-sub">{{ overview.containers_running }} running · {{ overview.containers_paused }} paused · {{ overview.containers_stopped }} stopped</div>
        <div class="ov-sub">Taille writable : {{ fmtBytes(overview.containers_size_bytes) }}</div>
      </div>
      <div class="ov-card">
        <div class="ov-label">Images</div>
        <div class="ov-value">{{ overview.images_count }}</div>
        <div class="ov-sub">Taille totale : {{ fmtBytes(overview.images_size_bytes) }}</div>
        <div class="ov-sub reclaimable">Récupérables : {{ fmtBytes(overview.reclaimable_images_bytes) }}</div>
      </div>
      <div class="ov-card">
        <div class="ov-label">Volumes</div>
        <div class="ov-value">{{ overview.volumes_count }}</div>
        <div class="ov-sub">Taille totale : {{ fmtBytes(overview.volumes_size_bytes) }}</div>
        <div class="ov-sub reclaimable">Récupérables : {{ fmtBytes(overview.reclaimable_volumes_bytes) }}</div>
      </div>
      <div class="ov-card">
        <div class="ov-label">Build cache</div>
        <div class="ov-value">{{ fmtBytes(overview.build_cache_size_bytes) }}</div>
        <div class="ov-sub reclaimable">Récupérable : {{ fmtBytes(overview.reclaimable_build_cache_bytes) }}</div>
      </div>
      <div class="ov-card highlight">
        <div class="ov-label">Total récupérable</div>
        <div class="ov-value">{{ fmtBytes(overview.reclaimable_images_bytes + overview.reclaimable_containers_bytes + overview.reclaimable_volumes_bytes + overview.reclaimable_build_cache_bytes) }}</div>
        <div class="ov-sub">Lance un nettoyage pour libérer cet espace</div>
      </div>
    </div>

    <!-- ── Containers ── -->
    <div v-else-if="tab === 'containers'">
      <div class="filters">
        <select v-model="filterContainerState">
          <option value="all">Tous ({{ containers.length }})</option>
          <option value="running">Running ({{ containers.filter(c => c.state === 'running').length }})</option>
          <option value="stopped">Arrêtés ({{ containers.filter(c => c.state !== 'running').length }})</option>
        </select>
      </div>
      <table class="docker-table">
        <thead>
          <tr><th>Nom</th><th>Image</th><th>État</th><th>Statut</th><th>Ports</th><th>Taille</th><th class="actions-h">Actions</th></tr>
        </thead>
        <tbody>
          <tr v-for="c in filteredContainers" :key="c.id">
            <td><code>{{ cleanName(c.names[0] ?? shortId(c.id)) }}</code></td>
            <td class="muted">{{ c.image }}</td>
            <td><span class="state-pill" :class="c.state">{{ c.state }}</span></td>
            <td class="muted small">{{ c.status }}</td>
            <td class="ports small">{{ c.ports.join(', ') || '—' }}</td>
            <td class="small">{{ fmtBytes(c.size_rw_bytes ?? 0) }}</td>
            <td class="actions">
              <button v-if="visible('docker.action.start')" class="btn xs" :disabled="busy || c.state === 'running'" title="Démarrer" @click="startCt(c)">▶</button>
              <button v-if="visible('docker.action.stop')" class="btn xs" :disabled="busy || c.state !== 'running'" title="Arrêter" @click="stopCt(c)">⏹</button>
              <button v-if="visible('docker.action.restart')" class="btn xs" :disabled="busy" title="Redémarrer" @click="restartCt(c)">↻</button>
              <button v-if="visible('docker.action.logs')" class="btn xs" :disabled="busy" title="Logs" @click="openLogs(c)">📋</button>
              <button v-if="visible('docker.action.remove_container')" class="btn xs danger" :disabled="busy" title="Supprimer" @click="removeCt(c)">🗑</button>
            </td>
          </tr>
        </tbody>
      </table>
    </div>

    <!-- ── Images ── -->
    <div v-else-if="tab === 'images'">
      <div class="filters">
        <label><input type="checkbox" v-model="showOnlyDangling" /> Uniquement non utilisées / dangling</label>
        <span class="muted">{{ filteredImages.length }} image(s)</span>
      </div>
      <table class="docker-table">
        <thead>
          <tr><th>Tag</th><th>ID</th><th>Créée</th><th>Taille</th><th>Conteneurs</th><th class="actions-h">Actions</th></tr>
        </thead>
        <tbody>
          <tr v-for="img in filteredImages" :key="img.id" :class="{ dangling: img.dangling }">
            <td>
              <code v-if="img.repo_tags.length > 0">{{ img.repo_tags[0] }}</code>
              <span v-else class="muted">&lt;none&gt;</span>
              <span v-if="img.dangling" class="badge dangling-badge">dangling</span>
            </td>
            <td class="small mono">{{ shortId(img.id) }}</td>
            <td class="small muted">{{ fmtTs(img.created) }}</td>
            <td class="small">{{ fmtBytes(img.size_bytes) }}</td>
            <td class="small">{{ img.containers > 0 ? img.containers : '—' }}</td>
            <td class="actions">
              <button v-if="visible('docker.action.remove_image')" class="btn xs danger" :disabled="busy" title="Supprimer" @click="removeImg(img)">🗑</button>
            </td>
          </tr>
        </tbody>
      </table>
    </div>

    <!-- ── Volumes ── -->
    <div v-else-if="tab === 'volumes'">
      <div class="filters">
        <label><input type="checkbox" v-model="showOnlyUnused" /> Uniquement orphelins</label>
        <span class="muted">{{ filteredVolumes.length }} volume(s)</span>
      </div>
      <table class="docker-table">
        <thead>
          <tr><th>Nom</th><th>Driver</th><th>Mountpoint</th><th>Taille</th><th>Réf</th><th class="actions-h">Actions</th></tr>
        </thead>
        <tbody>
          <tr v-for="v in filteredVolumes" :key="v.name" :class="{ orphan: !v.in_use }">
            <td><code>{{ v.name }}</code><span v-if="!v.in_use" class="badge orphan-badge">orphelin</span></td>
            <td class="small">{{ v.driver }}</td>
            <td class="small mono muted">{{ v.mountpoint }}</td>
            <td class="small">{{ fmtBytes(v.size_bytes) }}</td>
            <td class="small">{{ v.ref_count ?? '—' }}</td>
            <td class="actions">
              <button v-if="visible('docker.action.remove_volume')" class="btn xs danger" :disabled="busy" title="Supprimer" @click="removeVol(v)">🗑</button>
            </td>
          </tr>
        </tbody>
      </table>
    </div>

    <!-- ── Networks ── -->
    <div v-else-if="tab === 'networks'">
      <table class="docker-table">
        <thead>
          <tr><th>Nom</th><th>Driver</th><th>Scope</th><th>Conteneurs</th><th>Interne</th></tr>
        </thead>
        <tbody>
          <tr v-for="n in networks" :key="n.id">
            <td><code>{{ n.name }}</code></td>
            <td class="small">{{ n.driver }}</td>
            <td class="small">{{ n.scope }}</td>
            <td class="small">{{ n.containers_count }}</td>
            <td class="small">{{ n.internal ? 'oui' : 'non' }}</td>
          </tr>
        </tbody>
      </table>
    </div>

    <!-- ── Prune ── -->
    <div v-else-if="tab === 'prune'" class="prune-grid">
      <div v-if="visible('docker.prune.containers')" class="prune-card">
        <h4>📦 Conteneurs arrêtés</h4>
        <p class="muted">Supprime tous les conteneurs en état non running.</p>
        <p v-if="overview" class="reclaim">Récupérable : {{ fmtBytes(overview.reclaimable_containers_bytes) }}</p>
        <button class="btn" :disabled="busy" @click="pruneContainers">Nettoyer</button>
      </div>
      <div v-if="visible('docker.prune.images')" class="prune-card">
        <h4>🖼 Images dangling</h4>
        <p class="muted">Images sans tag, jamais utilisées.</p>
        <p v-if="overview" class="reclaim">Récupérable : {{ fmtBytes(overview.reclaimable_images_bytes) }}</p>
        <button class="btn" :disabled="busy" @click="pruneImages(false)">Nettoyer dangling</button>
        <button class="btn warning" :disabled="busy" @click="pruneImages(true)">Toutes inutilisées</button>
      </div>
      <div v-if="visible('docker.prune.volumes')" class="prune-card">
        <h4>💾 Volumes orphelins</h4>
        <p class="muted">⚠️ Volumes sans conteneur lié. Risque de perte de données.</p>
        <p v-if="overview" class="reclaim">Récupérable : {{ fmtBytes(overview.reclaimable_volumes_bytes) }}</p>
        <button class="btn danger" :disabled="busy" @click="pruneVolumes">Nettoyer</button>
      </div>
      <div v-if="visible('docker.prune.networks')" class="prune-card">
        <h4>🌐 Réseaux inutilisés</h4>
        <p class="muted">Réseaux sans conteneur attaché.</p>
        <button class="btn" :disabled="busy" @click="pruneNetworks">Nettoyer</button>
      </div>
      <div class="prune-card">
        <h4>🧱 Build cache</h4>
        <p class="muted">Cache de couches buildées non utilisées.</p>
        <p v-if="overview" class="reclaim">Récupérable : {{ fmtBytes(overview.reclaimable_build_cache_bytes) }}</p>
        <p class="muted small">Inclus dans "Nettoyage complet ↓".</p>
      </div>
      <div v-if="visible('docker.prune.system')" class="prune-card highlight">
        <h4>🚀 Nettoyage complet</h4>
        <p class="muted">conteneurs + images dangling + réseaux.</p>
        <button class="btn" :disabled="busy" @click="pruneSystem(false, false)">Nettoyage standard</button>
        <button class="btn warning" :disabled="busy" @click="pruneSystem(false, true)">+ toutes images inutilisées</button>
        <button class="btn danger" :disabled="busy" @click="pruneSystem(true, true)">+ volumes ⚠️</button>
      </div>
    </div>

    <!-- ── Logs modal ── -->
    <div v-if="logsOpen" class="logs-modal" @click.self="closeLogs">
      <div class="logs-window">
        <div class="logs-head">
          <strong>📋 Logs : {{ logsContainer ? cleanName(logsContainer.names[0] ?? '') : '' }}</strong>
          <div class="logs-controls">
            <label>Lignes :
              <select v-model.number="logsTail" @change="fetchLogs">
                <option :value="50">50</option>
                <option :value="200">200</option>
                <option :value="500">500</option>
                <option :value="2000">2000</option>
                <option :value="5000">5000</option>
              </select>
            </label>
            <button class="btn xs" :disabled="logsLoading" @click="fetchLogs">↻</button>
            <button class="btn xs" @click="closeLogs">Fermer</button>
          </div>
        </div>
        <pre v-if="!logsLoading" class="logs-body">{{ logsContent || '(vide)' }}</pre>
        <div v-else class="muted center">Chargement…</div>
      </div>
    </div>
  </section>
</template>

<style scoped>
.docker-section {
  margin-bottom: 24px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 12px;
  padding: 18px 20px;
}
.docker-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  flex-wrap: wrap;
  gap: 14px;
  margin-bottom: 16px;
}
.section-title {
  margin: 0;
  font-size: 16px;
  font-weight: 700;
}
.tabs { display: flex; gap: 6px; flex-wrap: wrap; }
.tabs button {
  padding: 6px 12px;
  border-radius: 8px;
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  color: var(--text-secondary);
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.18s ease;
}
.tabs button:hover { color: var(--text-primary); border-color: var(--accent); }
.tabs button.active {
  background: color-mix(in srgb, var(--accent) 18%, var(--bg-secondary));
  color: var(--accent);
  border-color: var(--accent);
}

/* Overview cards */
.overview-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
  gap: 12px;
}
.ov-card {
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: 10px;
  padding: 12px 14px;
}
.ov-card.highlight {
  background: color-mix(in srgb, var(--accent) 10%, var(--bg-secondary));
  border-color: var(--accent);
}
.ov-label { font-size: 11px; font-weight: 700; color: var(--text-secondary); text-transform: uppercase; letter-spacing: 0.4px; }
.ov-value { font-size: 22px; font-weight: 700; margin-top: 4px; }
.ov-sub { font-size: 11px; color: var(--text-secondary); margin-top: 4px; }
.ov-sub.reclaimable { color: var(--warning, #e67e22); font-weight: 600; }

/* Tables */
.filters {
  display: flex;
  align-items: center;
  gap: 14px;
  margin-bottom: 10px;
  font-size: 12px;
}
.filters select, .filters input { font-size: 12px; }
.docker-table {
  width: 100%;
  border-collapse: collapse;
  font-size: 12px;
}
.docker-table th, .docker-table td {
  text-align: left;
  padding: 8px 10px;
  border-bottom: 1px solid color-mix(in srgb, var(--border) 60%, transparent);
}
.docker-table th {
  font-size: 11px;
  text-transform: uppercase;
  color: var(--text-secondary);
  letter-spacing: 0.4px;
}
.docker-table td.small { font-size: 11px; }
.docker-table td.mono, .docker-table .ports { font-family: "JetBrains Mono", monospace; }
.docker-table tr.dangling, .docker-table tr.orphan {
  background: color-mix(in srgb, var(--warning, #e67e22) 6%, transparent);
}
.actions-h { text-align: right; }
.actions { text-align: right; white-space: nowrap; }
.actions .btn { margin-left: 4px; }

.state-pill {
  display: inline-block;
  padding: 2px 8px;
  border-radius: 12px;
  font-size: 10px;
  text-transform: uppercase;
  font-weight: 700;
  letter-spacing: 0.4px;
  background: var(--bg-secondary);
  color: var(--text-secondary);
}
.state-pill.running { background: color-mix(in srgb, var(--success, #2ecc71) 20%, var(--bg-secondary)); color: var(--success, #2ecc71); }
.state-pill.exited, .state-pill.dead { background: color-mix(in srgb, var(--danger) 18%, var(--bg-secondary)); color: var(--danger); }
.state-pill.paused, .state-pill.restarting { background: color-mix(in srgb, var(--warning, #e67e22) 20%, var(--bg-secondary)); color: var(--warning, #e67e22); }

.badge {
  display: inline-block;
  margin-left: 6px;
  padding: 1px 6px;
  border-radius: 4px;
  font-size: 9px;
  font-weight: 700;
  text-transform: uppercase;
}
.dangling-badge, .orphan-badge {
  background: var(--warning, #e67e22);
  color: white;
}

/* Buttons */
.btn {
  padding: 6px 12px;
  border-radius: 8px;
  border: 1px solid var(--border);
  background: var(--bg-secondary);
  color: var(--text-primary);
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.15s ease;
}
.btn:hover:not(:disabled) { border-color: var(--accent); color: var(--accent); }
.btn:disabled { opacity: 0.45; cursor: not-allowed; }
.btn.xs { padding: 3px 8px; font-size: 11px; }
.btn.danger { border-color: color-mix(in srgb, var(--danger) 50%, var(--border)); color: var(--danger); }
.btn.danger:hover:not(:disabled) { background: color-mix(in srgb, var(--danger) 15%, var(--bg-secondary)); }
.btn.warning { border-color: color-mix(in srgb, var(--warning, #e67e22) 50%, var(--border)); color: var(--warning, #e67e22); }
.btn.warning:hover:not(:disabled) { background: color-mix(in srgb, var(--warning, #e67e22) 15%, var(--bg-secondary)); }

/* Prune grid */
.prune-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(260px, 1fr));
  gap: 14px;
}
.prune-card {
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: 10px;
  padding: 14px 16px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.prune-card.highlight {
  background: color-mix(in srgb, var(--accent) 10%, var(--bg-secondary));
  border-color: var(--accent);
}
.prune-card h4 { margin: 0; font-size: 14px; }
.prune-card p { margin: 0; font-size: 12px; }
.prune-card .reclaim { color: var(--warning, #e67e22); font-weight: 600; font-size: 12px; }

/* Logs modal */
.logs-modal {
  position: fixed;
  inset: 0;
  background: rgba(0,0,0,0.7);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
  padding: 30px;
}
.logs-window {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 12px;
  width: min(1100px, 95vw);
  max-height: 85vh;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}
.logs-head {
  display: flex;
  justify-content: space-between;
  align-items: center;
  flex-wrap: wrap;
  gap: 10px;
  padding: 12px 16px;
  border-bottom: 1px solid var(--border);
  background: var(--bg-secondary);
}
.logs-controls { display: flex; gap: 8px; align-items: center; font-size: 12px; }
.logs-body {
  margin: 0;
  padding: 14px 16px;
  overflow: auto;
  flex: 1;
  font-family: "JetBrains Mono", monospace;
  font-size: 11px;
  line-height: 1.45;
  white-space: pre-wrap;
  word-break: break-all;
  background: #0e1116;
  color: #d4d4d8;
}
.center { padding: 30px; text-align: center; }
.muted { color: var(--text-secondary); font-size: 12px; }
.small { font-size: 11px; }
</style>
