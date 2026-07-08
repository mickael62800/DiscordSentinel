<script setup lang="ts">
import { errMsg } from "@/utils/errMsg";
import AppSelect from "@/components/atoms/AppSelect.vue";
import { computed, onMounted, ref } from "vue";
import {
  serverSecurityService,
  type AuthFailureEntry,
  type SecurityWindow,
  type SshFailuresResponse,
  type SuspiciousResponse,
  type TopIpEntry,
  type GeoIpEntry,
} from "@/services/serverSecurityService";
import { useFormatDate } from "@/composables/useFormatDate";

const { formatDateTimeShort: fmtDate } = useFormatDate();
import { useToast } from "@/composables/useToast";
import { useConfirm } from "@/composables/useConfirm";
import { errMsg } from "@/utils/errMsg";
import { useMyRole } from "@/composables/useMyRole";
import TrafficTrendChart from "./TrafficTrendChart.vue";

const { success, error: showError } = useToast();
const { confirm } = useConfirm();
const { role, isSuper } = useMyRole();
const canManage = computed(() => isSuper.value || role.value === "owner");

const topIps = ref<TopIpEntry[]>([]);
const topIpsWindow = ref<SecurityWindow>("1h");
const authFailures = ref<AuthFailureEntry[]>([]);
const authWindow = ref<SecurityWindow>("24h");
const sshFailures = ref<SshFailuresResponse | null>(null);
const sshError = ref<string | null>(null);
const suspicious = ref<SuspiciousResponse | null>(null);
const suspiciousError = ref<string | null>(null);
const geoMap = ref<Record<string, GeoIpEntry>>({});

async function loadTopIps() {
  try { topIps.value = await serverSecurityService.topIps(topIpsWindow.value, 20); }
  catch (e) { showError(`Top IPs : ${errMsg(e)}`); }
}
async function loadAuthFailures() {
  try { authFailures.value = await serverSecurityService.authFailures(authWindow.value, 100); }
  catch (e) { showError(`Echecs auth : ${errMsg(e)}`); }
}
async function loadSshFailures() {
  sshError.value = null;
  try { sshFailures.value = await serverSecurityService.sshFailures(); }
  catch (e) { sshError.value = errMsg(e); sshFailures.value = null; }
}
async function loadSuspicious() {
  suspiciousError.value = null;
  try { suspicious.value = await serverSecurityService.nginxSuspicious(); }
  catch (e) { suspiciousError.value = errMsg(e); suspicious.value = null; }
}
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

async function banIp(ip: string) {
  if (
    !(await confirm({
      title: "Bannir l'IP",
      message: `Bannir l'IP ${ip} ? Elle ne pourra plus accéder au serveur.`,
    }))
  )
    return;
  try {
    const r = await serverSecurityService.banIp(ip, "ban manuel via panel sécurité");
    success(r.message);
  } catch (e) { showError(`Echec ban : ${errMsg(e)}`); }
}

async function refresh() {
  await Promise.allSettled([loadTopIps(), loadAuthFailures(), loadSshFailures(), loadSuspicious()]);
  void loadGeoForAll();
}

defineExpose({ refresh });

function truncate(s: string, n: number): string { return s.length > n ? s.slice(0, n) + "…" : s; }

onMounted(refresh);
</script>

<template>
  <div class="tab-content">
    <!-- Top IPs -->
    <section class="card">
      <div class="card-head">
        <h2>📊 Top IPs par requêtes</h2>
        <div class="card-actions">
          <AppSelect v-model="topIpsWindow" @change="loadTopIps">
            <option value="1h">1h</option>
            <option value="24h">24h</option>
            <option value="7d">7j</option>
          </AppSelect>
          <button class="btn-secondary xs" @click="loadTopIps">↻</button>
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
              <button v-if="canManage" class="btn-secondary xs danger" @click="banIp(ip.client_ip)" title="Bannir">🚫 Ban</button>
            </td>
          </tr>
        </tbody>
      </table>
      <div v-else class="empty-state">Aucune requête sur la fenêtre sélectionnée.</div>
    </section>

    <!-- Échecs d'auth -->
    <section class="card">
      <div class="card-head">
        <h2>🔒 Échecs d'authentification ({{ authFailures.length }})</h2>
        <div class="card-actions">
          <AppSelect v-model="authWindow" @change="loadAuthFailures">
            <option value="1h">1h</option>
            <option value="24h">24h</option>
            <option value="7d">7j</option>
          </AppSelect>
          <button class="btn-secondary xs" @click="loadAuthFailures">↻</button>
        </div>
      </div>
      <table v-if="authFailures.length > 0" class="data-table">
        <thead>
          <tr><th>Quand</th><th>Code</th><th>Méthode</th><th>Route</th><th>IP</th><th>User-Agent</th><th class="actions-h">Action</th></tr>
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
              <button v-if="canManage && e.client_ip && e.client_ip !== '-'" class="btn-secondary xs danger" @click="banIp(e.client_ip)" title="Bannir">🚫</button>
            </td>
          </tr>
        </tbody>
      </table>
      <div v-else class="empty-state">Aucun échec d'authentification 🎉</div>
    </section>

    <!-- SSH failures -->
    <section class="card">
      <div class="card-head">
        <h2>🔑 Tentatives SSH échouées</h2>
        <button class="btn-secondary xs" @click="loadSshFailures">↻</button>
      </div>
      <div v-if="sshError" class="info-banner">
        <p class="small">{{ sshError }}</p>
        <p class="hint small">Setup : <code>sudo bash sentinel-infrastructure/scripts/setup-host-security.sh ssh-failures</code></p>
      </div>
      <div v-else-if="sshFailures">
        <p class="muted small">
          <strong>{{ sshFailures.total_24h }}</strong> tentatives sur 24h · Maj {{ fmtDate(sshFailures.updated_at) }}
        </p>
        <table v-if="sshFailures.entries.length > 0" class="data-table">
          <thead><tr><th>Quand</th><th>Utilisateur</th><th>IP</th><th>Action</th></tr></thead>
          <tbody>
            <tr v-for="(e, i) in sshFailures.entries.slice(0, 30)" :key="i">
              <td class="small muted">{{ e.timestamp }}</td>
              <td class="small mono">{{ e.user }}</td>
              <td class="small mono">{{ e.ip }}</td>
              <td class="actions">
                <button v-if="canManage && e.ip !== '?'" class="btn-secondary xs danger" @click="banIp(e.ip)">🚫 Ban</button>
              </td>
            </tr>
          </tbody>
        </table>
        <div v-else class="empty-state">Aucune tentative SSH ratée 🎉</div>
      </div>
    </section>

    <!-- Patterns suspects nginx -->
    <section class="card">
      <div class="card-head">
        <h2>🕷 Patterns suspects nginx (24h)</h2>
        <button class="btn-secondary xs" @click="loadSuspicious">↻</button>
      </div>
      <div v-if="suspiciousError" class="info-banner">
        <p class="small">{{ suspiciousError }}</p>
        <p class="hint small">Setup : <code>sudo bash sentinel-infrastructure/scripts/setup-host-security.sh nginx-suspicious</code></p>
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
              <td><button v-if="canManage" class="btn-secondary xs danger" @click="banIp(e.ip)">🚫</button></td>
            </tr>
          </tbody>
        </table>
        <div v-else class="empty-state">Aucun pattern suspect détecté 🎉</div>
      </div>
    </section>

    <TrafficTrendChart />
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
select {
  padding: 5px 8px; border-radius: 6px; border: 1px solid var(--border);
  background: var(--bg-secondary); color: var(--text-primary); font-size: 12px;
}
</style>
