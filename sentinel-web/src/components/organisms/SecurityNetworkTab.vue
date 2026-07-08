<script setup lang="ts">
import { errMsg } from "@/utils/errMsg";
import { computed, onMounted, ref } from "vue";
import {
  serverSecurityService,
  type AuthFailureEntry,
  type ConnectionsResponse,
  type GeoIpEntry,
  type OpenPortsResponse,
  type OutboundResponse,
  type SecurityWindow,
  type TlsErrorsResponse,
  type TopIpEntry,
} from "@/services/serverSecurityService";
import { useFormatDate } from "@/composables/useFormatDate";

const { formatDateTimeShort: fmtDate } = useFormatDate();
import { useToast } from "@/composables/useToast";
import { useConfirm } from "@/composables/useConfirm";
import { useMyRole } from "@/composables/useMyRole";

const { success, error: showError } = useToast();
const { confirm } = useConfirm();
const { role, isSuper } = useMyRole();
const canManage = computed(() => isSuper.value || role.value === "owner");

const topIps = ref<TopIpEntry[]>([]);
const topIpsWindow = ref<SecurityWindow>("1h");
const authFailures = ref<AuthFailureEntry[]>([]);
const authWindow = ref<SecurityWindow>("24h");
const connections = ref<ConnectionsResponse | null>(null);
const connectionsError = ref<string | null>(null);
const openPorts = ref<OpenPortsResponse | null>(null);
const portsError = ref<string | null>(null);
const outbound = ref<OutboundResponse | null>(null);
const outboundError = ref<string | null>(null);
const tlsErrors = ref<TlsErrorsResponse | null>(null);
const tlsErrorsError = ref<string | null>(null);
const geoMap = ref<Record<string, GeoIpEntry>>({});

interface GeoRow {
  ip: string; total: number; failed: number;
  country?: string; countryCode?: string; city?: string; isp?: string; asn?: string;
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
      r.country = g.country; r.countryCode = g.countryCode;
      r.city = g.city; r.isp = g.isp; r.asn = g.asn;
    }
  }
  return Array.from(map.values()).sort((a, b) => b.total + b.failed - (a.total + a.failed));
});

async function loadGeoForAll() {
  await Promise.allSettled([
    serverSecurityService.topIps(topIpsWindow.value, 20).then((r) => { topIps.value = r; }),
    serverSecurityService.authFailures(authWindow.value, 100).then((r) => { authFailures.value = r; }),
  ]);
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

async function loadConnections() {
  connectionsError.value = null;
  try { connections.value = await serverSecurityService.connections(); }
  catch (e) { connectionsError.value = errMsg(e); connections.value = null; }
}
async function loadOpenPorts() {
  portsError.value = null;
  try { openPorts.value = await serverSecurityService.openPorts(); }
  catch (e) { portsError.value = errMsg(e); openPorts.value = null; }
}
async function loadOutbound() {
  outboundError.value = null;
  try { outbound.value = await serverSecurityService.outbound(); }
  catch (e) { outboundError.value = errMsg(e); outbound.value = null; }
}
async function loadTlsErrors() {
  tlsErrorsError.value = null;
  try { tlsErrors.value = await serverSecurityService.tlsErrors(); }
  catch (e) { tlsErrorsError.value = errMsg(e); tlsErrors.value = null; }
}

async function banIp(ip: string) {
  if (!(await confirm({ title: "Bannir l'IP", message: `Bannir l'IP ${ip} ?` }))) return;
  try {
    const r = await serverSecurityService.banIp(ip, "ban manuel via panel sécurité");
    success(r.message);
  } catch (e) { showError(`Echec ban : ${errMsg(e)}`); }
}

async function refresh() {
  await Promise.allSettled([
    loadGeoForAll(), loadConnections(), loadOpenPorts(), loadOutbound(), loadTlsErrors(),
  ]);
}

defineExpose({ refresh });

function truncate(s: string, n: number): string { return s.length > n ? s.slice(0, n) + "…" : s; }

onMounted(refresh);
</script>

<template>
  <div class="tab-content">
    <!-- Géolocalisation IPs -->
    <section class="card">
      <div class="card-head">
        <h2>🌍 Géolocalisation IPs ({{ geoRows.length }})</h2>
        <button class="btn-secondary xs" @click="loadGeoForAll">↻</button>
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
              <button v-if="canManage" class="btn-secondary xs danger" @click="banIp(r.ip)">🚫</button>
            </td>
          </tr>
        </tbody>
      </table>
      <div v-else class="empty-state">Aucune IP à géolocaliser pour le moment.</div>
    </section>

    <!-- Connexions actives -->
    <section class="card">
      <div class="card-head">
        <h2>🔗 Connexions actives ({{ connections?.total ?? 0 }})</h2>
        <button class="btn-secondary xs" @click="loadConnections">↻</button>
      </div>
      <div v-if="connectionsError" class="info-banner">
        <p class="small">{{ connectionsError }}</p>
        <p class="hint small">Setup : <code>sudo bash sentinel-infrastructure/scripts/setup-host-security.sh connections</code></p>
      </div>
      <div v-else-if="connections">
        <p class="muted small">Maj {{ fmtDate(connections.updated_at) }}</p>
        <table v-if="connections.connections.length > 0" class="data-table">
          <thead><tr><th>État</th><th>Local</th><th>Remote</th></tr></thead>
          <tbody>
            <tr v-for="(c, i) in connections.connections.slice(0, 50)" :key="i">
              <td><span class="code-pill">{{ c.state }}</span></td>
              <td class="small mono">{{ c.local_addr }}</td>
              <td class="small mono">{{ c.remote_addr }}</td>
            </tr>
          </tbody>
        </table>
        <div v-else class="empty-state">Aucune connexion établie.</div>
      </div>
    </section>

    <!-- Open ports -->
    <section class="card">
      <div class="card-head">
        <h2>🔍 Ports ouverts ({{ openPorts?.ports.length ?? 0 }})</h2>
        <button class="btn-secondary xs" @click="loadOpenPorts">↻</button>
      </div>
      <div v-if="portsError" class="info-banner">
        <p class="small">{{ portsError }}</p>
        <p class="hint small">Setup : <code>sudo bash sentinel-infrastructure/scripts/setup-host-security.sh open-ports</code></p>
      </div>
      <div v-else-if="openPorts">
        <p class="muted small">
          Maj {{ fmtDate(openPorts.updated_at) }} ·
          <strong v-if="openPorts.unexpected_count > 0" class="alert">⚠️ {{ openPorts.unexpected_count }} port(s) inattendu(s)</strong>
          <span v-else>✅ Aucun port inattendu</span>
        </p>
        <table v-if="openPorts.ports.length > 0" class="data-table">
          <thead><tr><th>Port</th><th>Protocole</th><th>Service</th><th>Statut</th></tr></thead>
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
        <button class="btn-secondary xs" @click="loadOutbound">↻</button>
      </div>
      <div v-if="outboundError" class="info-banner">
        <p class="small">{{ outboundError }}</p>
        <p class="hint small">Setup : <code>sudo bash sentinel-infrastructure/scripts/setup-host-security.sh outbound</code></p>
      </div>
      <div v-else-if="outbound">
        <p class="muted small">Maj {{ fmtDate(outbound.updated_at) }} · IPs externes contactées par les services serveur</p>
        <table v-if="outbound.connections.length > 0" class="data-table">
          <thead><tr><th>Local</th><th>Remote (IP externe)</th><th>Process</th></tr></thead>
          <tbody>
            <tr v-for="(c, i) in outbound.connections" :key="i">
              <td class="small mono">{{ c.local_addr }}</td>
              <td class="small mono">{{ c.remote_addr }}</td>
              <td class="small">{{ c.process ?? "—" }}</td>
            </tr>
          </tbody>
        </table>
        <div v-else class="empty-state">Aucune connexion sortante détectée.</div>
      </div>
    </section>

    <!-- TLS handshake errors -->
    <section class="card">
      <div class="card-head">
        <h2>🔐 Erreurs handshake TLS (24h)</h2>
        <button class="btn-secondary xs" @click="loadTlsErrors">↻</button>
      </div>
      <div v-if="tlsErrorsError" class="info-banner">
        <p class="small">{{ tlsErrorsError }}</p>
        <p class="hint small">Setup : <code>sudo bash sentinel-infrastructure/scripts/setup-host-security.sh tls-errors</code></p>
      </div>
      <div v-else-if="tlsErrors">
        <p class="muted small">Maj {{ fmtDate(tlsErrors.updated_at) }} · {{ tlsErrors.total_24h }} erreur(s)</p>
        <table v-if="tlsErrors.entries.length > 0" class="data-table">
          <thead><tr><th>Client</th><th>Erreur</th><th></th></tr></thead>
          <tbody>
            <tr v-for="(e, i) in tlsErrors.entries.slice(0, 30)" :key="i">
              <td class="small mono">{{ e.client }}</td>
              <td class="small">{{ truncate(e.error, 120) }}</td>
              <td><button v-if="canManage && e.client !== '?'" class="btn-secondary xs danger" @click="banIp(e.client)">🚫</button></td>
            </tr>
          </tbody>
        </table>
        <div v-else class="empty-state">Aucune erreur TLS handshake 🎉</div>
      </div>
    </section>
  </div>
</template>

<style scoped>
.tab-content { display: flex; flex-direction: column; }
.card {
  background: var(--bg-card); border: 1px solid var(--border); border-radius: 12px;
  padding: 18px 20px; margin-bottom: 16px;
}
.card h2 { margin: 0 0 12px; font-size: 16px; }
.muted { color: var(--text-secondary); }
.muted.small, .small { font-size: 11px; }
.mono { font-family: "JetBrains Mono", monospace; }
.alert { color: var(--warning, #e67e22); font-weight: 600; }
</style>
