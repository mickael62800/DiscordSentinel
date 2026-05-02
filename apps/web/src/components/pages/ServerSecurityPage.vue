<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import {
  serverSecurityService,
  type AuthFailureEntry,
  type BannedIpsResponse,
  type SecurityWindow,
  type ConnectionsResponse,
  type DiskTrendResponse,
  type FileIntegrityResponse,
  type OpenPortsResponse,
  type OutboundResponse,
  type ServerEventDto,
  type SshFailuresResponse,
  type SuccessfulLoginEntry,
  type SuspiciousResponse,
  type TlsCertInfo,
  type TlsErrorsResponse,
  type TopIpEntry,
  type TrivyResponse,
  type GeoIpEntry,
  type ContainerChangesResponse,
} from "@/services/serverSecurityService";
import { useToast } from "@/composables/useToast";
import { useMyRole } from "@/composables/useMyRole";
import TrafficTrendChart from "@/components/organisms/TrafficTrendChart.vue";

const { error: showError } = useToast();
const { role, isSuper } = useMyRole();
const canManage = computed(() => isSuper.value || role.value === "owner");

type TabKey =
  | "overview"
  | "attacks"
  | "bans"
  | "network"
  | "integrity"
  | "audit"
  | "alerts";

const currentTab = ref<TabKey>("overview");
const refreshing = ref(false);
const cleaning = ref(false);
const cleanupOpts = ref({ days: 0, includeAudit: true });
const showCleanupModal = ref(false);

// Sections data
const topIps = ref<TopIpEntry[]>([]);
const topIpsWindow = ref<SecurityWindow>("1h");
const authFailures = ref<AuthFailureEntry[]>([]);
const authWindow = ref<SecurityWindow>("24h");
const banned = ref<BannedIpsResponse | null>(null);
const serverEvents = ref<ServerEventDto[]>([]);
const eventsFilter = ref<"all" | "docker" | "security" | "rbac">("all");
const tls = ref<TlsCertInfo | null>(null);
const tlsError = ref<string | null>(null);
const lastLogins = ref<SuccessfulLoginEntry[]>([]);
const sshFailures = ref<SshFailuresResponse | null>(null);
const sshError = ref<string | null>(null);
const diskTrend = ref<DiskTrendResponse | null>(null);
const diskError = ref<string | null>(null);
const connections = ref<ConnectionsResponse | null>(null);
const connectionsError = ref<string | null>(null);
const openPorts = ref<OpenPortsResponse | null>(null);
const portsError = ref<string | null>(null);
const trivy = ref<TrivyResponse | null>(null);
const trivyError = ref<string | null>(null);
const integrity = ref<FileIntegrityResponse | null>(null);
const integrityError = ref<string | null>(null);
const outbound = ref<OutboundResponse | null>(null);
const outboundError = ref<string | null>(null);
const suspicious = ref<SuspiciousResponse | null>(null);
const suspiciousError = ref<string | null>(null);
const tlsErrors = ref<TlsErrorsResponse | null>(null);
const tlsErrorsError = ref<string | null>(null);
const geoMap = ref<Record<string, GeoIpEntry>>({});
const containers = ref<ContainerChangesResponse | null>(null);
const containersError = ref<string | null>(null);

// ── Loaders ──
async function loadTopIps() {
  try { topIps.value = await serverSecurityService.topIps(topIpsWindow.value, 20); }
  catch (e: any) { showError(`Top IPs : ${e?.message ?? e}`); }
}
async function loadAuthFailures() {
  try { authFailures.value = await serverSecurityService.authFailures(authWindow.value, 100); }
  catch (e: any) { showError(`Echecs auth : ${e?.message ?? e}`); }
}
async function loadBanned() {
  try { banned.value = await serverSecurityService.bannedIps(); }
  catch (e: any) { showError(`Bans : ${e?.message ?? e}`); }
}
async function loadServerEvents() {
  try {
    const prefix = eventsFilter.value === "all" ? undefined : eventsFilter.value;
    serverEvents.value = await serverSecurityService.serverEvents({ action_prefix: prefix, limit: 100 });
  } catch (e: any) { showError(`Events serveur : ${e?.message ?? e}`); }
}
async function loadLastLogins() {
  try { lastLogins.value = await serverSecurityService.lastLogins(20); }
  catch (e: any) { showError(`Logins : ${e?.message ?? e}`); }
}
async function loadSshFailures() {
  sshError.value = null;
  try { sshFailures.value = await serverSecurityService.sshFailures(); }
  catch (e: any) { sshError.value = e?.message ?? String(e); sshFailures.value = null; }
}
async function loadDiskTrend() {
  diskError.value = null;
  try { diskTrend.value = await serverSecurityService.diskTrend(); }
  catch (e: any) { diskError.value = e?.message ?? String(e); diskTrend.value = null; }
}
async function loadConnections() {
  connectionsError.value = null;
  try { connections.value = await serverSecurityService.connections(); }
  catch (e: any) { connectionsError.value = e?.message ?? String(e); connections.value = null; }
}
async function loadOpenPorts() {
  portsError.value = null;
  try { openPorts.value = await serverSecurityService.openPorts(); }
  catch (e: any) { portsError.value = e?.message ?? String(e); openPorts.value = null; }
}
async function loadTrivy() {
  trivyError.value = null;
  try { trivy.value = await serverSecurityService.trivy(); }
  catch (e: any) { trivyError.value = e?.message ?? String(e); trivy.value = null; }
}
async function loadIntegrity() {
  integrityError.value = null;
  try { integrity.value = await serverSecurityService.fileIntegrity(); }
  catch (e: any) { integrityError.value = e?.message ?? String(e); integrity.value = null; }
}
async function loadOutbound() {
  outboundError.value = null;
  try { outbound.value = await serverSecurityService.outbound(); }
  catch (e: any) { outboundError.value = e?.message ?? String(e); outbound.value = null; }
}
async function loadContainers() {
  containersError.value = null;
  try { containers.value = await serverSecurityService.containerChanges(); }
  catch (e: any) { containersError.value = e?.message ?? String(e); containers.value = null; }
}
async function loadSuspicious() {
  suspiciousError.value = null;
  try { suspicious.value = await serverSecurityService.nginxSuspicious(); }
  catch (e: any) { suspiciousError.value = e?.message ?? String(e); suspicious.value = null; }
}
async function loadTlsErrors() {
  tlsErrorsError.value = null;
  try { tlsErrors.value = await serverSecurityService.tlsErrors(); }
  catch (e: any) { tlsErrorsError.value = e?.message ?? String(e); tlsErrors.value = null; }
}
async function loadTls() {
  tlsError.value = null;
  try { tls.value = await serverSecurityService.tlsCert(); }
  catch (e: any) { tlsError.value = e?.message ?? String(e); tls.value = null; }
}

async function banIp(ip: string) {
  if (!confirm(`Bannir l'IP ${ip} ? Elle ne pourra plus accéder au serveur.`)) return;
  try {
    const r = await serverSecurityService.banIp(ip, "ban manuel via panel sécurité");
    alert(`✅ ${r.message}`);
    await loadBanned();
    await loadServerEvents();
  } catch (e: any) { showError(`Echec ban : ${e?.message ?? e}`); }
}

async function unbanIp(ip: string) {
  if (!confirm(`Débannir l'IP ${ip} ?`)) return;
  try {
    const r = await serverSecurityService.unbanIp(ip, "unban manuel via panel sécurité");
    alert(`✅ ${r.message}`);
    await loadBanned();
    await loadServerEvents();
  } catch (e: any) { showError(`Echec unban : ${e?.message ?? e}`); }
}

async function refreshAll() {
  refreshing.value = true;
  await Promise.allSettled([
    loadTopIps(), loadAuthFailures(), loadBanned(), loadServerEvents(), loadTls(),
    loadLastLogins(), loadSshFailures(), loadDiskTrend(), loadConnections(),
    loadOpenPorts(), loadTrivy(), loadIntegrity(), loadOutbound(),
    loadSuspicious(), loadTlsErrors(), loadContainers(),
  ]);
  void loadGeoForAll();
  refreshing.value = false;
}

async function runCleanup() {
  cleaning.value = true;
  try {
    const r = await serverSecurityService.cleanup({
      older_than_days: cleanupOpts.value.days,
      include_audit_logs: cleanupOpts.value.includeAudit,
    });
    showCleanupModal.value = false;
    await refreshAll();
    alert(
      `✅ Nettoyage terminé\n\n` +
      `• Logs API supprimés : ${r.deleted_api_logs}\n` +
      `• Audit logs Discord supprimés : ${r.deleted_audit_logs}\n\n` +
      `${r.message}`,
    );
  } catch (e: any) { showError(`Echec cleanup : ${e?.message ?? e}`); }
  finally { cleaning.value = false; }
}

onMounted(refreshAll);

// ── Helpers ──
function fmtDate(s: string): string { return new Date(s).toLocaleString("fr-FR"); }
function truncate(s: string, n: number): string { return s.length > n ? s.slice(0, n) + "…" : s; }

// ── Computed pour vue d'ensemble ──
const totalFailedRequests = computed(() =>
  topIps.value.reduce((sum, ip) => sum + ip.failed, 0),
);
const totalBannedIps = computed(() =>
  banned.value?.jails.reduce((sum, j) => sum + j.banned_ips.length, 0) ?? 0,
);
// Agrege IPs uniques (Top IPs + Auth failures) avec leur geoloc
interface GeoRow {
  ip: string;
  total: number;
  failed: number;
  country?: string;
  countryCode?: string;
  city?: string;
  isp?: string;
  asn?: string;
}
const geoRows = computed<GeoRow[]>(() => {
  const map = new Map<string, GeoRow>();
  for (const t of topIps.value) {
    map.set(t.client_ip, { ip: t.client_ip, total: t.total, failed: t.failed });
  }
  for (const a of authFailures.value) {
    if (!a.client_ip || a.client_ip === "-") continue;
    const ex = map.get(a.client_ip) ?? { ip: a.client_ip, total: 0, failed: 0 };
    ex.failed = (ex.failed || 0) + 1;
    map.set(a.client_ip, ex);
  }
  for (const r of map.values()) {
    const g = geoMap.value[r.ip];
    if (g) {
      r.country = g.country;
      r.countryCode = g.countryCode;
      r.city = g.city;
      r.isp = g.isp;
      r.asn = g.asn;
    }
  }
  return Array.from(map.values()).sort((a, b) => b.total + b.failed - (a.total + a.failed));
});

async function loadGeoForAll() {
  const ips = Array.from(new Set([
    ...topIps.value.map((i) => i.client_ip),
    ...authFailures.value.map((a) => a.client_ip).filter((x) => x && x !== "-"),
  ])).slice(0, 100);
  if (ips.length === 0) return;
  try {
    const entries = await serverSecurityService.geoip(ips);
    const m: Record<string, GeoIpEntry> = { ...geoMap.value };
    for (const e of entries) m[e.query] = e;
    geoMap.value = m;
  } catch { /* best-effort */ }
}

const tlsBadgeClass = computed(() => {
  if (!tls.value) return "";
  if (tls.value.is_expired) return "danger";
  if (tls.value.is_warning) return "warning";
  return "ok";
});

const tabs: Array<{ key: TabKey; label: string; icon: string }> = [
  { key: "overview", label: "Vue d'ensemble", icon: "📊" },
  { key: "attacks", label: "Attaques", icon: "🔥" },
  { key: "bans", label: "Bans & Protections", icon: "🚫" },
  { key: "network", label: "Réseau", icon: "🌐" },
  { key: "integrity", label: "Intégrité", icon: "🛡️" },
  { key: "audit", label: "Audit", icon: "📋" },
  { key: "alerts", label: "Alertes", icon: "🔔" },
];
</script>

<template>
  <div class="security-page">
    <div class="page-header">
      <div>
        <h1>🛡️ Sécurité serveur</h1>
        <p class="muted small">Surveillance des attaques, intégrité et protections actives.</p>
      </div>
      <div class="header-actions">
        <button v-if="canManage" class="btn danger" :disabled="cleaning" @click="showCleanupModal = true">🗑 Tout nettoyer</button>
        <button class="btn primary" :disabled="refreshing" @click="refreshAll">
          {{ refreshing ? "Actualisation…" : "↻ Actualiser tout" }}
        </button>
      </div>
    </div>

    <!-- ── Onglets ── -->
    <div class="tabs">
      <button
        v-for="tab in tabs"
        :key="tab.key"
        :class="['tab', { active: currentTab === tab.key }]"
        @click="currentTab = tab.key"
      >
        <span class="tab-icon">{{ tab.icon }}</span> {{ tab.label }}
      </button>
    </div>

    <!-- ════════ ONGLET 1 : VUE D'ENSEMBLE ════════ -->
    <div v-if="currentTab === 'overview'" class="tab-content">
      <section class="kpis">
        <div class="kpi-card">
          <span class="kpi-label">Erreurs HTTP (1h)</span>
          <span class="kpi-value">{{ totalFailedRequests }}</span>
          <span class="kpi-hint">4xx + 5xx sur Top IPs</span>
        </div>
        <div class="kpi-card">
          <span class="kpi-label">Échecs auth (24h)</span>
          <span class="kpi-value">{{ authFailures.length }}</span>
          <span class="kpi-hint">401 / 403 récents</span>
        </div>
        <div class="kpi-card">
          <span class="kpi-label">IPs bannies</span>
          <span class="kpi-value">{{ totalBannedIps }}</span>
          <span class="kpi-hint">via fail2ban / ufw</span>
        </div>
        <div class="kpi-card">
          <span class="kpi-label">Cert TLS</span>
          <span class="kpi-value" :class="tlsBadgeClass">
            <template v-if="tls">{{ tls.days_until_expiry }}j</template>
            <template v-else>—</template>
          </span>
          <span class="kpi-hint">jours restants</span>
        </div>
      </section>

      <div class="overview-hint">
        💡 Les onglets ci-dessus regroupent par thème : <strong>Attaques</strong> (qui frappe à la
        porte), <strong>Bans</strong> (qui est bloqué), <strong>Réseau</strong> (connexions et
        ports), <strong>Intégrité</strong> (cert TLS, fichiers, vulns), <strong>Audit</strong>
        (qui a fait quoi), <strong>Alertes</strong> (config notifications).
      </div>
    </div>

    <!-- ════════ ONGLET 2 : ATTAQUES ════════ -->
    <div v-if="currentTab === 'attacks'" class="tab-content">
      <!-- Top IPs -->
      <section class="card">
        <div class="card-head">
          <h2>📊 Top IPs par requêtes</h2>
          <div class="card-actions">
            <select v-model="topIpsWindow" @change="loadTopIps">
              <option value="1h">1h</option>
              <option value="24h">24h</option>
              <option value="7d">7j</option>
            </select>
            <button class="btn xs" @click="loadTopIps">↻</button>
          </div>
        </div>
        <table v-if="topIps.length > 0" class="data-table">
          <thead>
            <tr><th>IP</th><th>Pays</th><th>FAI</th><th class="num">Total</th><th class="num">4xx/5xx</th><th>Dernier</th><th class="actions-h">Action</th></tr>
          </thead>
          <tbody>
            <tr v-for="ip in topIps" :key="ip.client_ip" :class="{ alert: ip.failed > 10 }">
              <td><code>{{ ip.client_ip }}</code></td>
              <td class="small">
                <span v-if="geoMap[ip.client_ip]?.countryCode">
                  {{ geoMap[ip.client_ip].countryCode }} · {{ geoMap[ip.client_ip].country }}
                </span>
                <span v-else class="muted">—</span>
              </td>
              <td class="small muted">{{ truncate(geoMap[ip.client_ip]?.isp ?? "—", 30) }}</td>
              <td class="num">{{ ip.total }}</td>
              <td class="num" :class="{ danger: ip.failed > 10 }">{{ ip.failed }}</td>
              <td class="muted small">{{ fmtDate(ip.last_seen) }}</td>
              <td class="actions">
                <button v-if="canManage" class="btn xs danger" @click="banIp(ip.client_ip)" title="Bannir cette IP">🚫 Ban</button>
              </td>
            </tr>
          </tbody>
        </table>
        <div v-else class="empty">Aucune requête sur la fenêtre sélectionnée.</div>
      </section>

      <!-- Échecs d'auth -->
      <section class="card">
        <div class="card-head">
          <h2>🔒 Échecs d'authentification ({{ authFailures.length }})</h2>
          <div class="card-actions">
            <select v-model="authWindow" @change="loadAuthFailures">
              <option value="1h">1h</option>
              <option value="24h">24h</option>
              <option value="7d">7j</option>
            </select>
            <button class="btn xs" @click="loadAuthFailures">↻</button>
          </div>
        </div>
        <table v-if="authFailures.length > 0" class="data-table">
          <thead>
            <tr>
              <th>Quand</th><th>Code</th><th>Méthode</th><th>Route</th><th>IP</th>
              <th>User-Agent</th><th class="actions-h">Action</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="(e, i) in authFailures" :key="i">
              <td class="small muted">{{ fmtDate(e.timestamp) }}</td>
              <td>
                <span class="code-pill" :class="{ danger: e.status_code >= 500, warn: e.status_code === 401 || e.status_code === 403 }">
                  {{ e.status_code }}
                </span>
              </td>
              <td class="small mono">{{ e.method }}</td>
              <td class="small mono">{{ truncate(e.route, 60) }}</td>
              <td class="small mono">{{ e.client_ip }}</td>
              <td class="small muted ua">{{ truncate(e.user_agent, 80) }}</td>
              <td class="actions">
                <button v-if="canManage && e.client_ip && e.client_ip !== '-'" class="btn xs danger" @click="banIp(e.client_ip)" title="Bannir">🚫</button>
              </td>
            </tr>
          </tbody>
        </table>
        <div v-else class="empty">Aucun échec d'authentification 🎉</div>
      </section>

      <!-- Placeholder : SSH failures + Patterns nginx + Trafic anormal -->
      <!-- SSH failures (host fichier-shim) -->
      <section class="card">
        <div class="card-head">
          <h2>🔑 Tentatives SSH échouées</h2>
          <button class="btn xs" @click="loadSshFailures">↻</button>
        </div>
        <div v-if="sshError" class="info">
          <p class="small">{{ sshError }}</p>
          <p class="hint small">Setup : <code>sudo bash infra/scripts/setup-host-security.sh ssh-failures</code></p>
        </div>
        <div v-else-if="sshFailures">
          <p class="muted small">
            <strong>{{ sshFailures.total_24h }}</strong> tentatives sur 24h · Maj {{ fmtDate(sshFailures.updated_at) }}
          </p>
          <table v-if="sshFailures.entries.length > 0" class="data-table">
            <thead>
              <tr><th>Quand</th><th>Utilisateur</th><th>IP</th><th>Action</th></tr>
            </thead>
            <tbody>
              <tr v-for="(e, i) in sshFailures.entries.slice(0, 30)" :key="i">
                <td class="small muted">{{ e.timestamp }}</td>
                <td class="small mono">{{ e.user }}</td>
                <td class="small mono">{{ e.ip }}</td>
                <td class="actions">
                  <button v-if="canManage && e.ip !== '?'" class="btn xs danger" @click="banIp(e.ip)">🚫 Ban</button>
                </td>
              </tr>
            </tbody>
          </table>
          <div v-else class="empty">Aucune tentative SSH ratée 🎉</div>
        </div>
      </section>

      <!-- Patterns suspects nginx -->
      <section class="card">
        <div class="card-head">
          <h2>🕷 Patterns suspects nginx (24h)</h2>
          <button class="btn xs" @click="loadSuspicious">↻</button>
        </div>
        <div v-if="suspiciousError" class="info">
          <p class="small">{{ suspiciousError }}</p>
          <p class="hint small">Setup : <code>sudo bash infra/scripts/setup-host-security.sh nginx-suspicious</code></p>
        </div>
        <div v-else-if="suspicious">
          <p class="muted small">Maj {{ fmtDate(suspicious.updated_at) }} · {{ suspicious.total_24h }} requête(s) suspecte(s)</p>
          <div class="cve-summary">
            <div class="cve-stat danger"><span class="lbl">SQLi</span><strong>{{ suspicious.by_category?.sqli ?? 0 }}</strong></div>
            <div class="cve-stat warning"><span class="lbl">XSS</span><strong>{{ suspicious.by_category?.xss ?? 0 }}</strong></div>
            <div class="cve-stat warning"><span class="lbl">Traversal</span><strong>{{ suspicious.by_category?.traversal ?? 0 }}</strong></div>
            <div class="cve-stat"><span class="lbl">Scanners</span><strong>{{ suspicious.by_category?.scanner ?? 0 }}</strong></div>
          </div>
          <table v-if="suspicious.entries.length > 0" class="data-table">
            <thead><tr><th>IP</th><th>Méth.</th><th>URL</th><th>Status</th><th>Catégorie</th><th>UA</th><th></th></tr></thead>
            <tbody>
              <tr v-for="(e, i) in suspicious.entries.slice(0, 50)" :key="i">
                <td class="small mono">{{ e.ip }}</td>
                <td class="small">{{ e.method }}</td>
                <td class="small mono">{{ truncate(e.url, 60) }}</td>
                <td class="small">{{ e.status }}</td>
                <td><span class="badge" :class="e.category === 'sqli' ? 'danger' : 'warning'">{{ e.category }}</span></td>
                <td class="small">{{ truncate(e.user_agent, 30) }}</td>
                <td><button v-if="canManage" class="btn xs danger" @click="banIp(e.ip)">🚫</button></td>
              </tr>
            </tbody>
          </table>
          <div v-else class="empty">Aucun pattern suspect détecté 🎉</div>
        </div>
      </section>
      <!-- Trafic anormal (vrai composant) -->
      <TrafficTrendChart />
    </div>

    <!-- ════════ ONGLET 3 : BANS & PROTECTIONS ════════ -->
    <div v-if="currentTab === 'bans'" class="tab-content">
      <!-- IPs bannies -->
      <section class="card">
        <h2>🚫 IPs bannies (fail2ban)</h2>
        <div v-if="banned && !banned.installed" class="info">
          <p>{{ banned.message }}</p>
          <p class="hint small">
            Setup rapide : <code>sudo bash infra/scripts/setup-host-security.sh fail2ban</code>
          </p>
        </div>
        <div v-else-if="banned && banned.jails.length > 0">
          <p class="muted small">
            Maj : {{ fmtDate(banned.updated_at!) }} · {{ banned.message }}
          </p>
          <div v-for="jail in banned.jails" :key="jail.name" class="jail-card">
            <div class="jail-head">
              <strong>🔒 {{ jail.name }}</strong>
              <span class="badge" :class="jail.banned_ips.length > 0 ? 'danger' : 'ok'">
                {{ jail.banned_ips.length }} IP{{ jail.banned_ips.length > 1 ? 's' : '' }} bannie{{ jail.banned_ips.length > 1 ? 's' : '' }}
                · total cumul {{ jail.total_banned }}
              </span>
            </div>
            <ul v-if="jail.banned_ips.length > 0" class="ip-list">
              <li v-for="ip in jail.banned_ips" :key="ip" class="ip-pill">
                <code>{{ ip }}</code>
                <button v-if="canManage" class="btn xs" @click="unbanIp(ip)" title="Débannir cette IP">↻ Débannir</button>
              </li>
            </ul>
            <div v-else class="muted small">Aucune IP actuellement bannie sur cette jail.</div>
          </div>
        </div>
        <div v-else class="empty">fail2ban actif mais aucune jail configurée.</div>
      </section>

      <!-- Rate limit dynamique (info) -->
      <section class="card">
        <h2>⚡ Rate limit dynamique</h2>
        <p class="muted small">
          Actif côté API : chaque IP est trackée en mémoire. Au-delà du seuil
          configuré, un ban est demandé automatiquement au shim
          <code>ban-apply</code> qui passe par <code>fail2ban-client set sentinel-api banip</code>.
        </p>
        <ul class="muted small">
          <li>Variables : <code>RATE_LIMIT_THRESHOLD</code> (défaut 200 req), <code>RATE_LIMIT_WINDOW_SECS</code> (défaut 60s).</li>
          <li>Cooldown : 5 min sans nouveau ban pour la même IP.</li>
          <li>Vérifier les bans appliqués dans la table « IPs bannies » ci-dessus.</li>
        </ul>
      </section>
      <!-- Last successful logins -->
      <section class="card">
        <div class="card-head">
          <h2>✅ Derniers logins Discord OAuth ({{ lastLogins.length }})</h2>
          <button class="btn xs" @click="loadLastLogins">↻</button>
        </div>
        <p class="muted small">20 derniers utilisateurs qui se sont connectés via OAuth Discord.</p>
        <table v-if="lastLogins.length > 0" class="data-table">
          <thead>
            <tr><th>Quand</th><th>Username</th><th>Discord ID</th><th>IP</th><th>User-Agent</th></tr>
          </thead>
          <tbody>
            <tr v-for="(l, i) in lastLogins" :key="i">
              <td class="small muted">{{ fmtDate(l.timestamp) }}</td>
              <td><strong>{{ l.username ?? "—" }}</strong></td>
              <td class="small mono">{{ l.discord_user_id }}</td>
              <td class="small mono">{{ l.client_ip ?? "—" }}</td>
              <td class="small muted ua">{{ truncate(l.user_agent ?? "", 80) }}</td>
            </tr>
          </tbody>
        </table>
        <div v-else class="empty">Aucun login enregistré. Connecte-toi pour générer le premier event.</div>
      </section>
    </div>

    <!-- ════════ ONGLET 4 : RÉSEAU ════════ -->
    <div v-if="currentTab === 'network'" class="tab-content">
      <!-- Géolocalisation IPs (Top IPs + Auth failures) -->
      <section class="card">
        <div class="card-head">
          <h2>🌍 Géolocalisation IPs ({{ geoRows.length }})</h2>
          <button class="btn xs" @click="loadGeoForAll">↻</button>
        </div>
        <p class="muted small">
          Pays / FAI / ASN pour les IPs vues sur Top IPs et Auth failures.
          Lookup via ip-api.com (gratuit, 45 req/min, 100 IPs max par requête).
        </p>
        <table v-if="geoRows.length > 0" class="data-table">
          <thead>
            <tr>
              <th>IP</th><th>Pays</th><th>Ville</th><th>FAI</th><th>ASN</th>
              <th class="num">Total</th><th class="num">Failed</th>
              <th class="actions-h">Action</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="r in geoRows.slice(0, 100)" :key="r.ip" :class="{ alert: (r.failed ?? 0) > 10 }">
              <td><code>{{ r.ip }}</code></td>
              <td class="small">
                <span v-if="r.countryCode">{{ r.countryCode }} · {{ r.country }}</span>
                <span v-else class="muted">—</span>
              </td>
              <td class="small">{{ r.city ?? "—" }}</td>
              <td class="small muted">{{ truncate(r.isp ?? "—", 30) }}</td>
              <td class="small mono">{{ truncate(r.asn ?? "—", 24) }}</td>
              <td class="num">{{ r.total }}</td>
              <td class="num" :class="{ danger: (r.failed ?? 0) > 10 }">{{ r.failed ?? 0 }}</td>
              <td class="actions">
                <button v-if="canManage" class="btn xs danger" @click="banIp(r.ip)">🚫</button>
              </td>
            </tr>
          </tbody>
        </table>
        <div v-else class="empty">Aucune IP à géolocaliser pour le moment.</div>
      </section>

      <!-- Connexions actives -->
      <section class="card">
        <div class="card-head">
          <h2>🔗 Connexions actives ({{ connections?.total ?? 0 }})</h2>
          <button class="btn xs" @click="loadConnections">↻</button>
        </div>
        <div v-if="connectionsError" class="info">
          <p class="small">{{ connectionsError }}</p>
          <p class="hint small">Setup : <code>sudo bash infra/scripts/setup-host-security.sh connections</code></p>
        </div>
        <div v-else-if="connections">
          <p class="muted small">Maj {{ fmtDate(connections.updated_at) }}</p>
          <table v-if="connections.connections.length > 0" class="data-table">
            <thead>
              <tr><th>État</th><th>Local</th><th>Remote</th></tr>
            </thead>
            <tbody>
              <tr v-for="(c, i) in connections.connections.slice(0, 50)" :key="i">
                <td><span class="code-pill">{{ c.state }}</span></td>
                <td class="small mono">{{ c.local_addr }}</td>
                <td class="small mono">{{ c.remote_addr }}</td>
              </tr>
            </tbody>
          </table>
          <div v-else class="empty">Aucune connexion établie.</div>
        </div>
      </section>

      <!-- Open ports -->
      <section class="card">
        <div class="card-head">
          <h2>🔍 Ports ouverts ({{ openPorts?.ports.length ?? 0 }})</h2>
          <button class="btn xs" @click="loadOpenPorts">↻</button>
        </div>
        <div v-if="portsError" class="info">
          <p class="small">{{ portsError }}</p>
          <p class="hint small">Setup : <code>sudo bash infra/scripts/setup-host-security.sh open-ports</code></p>
        </div>
        <div v-else-if="openPorts">
          <p class="muted small">
            Maj {{ fmtDate(openPorts.updated_at) }} ·
            <strong v-if="openPorts.unexpected_count > 0" class="alert">⚠️ {{ openPorts.unexpected_count }} port(s) inattendu(s)</strong>
            <span v-else>✅ Aucun port inattendu</span>
          </p>
          <table v-if="openPorts.ports.length > 0" class="data-table">
            <thead>
              <tr><th>Port</th><th>Protocole</th><th>Service</th><th>Statut</th></tr>
            </thead>
            <tbody>
              <tr v-for="p in openPorts.ports" :key="`${p.port}-${p.protocol}`" :class="{ alert: !p.expected }">
                <td><code>{{ p.port }}</code></td>
                <td class="small">{{ p.protocol }}</td>
                <td class="small">{{ p.service ?? "—" }}</td>
                <td>
                  <span v-if="p.expected" class="badge ok">attendu</span>
                  <span v-else class="badge danger">⚠️ INATTENDU</span>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </section>

      <!-- Connexions outbound -->
      <section class="card">
        <div class="card-head">
          <h2>🌐 Connexions outbound ({{ outbound?.total ?? 0 }})</h2>
          <button class="btn xs" @click="loadOutbound">↻</button>
        </div>
        <div v-if="outboundError" class="info">
          <p class="small">{{ outboundError }}</p>
          <p class="hint small">Setup : <code>sudo bash infra/scripts/setup-host-security.sh outbound</code></p>
        </div>
        <div v-else-if="outbound">
          <p class="muted small">
            Maj {{ fmtDate(outbound.updated_at) }} · IPs externes contactées par les services serveur
          </p>
          <table v-if="outbound.connections.length > 0" class="data-table">
            <thead>
              <tr><th>Local</th><th>Remote (IP externe)</th><th>Process</th></tr>
            </thead>
            <tbody>
              <tr v-for="(c, i) in outbound.connections" :key="i">
                <td class="small mono">{{ c.local_addr }}</td>
                <td class="small mono">{{ c.remote_addr }}</td>
                <td class="small">{{ c.process ?? "—" }}</td>
              </tr>
            </tbody>
          </table>
          <div v-else class="empty">Aucune connexion sortante détectée.</div>
        </div>
      </section>
      <!-- TLS handshake errors -->
      <section class="card">
        <div class="card-head">
          <h2>🔐 Erreurs handshake TLS (24h)</h2>
          <button class="btn xs" @click="loadTlsErrors">↻</button>
        </div>
        <div v-if="tlsErrorsError" class="info">
          <p class="small">{{ tlsErrorsError }}</p>
          <p class="hint small">Setup : <code>sudo bash infra/scripts/setup-host-security.sh tls-errors</code></p>
        </div>
        <div v-else-if="tlsErrors">
          <p class="muted small">Maj {{ fmtDate(tlsErrors.updated_at) }} · {{ tlsErrors.total_24h }} erreur(s)</p>
          <table v-if="tlsErrors.entries.length > 0" class="data-table">
            <thead><tr><th>Client</th><th>Erreur</th><th></th></tr></thead>
            <tbody>
              <tr v-for="(e, i) in tlsErrors.entries.slice(0, 30)" :key="i">
                <td class="small mono">{{ e.client }}</td>
                <td class="small">{{ truncate(e.error, 120) }}</td>
                <td><button v-if="canManage && e.client !== '?'" class="btn xs danger" @click="banIp(e.client)">🚫</button></td>
              </tr>
            </tbody>
          </table>
          <div v-else class="empty">Aucune erreur TLS handshake 🎉</div>
        </div>
      </section>
    </div>

    <!-- ════════ ONGLET 5 : INTÉGRITÉ ════════ -->
    <div v-if="currentTab === 'integrity'" class="tab-content">
      <!-- Cert TLS -->
      <section class="card">
        <h2>🔐 Certificat TLS</h2>
        <div v-if="tlsError" class="error">
          Erreur lecture cert : <code>{{ tlsError }}</code>
        </div>
        <div v-else-if="tls" class="tls-info">
          <div class="tls-row"><span class="lbl">Domaine</span><strong>{{ tls.domain }}</strong></div>
          <div class="tls-row"><span class="lbl">Émetteur</span><span class="small">{{ tls.issuer }}</span></div>
          <div class="tls-row"><span class="lbl">Valide depuis</span><span class="small">{{ tls.not_before }}</span></div>
          <div class="tls-row"><span class="lbl">Expire le</span><span class="small">{{ tls.not_after }}</span></div>
          <div class="tls-row">
            <span class="lbl">Statut</span>
            <span class="badge" :class="tlsBadgeClass">
              <template v-if="tls.is_expired">⚠️ Expiré</template>
              <template v-else-if="tls.is_warning">⚠️ Expire dans {{ tls.days_until_expiry }} j</template>
              <template v-else>✅ Valide ({{ tls.days_until_expiry }} j restants)</template>
            </span>
          </div>
        </div>
        <div v-else class="empty">Chargement…</div>
      </section>

      <!-- Container changes -->
      <section class="card">
        <div class="card-head">
          <h2>🐳 Conteneurs surveillés</h2>
          <button class="btn xs" @click="loadContainers">↻</button>
        </div>
        <div v-if="containersError" class="info">
          <p class="small">{{ containersError }}</p>
        </div>
        <div v-else-if="containers">
          <p class="muted small">
            Dernier check {{ fmtDate(containers.last_check) }} ·
            {{ containers.current.length }} conteneur(s) ·
            {{ containers.changes_24h.length }} changement(s) recent(s)
          </p>
          <table v-if="containers.changes_24h.length > 0" class="data-table">
            <thead><tr><th>Quand</th><th>Type</th><th>Conteneur</th><th>Image</th></tr></thead>
            <tbody>
              <tr v-for="(c, i) in containers.changes_24h.slice().reverse().slice(0, 20)" :key="i"
                  :class="{ alert: c.kind === 'removed' || c.kind === 'image_changed' }">
                <td class="small">{{ fmtDate(c.timestamp) }}</td>
                <td>
                  <span class="badge" :class="c.kind === 'removed' ? 'danger' : c.kind === 'image_changed' ? 'warning' : ''">
                    {{ c.kind }}
                  </span>
                </td>
                <td class="small mono">{{ c.container.name }}</td>
                <td class="small mono">{{ truncate(c.container.image, 40) }}</td>
              </tr>
            </tbody>
          </table>
          <details v-if="containers.current.length > 0" class="small">
            <summary class="muted">Liste actuelle ({{ containers.current.length }})</summary>
            <table class="data-table">
              <thead><tr><th>Nom</th><th>Image</th><th>État</th></tr></thead>
              <tbody>
                <tr v-for="c in containers.current" :key="c.id">
                  <td class="mono">{{ c.name }}</td>
                  <td class="mono">{{ truncate(c.image, 40) }}</td>
                  <td>{{ c.state }}</td>
                </tr>
              </tbody>
            </table>
          </details>
        </div>
      </section>

      <!-- Trivy vulnerabilities -->
      <section class="card">
        <div class="card-head">
          <h2>🐳 Vulnérabilités Docker (Trivy)</h2>
          <button class="btn xs" @click="loadTrivy">↻</button>
        </div>
        <div v-if="trivyError" class="info">
          <p class="small">{{ trivyError }}</p>
          <p class="hint small">Setup : <code>sudo bash infra/scripts/setup-host-security.sh trivy</code></p>
        </div>
        <div v-else-if="trivy">
          <p class="muted small">
            Maj {{ fmtDate(trivy.updated_at) }} · Scan automatique 1×/jour à 3h
          </p>
          <div class="cve-summary">
            <div class="cve-stat danger"><span class="lbl">Critical</span><strong>{{ trivy.critical }}</strong></div>
            <div class="cve-stat warning"><span class="lbl">High</span><strong>{{ trivy.high }}</strong></div>
            <div class="cve-stat"><span class="lbl">Medium</span><strong>{{ trivy.medium }}</strong></div>
            <div class="cve-stat ok"><span class="lbl">Low</span><strong>{{ trivy.low }}</strong></div>
          </div>
          <table v-if="trivy.vulnerabilities.length > 0" class="data-table">
            <thead>
              <tr><th>Image</th><th>CVE</th><th>Sévérité</th><th>Package</th><th>Fix</th></tr>
            </thead>
            <tbody>
              <tr v-for="v in trivy.vulnerabilities.filter(v => v.severity === 'CRITICAL' || v.severity === 'HIGH').slice(0, 50)" :key="v.cve + v.image">
                <td class="small mono">{{ v.image }}</td>
                <td class="small mono"><a :href="`https://nvd.nist.gov/vuln/detail/${v.cve}`" target="_blank" rel="noopener noreferrer">{{ v.cve }}</a></td>
                <td>
                  <span class="badge" :class="{ danger: v.severity === 'CRITICAL', warning: v.severity === 'HIGH' }">
                    {{ v.severity }}
                  </span>
                </td>
                <td class="small">{{ v.package ?? "—" }}</td>
                <td class="small">{{ v.fixed_version ?? "—" }}</td>
              </tr>
            </tbody>
          </table>
          <div v-else class="empty">✅ Aucune vulnérabilité critique/high détectée.</div>
        </div>
      </section>

      <!-- Intégrité fichiers -->
      <section class="card">
        <div class="card-head">
          <h2>📁 Intégrité fichiers critiques ({{ integrity?.files.length ?? 0 }})</h2>
          <button class="btn xs" @click="loadIntegrity">↻</button>
        </div>
        <div v-if="integrityError" class="info">
          <p class="small">{{ integrityError }}</p>
          <p class="hint small">Setup : <code>sudo bash infra/scripts/setup-host-security.sh file-integrity</code></p>
        </div>
        <div v-else-if="integrity">
          <p class="muted small">
            Maj {{ fmtDate(integrity.updated_at) }} ·
            <strong v-if="integrity.modified_count > 0" class="alert">⚠️ {{ integrity.modified_count }} fichier(s) modifié(s)</strong>
            <span v-else>✅ Tous les fichiers correspondent au baseline</span>
          </p>
          <table v-if="integrity.files.length > 0" class="data-table">
            <thead>
              <tr><th>Chemin</th><th>SHA256</th><th>Modifié</th><th>Statut</th></tr>
            </thead>
            <tbody>
              <tr v-for="f in integrity.files" :key="f.path" :class="{ alert: f.status === 'modified' }">
                <td class="small mono">{{ f.path }}</td>
                <td class="small mono muted">{{ f.sha256.slice(0, 16) }}…</td>
                <td class="small muted">{{ f.modified_at }}</td>
                <td>
                  <span class="badge" :class="{
                    ok: f.status === 'ok',
                    danger: f.status === 'modified',
                    warning: f.status === 'missing'
                  }">
                    {{ f.status === 'ok' ? '✅ OK' : f.status === 'modified' ? '⚠️ MODIFIÉ' : '❌ MANQUANT' }}
                  </span>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </section>
    </div>

    <!-- ════════ ONGLET 6 : AUDIT ════════ -->
    <div v-if="currentTab === 'audit'" class="tab-content">
      <section class="card">
        <div class="card-head">
          <h2>📋 Events serveur ({{ serverEvents.length }})</h2>
          <div class="card-actions">
            <select v-model="eventsFilter" @change="loadServerEvents">
              <option value="all">Toutes actions</option>
              <option value="docker">Docker</option>
              <option value="security">Sécurité</option>
              <option value="rbac">RBAC</option>
            </select>
            <button class="btn xs" @click="loadServerEvents">↻</button>
          </div>
        </div>
        <p class="muted small">Actions admin sur l'infra (start/stop conteneurs, ban IP, cleanup logs, etc.).</p>
        <table v-if="serverEvents.length > 0" class="data-table">
          <thead>
            <tr><th>Quand</th><th>Sévérité</th><th>Action</th><th>Acteur</th><th>Cible</th></tr>
          </thead>
          <tbody>
            <tr v-for="e in serverEvents" :key="e.id">
              <td class="small muted">{{ fmtDate(e.timestamp) }}</td>
              <td>
                <span class="code-pill" :class="{ warn: e.severity === 'warn', danger: e.severity === 'critical' }">
                  {{ e.severity }}
                </span>
              </td>
              <td><code class="event-type">{{ e.action }}</code></td>
              <td class="small mono">{{ e.actor ?? "—" }}</td>
              <td class="small mono">{{ e.target ?? "—" }}</td>
            </tr>
          </tbody>
        </table>
        <div v-else class="empty">Aucun event serveur enregistré.</div>
      </section>

    </div>

    <!-- ════════ ONGLET 7 : ALERTES ════════ -->
    <div v-if="currentTab === 'alerts'" class="tab-content">
      <section class="card">
        <h2>🔔 Notifications Discord</h2>
        <p class="muted small">
          Worker côté API qui poll les indicateurs critiques toutes les
          <code>SECURITY_ALERTS_INTERVAL_SECS</code> (défaut 5 min) et envoie
          un webhook Discord configuré via <code>SECURITY_ALERTS_WEBHOOK</code>.
        </p>
        <h3 class="small">Seuils déclencheurs</h3>
        <ul class="muted small">
          <li><strong>Brute-force auth</strong> : &gt; 50 echecs HTTP 401/403 sur 1h</li>
          <li><strong>Conteneurs</strong> : kind = <code>removed</code> ou <code>image_changed</code></li>
          <li><strong>TLS expiration</strong> : certificat à moins de 14 jours</li>
        </ul>
        <p class="muted small">
          Anti-spam : chaque alerte n'est envoyée qu'une fois (clef
          composite), reset après 500 alertes accumulées.
        </p>
        <p class="muted small">
          Pour configurer : créer un webhook Discord (Paramètres canal →
          Intégrations → Webhooks) et ajouter <code>SECURITY_ALERTS_WEBHOOK=https://discord.com/api/webhooks/...</code>
          dans <code>.env</code>, puis <code>docker compose restart api</code>.
        </p>
      </section>
    </div>

    <!-- ── Modale cleanup (commun à tous les onglets) ── -->
    <div v-if="showCleanupModal" class="modal-backdrop" @click.self="showCleanupModal = false">
      <div class="modal-card">
        <h3>🗑 Nettoyer les logs de sécurité</h3>
        <p class="muted">
          Vide la table <code>logs</code> (Top IPs + Échecs auth) selon le délai
          choisi, et optionnellement la table <code>audit_logs</code> (events Discord).
          <strong>0 jours = tout supprimer.</strong>
        </p>
        <div class="modal-form">
          <label>
            Ne garder que les logs de moins de :
            <select v-model.number="cleanupOpts.days">
              <option :value="0">0 jours (TOUT supprimer)</option>
              <option :value="1">1 jour</option>
              <option :value="7">7 jours</option>
              <option :value="30">30 jours</option>
              <option :value="90">90 jours</option>
            </select>
          </label>
          <label class="checkbox">
            <input type="checkbox" v-model="cleanupOpts.includeAudit" />
            Inclure aussi les audit logs Discord (member_join, etc.)
          </label>
        </div>
        <div class="modal-actions">
          <button class="btn" :disabled="cleaning" @click="showCleanupModal = false">Annuler</button>
          <button class="btn danger" :disabled="cleaning" @click="runCleanup">
            {{ cleaning ? "Nettoyage…" : "🗑 Confirmer" }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.security-page { padding: 16px; }
.page-header {
  display: flex; justify-content: space-between; align-items: flex-start;
  flex-wrap: wrap; gap: 16px; margin-bottom: 16px;
}
.page-header h1 { margin: 0 0 4px; font-size: 1.6rem; }
.muted { color: var(--text-secondary); margin: 0; }
.muted.small { font-size: 12px; }
.small { font-size: 11px; }
.mono { font-family: "JetBrains Mono", monospace; }
.header-actions { display: flex; gap: 8px; align-items: center; }

/* Tabs */
.tabs {
  display: flex;
  gap: 4px;
  border-bottom: 1px solid var(--border);
  margin-bottom: 20px;
  overflow-x: auto;
}
.tab {
  background: transparent;
  border: none;
  border-bottom: 2px solid transparent;
  padding: 10px 16px;
  font-size: 13px;
  font-weight: 600;
  color: var(--text-secondary);
  cursor: pointer;
  white-space: nowrap;
  transition: all 0.15s ease;
}
.tab:hover { color: var(--text-primary); }
.tab.active {
  color: var(--accent);
  border-bottom-color: var(--accent);
}
.tab-icon { margin-right: 4px; }
.tab-content { animation: fadeIn 0.15s ease; }
@keyframes fadeIn { from { opacity: 0; } to { opacity: 1; } }

/* KPIs */
.kpis {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
  gap: 14px;
  margin-bottom: 20px;
}
.kpi-card {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 12px;
  padding: 16px 18px;
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.kpi-label { font-size: 11px; color: var(--text-secondary); text-transform: uppercase; letter-spacing: 0.5px; font-weight: 600; }
.kpi-value { font-size: 28px; font-weight: 700; color: var(--text-primary); }
.kpi-value.warning { color: var(--warning, #e67e22); }
.kpi-value.danger { color: var(--danger); }
.kpi-value.ok { color: var(--success, #2ecc71); }
.kpi-hint { font-size: 11px; color: var(--text-secondary); }
.overview-hint {
  background: color-mix(in srgb, var(--accent) 6%, var(--bg-secondary));
  border-left: 3px solid var(--accent);
  padding: 14px 16px;
  border-radius: 4px;
  font-size: 13px;
  line-height: 1.5;
}

/* Cards */
.card {
  background: var(--bg-card); border: 1px solid var(--border); border-radius: 12px;
  padding: 18px 20px; margin-bottom: 16px;
}
.card h2 { margin: 0 0 12px; font-size: 16px; }
.card-head {
  display: flex; justify-content: space-between; align-items: center;
  flex-wrap: wrap; gap: 10px; margin-bottom: 12px;
}
.card-head h2 { margin: 0; }
.card-actions { display: flex; gap: 8px; align-items: center; }
.placeholder-card {
  background: color-mix(in srgb, var(--accent) 4%, var(--bg-card));
  border-style: dashed;
}

/* Buttons */
.btn {
  padding: 7px 14px; border-radius: 8px; border: 1px solid var(--border);
  background: var(--bg-secondary); color: var(--text-primary);
  font-size: 12px; font-weight: 600; cursor: pointer;
}
.btn:hover:not(:disabled) { border-color: var(--accent); color: var(--accent); }
.btn:disabled { opacity: 0.5; cursor: not-allowed; }
.btn.xs { padding: 3px 8px; font-size: 11px; }
.btn.primary { background: var(--accent); color: white; border-color: var(--accent); }
.btn.primary:hover:not(:disabled) { filter: brightness(1.1); color: white; }
.btn.danger { border-color: color-mix(in srgb, var(--danger) 50%, var(--border)); color: var(--danger); }
.btn.danger:hover:not(:disabled) { background: color-mix(in srgb, var(--danger) 15%, var(--bg-secondary)); }

select {
  padding: 5px 8px; border-radius: 6px; border: 1px solid var(--border);
  background: var(--bg-secondary); color: var(--text-primary); font-size: 12px;
}

/* Tables */
.data-table { width: 100%; border-collapse: collapse; font-size: 12px; }
.data-table th, .data-table td {
  padding: 8px 10px; text-align: left;
  border-bottom: 1px solid color-mix(in srgb, var(--border) 60%, transparent);
}
.data-table th { font-size: 10px; text-transform: uppercase; color: var(--text-secondary); letter-spacing: 0.5px; }
.data-table .num { text-align: right; }
.data-table .danger { color: var(--danger); font-weight: 700; }
.data-table tr.alert { background: color-mix(in srgb, var(--danger) 6%, transparent); }
.data-table .ua { max-width: 350px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.actions-h { text-align: right; }
.actions { text-align: right; white-space: nowrap; }

.code-pill {
  display: inline-block; padding: 2px 8px; border-radius: 6px;
  font-size: 10px; font-family: "JetBrains Mono", monospace; font-weight: 700;
  background: var(--bg-secondary); color: var(--text-secondary);
}
.code-pill.warn { background: color-mix(in srgb, var(--warning, #e67e22) 18%, var(--bg-secondary)); color: var(--warning, #e67e22); }
.code-pill.danger { background: color-mix(in srgb, var(--danger) 18%, var(--bg-secondary)); color: var(--danger); }

.event-type {
  font-family: "JetBrains Mono", monospace; font-size: 11px;
  background: var(--bg-secondary); padding: 2px 6px; border-radius: 4px;
}

.empty { padding: 20px; text-align: center; color: var(--text-secondary); font-size: 12px; font-style: italic; }
.info { padding: 14px; background: color-mix(in srgb, var(--accent) 6%, var(--bg-secondary)); border-left: 3px solid var(--accent); border-radius: 4px; }
.info p { margin: 0 0 8px; font-size: 13px; }
.info .hint { font-family: "JetBrains Mono", monospace; }
.error { padding: 12px; background: color-mix(in srgb, var(--danger) 8%, transparent); border-left: 3px solid var(--danger); border-radius: 4px; font-size: 13px; }

/* Jails (fail2ban) */
.jail-card { background: var(--bg-secondary); border: 1px solid var(--border); border-radius: 8px; padding: 12px 14px; margin-bottom: 10px; }
.jail-head { display: flex; justify-content: space-between; align-items: center; flex-wrap: wrap; gap: 8px; margin-bottom: 8px; }
.ip-list { list-style: none; margin: 0; padding: 0; display: flex; flex-wrap: wrap; gap: 6px; }
.ip-pill { display: inline-flex; align-items: center; gap: 6px; background: var(--bg-card); border: 1px solid var(--border); padding: 4px 6px 4px 10px; border-radius: 6px; }
.ip-list li code { font-family: "JetBrains Mono", monospace; font-size: 11px; }

/* TLS info */
.tls-info { display: flex; flex-direction: column; gap: 10px; }
.tls-row { display: grid; grid-template-columns: 130px 1fr; gap: 12px; align-items: center; }
.tls-row .lbl { color: var(--text-secondary); font-size: 11px; text-transform: uppercase; letter-spacing: 0.4px; }
.badge { display: inline-block; padding: 4px 10px; border-radius: 12px; font-size: 12px; font-weight: 600; }
.badge.ok { background: color-mix(in srgb, var(--success, #2ecc71) 18%, transparent); color: var(--success, #2ecc71); }
.badge.warning { background: color-mix(in srgb, var(--warning, #e67e22) 18%, transparent); color: var(--warning, #e67e22); }
.badge.danger { background: color-mix(in srgb, var(--danger) 18%, transparent); color: var(--danger); }

/* CVE summary cards */
.cve-summary { display: flex; gap: 14px; margin: 12px 0; flex-wrap: wrap; }
.cve-stat {
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 10px 16px;
  display: flex; flex-direction: column;
  min-width: 90px;
}
.cve-stat .lbl { font-size: 10px; color: var(--text-secondary); text-transform: uppercase; letter-spacing: 0.4px; }
.cve-stat strong { font-size: 22px; }
.cve-stat.danger strong { color: var(--danger); }
.cve-stat.warning strong { color: var(--warning, #e67e22); }
.cve-stat.ok strong { color: var(--success, #2ecc71); }

.alert { color: var(--warning, #e67e22); font-weight: 600; }

/* Modal */
.modal-backdrop {
  position: fixed; inset: 0; background: rgba(0,0,0,0.7);
  display: flex; align-items: center; justify-content: center; z-index: 1000; padding: 30px;
}
.modal-card { background: var(--bg-card); border: 1px solid var(--border); border-radius: 14px; padding: 24px 28px; max-width: 500px; width: 100%; }
.modal-card h3 { margin: 0 0 12px; font-size: 17px; }
.modal-form { display: flex; flex-direction: column; gap: 14px; margin: 18px 0; }
.modal-form label { display: flex; flex-direction: column; gap: 6px; font-size: 12px; color: var(--text-secondary); }
.modal-form label.checkbox { flex-direction: row; align-items: center; gap: 8px; }
.modal-form select { padding: 7px 10px; border-radius: 6px; border: 1px solid var(--border); background: var(--bg-secondary); color: var(--text-primary); font-size: 13px; }
.modal-actions { display: flex; justify-content: flex-end; gap: 10px; }

@media (max-width: 768px) {
  /* Tous les tableaux : scroll horizontal pour eviter le debordement */
  table {
    display: block;
    overflow-x: auto;
    -webkit-overflow-scrolling: touch;
    white-space: nowrap;
    font-size: 12px;
    width: 100%;
  }
  table th,
  table td {
    padding: 6px 8px !important;
  }
  /* SHA-256, hash longs : tronquer + ellipsis */
  td .mono,
  td code {
    max-width: 140px;
    overflow: hidden;
    text-overflow: ellipsis;
    display: inline-block;
    vertical-align: middle;
  }
  /* Onglets : scroll horizontal */
  .tabs,
  .tab-list,
  [role="tablist"] {
    overflow-x: auto;
    flex-wrap: nowrap;
  }
}
</style>
