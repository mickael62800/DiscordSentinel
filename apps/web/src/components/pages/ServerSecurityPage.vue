<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import {
  serverSecurityService,
  type AuthFailureEntry,
  type BannedIpsResponse,
  type SecurityWindow,
  type ServerEventDto,
  type SuccessfulLoginEntry,
  type TlsCertInfo,
  type TopIpEntry,
} from "@/services/serverSecurityService";
import { useToast } from "@/composables/useToast";
import TrafficTrendChart from "@/components/organisms/TrafficTrendChart.vue";

const { error: showError } = useToast();

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
  await Promise.allSettled([loadTopIps(), loadAuthFailures(), loadBanned(), loadServerEvents(), loadTls(), loadLastLogins()]);
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
        <button class="btn danger" :disabled="cleaning" @click="showCleanupModal = true">🗑 Tout nettoyer</button>
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
            <tr><th>IP</th><th class="num">Total</th><th class="num">4xx/5xx</th><th>Dernier</th><th class="actions-h">Action</th></tr>
          </thead>
          <tbody>
            <tr v-for="ip in topIps" :key="ip.client_ip" :class="{ alert: ip.failed > 10 }">
              <td><code>{{ ip.client_ip }}</code></td>
              <td class="num">{{ ip.total }}</td>
              <td class="num" :class="{ danger: ip.failed > 10 }">{{ ip.failed }}</td>
              <td class="muted small">{{ fmtDate(ip.last_seen) }}</td>
              <td class="actions">
                <button class="btn xs danger" @click="banIp(ip.client_ip)" title="Bannir cette IP">🚫 Ban</button>
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
                <button v-if="e.client_ip && e.client_ip !== '-'" class="btn xs danger" @click="banIp(e.client_ip)" title="Bannir">🚫</button>
              </td>
            </tr>
          </tbody>
        </table>
        <div v-else class="empty">Aucun échec d'authentification 🎉</div>
      </section>

      <!-- Placeholder : SSH failures + Patterns nginx + Trafic anormal -->
      <section class="card placeholder-card">
        <h2>🔑 Tentatives SSH échouées</h2>
        <p class="muted small">À venir : parse de <code>auth.log</code> (host) pour remonter les tentatives SSH ratées.</p>
      </section>
      <section class="card placeholder-card">
        <h2>🕷 Patterns suspects nginx</h2>
        <p class="muted small">À venir : détection scanners (<code>/wp-admin</code>, <code>/.env</code>), tentatives SQLi/XSS dans les logs nginx.</p>
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
                <button class="btn xs" @click="unbanIp(ip)" title="Débannir cette IP">↻ Débannir</button>
              </li>
            </ul>
            <div v-else class="muted small">Aucune IP actuellement bannie sur cette jail.</div>
          </div>
        </div>
        <div v-else class="empty">fail2ban actif mais aucune jail configurée.</div>
      </section>

      <!-- Placeholders -->
      <section class="card placeholder-card">
        <h2>⚡ Rate limit dynamique</h2>
        <p class="muted small">À venir : ban automatique d'une IP qui dépasse 100 req/min sur 5 min.</p>
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
      <section class="card placeholder-card">
        <h2>🌐 Connexions outbound</h2>
        <p class="muted small">À venir : qui l'API/bot contactent à l'extérieur (détection exfiltration).</p>
      </section>
      <section class="card placeholder-card">
        <h2>🔍 Open ports check</h2>
        <p class="muted small">À venir : vérification périodique nmap externe que seuls 80/443/SSH sont ouverts.</p>
      </section>
      <section class="card placeholder-card">
        <h2>🔐 TLS handshake errors</h2>
        <p class="muted small">À venir : compteur des erreurs SSL handshake (signe de scan TLS).</p>
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

      <section class="card placeholder-card">
        <h2>📁 Intégrité fichiers critiques</h2>
        <p class="muted small">À venir : SHA256 nginx.conf, docker-compose.yml, .env. Alerte si modifié hors du process normal.</p>
      </section>
      <section class="card placeholder-card">
        <h2>🐳 Vulnérabilités Docker (Trivy)</h2>
        <p class="muted small">À venir : scan CVE des images Docker. Affiche les CVE critiques détectées.</p>
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

      <section class="card placeholder-card">
        <h2>🌍 Géolocalisation IPs</h2>
        <p class="muted small">À venir : pays + ASN (organisation hébergeur) sur les IPs des Top IPs / Auth failures.</p>
      </section>
    </div>

    <!-- ════════ ONGLET 7 : ALERTES ════════ -->
    <div v-if="currentTab === 'alerts'" class="tab-content">
      <section class="card placeholder-card">
        <h2>🔔 Notifications Discord</h2>
        <p class="muted small">
          À venir : config d'un channel Discord qui recevra les alertes critiques
          (10+ échecs auth d'une même IP, conteneur down, certificat &lt; 7j, etc.).
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
</style>
