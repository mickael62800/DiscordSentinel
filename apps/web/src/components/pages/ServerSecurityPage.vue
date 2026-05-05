<script setup lang="ts">
import AppSelect from "@/components/atoms/AppSelect.vue";
import { computed, onMounted, ref } from "vue";
import {
  serverSecurityService,
  type AuthFailureEntry,
  type BannedIpsResponse,
  type ServerEventDto,
  type SuccessfulLoginEntry,
  type TlsCertInfo,
  type TopIpEntry,
} from "@/services/serverSecurityService";
import { useToast } from "@/composables/useToast";
import { useMyRole } from "@/composables/useMyRole";
import AppTabs from "@/components/molecules/AppTabs.vue";
import AdminPageShell from "@/components/layouts/AdminPageShell.vue";
import SecurityAttacksTab from "@/components/organisms/SecurityAttacksTab.vue";
import SecurityNetworkTab from "@/components/organisms/SecurityNetworkTab.vue";
import SecurityIntegrityTab from "@/components/organisms/SecurityIntegrityTab.vue";

const { error: showError } = useToast();
const { role, isSuper } = useMyRole();
const canManage = computed(() => isSuper.value || role.value === "owner");

type TabKey = "overview" | "attacks" | "bans" | "network" | "integrity" | "audit" | "alerts";

const currentTab = ref<TabKey>("overview");
const refreshing = ref(false);
const cleaning = ref(false);
const cleanupOpts = ref({ days: 0, includeAudit: true });
const showCleanupModal = ref(false);

// Data inline pour Overview/Bans/Audit (les 3 onglets non extraits + Overview)
const overviewTopIps = ref<TopIpEntry[]>([]);
const overviewAuthFailures = ref<AuthFailureEntry[]>([]);
const banned = ref<BannedIpsResponse | null>(null);
const tls = ref<TlsCertInfo | null>(null);
const lastLogins = ref<SuccessfulLoginEntry[]>([]);
const serverEvents = ref<ServerEventDto[]>([]);
const eventsFilter = ref<"all" | "docker" | "security" | "rbac">("all");

const attacksRef = ref<InstanceType<typeof SecurityAttacksTab> | null>(null);
const networkRef = ref<InstanceType<typeof SecurityNetworkTab> | null>(null);
const integrityRef = ref<InstanceType<typeof SecurityIntegrityTab> | null>(null);

async function loadOverviewKpis() {
  await Promise.allSettled([
    serverSecurityService.topIps("1h", 20).then((r) => { overviewTopIps.value = r; }),
    serverSecurityService.authFailures("24h", 100).then((r) => { overviewAuthFailures.value = r; }),
    serverSecurityService.bannedIps().then((r) => { banned.value = r; }),
    serverSecurityService.tlsCert().then((r) => { tls.value = r; }).catch(() => {}),
  ]);
}
async function loadBanned() {
  try { banned.value = await serverSecurityService.bannedIps(); }
  catch (e: any) { showError(`Bans : ${e?.message ?? e}`); }
}
async function loadLastLogins() {
  try { lastLogins.value = await serverSecurityService.lastLogins(20); }
  catch (e: any) { showError(`Logins : ${e?.message ?? e}`); }
}
async function loadServerEvents() {
  try {
    const prefix = eventsFilter.value === "all" ? undefined : eventsFilter.value;
    serverEvents.value = await serverSecurityService.serverEvents({ action_prefix: prefix, limit: 100 });
  } catch (e: any) { showError(`Events serveur : ${e?.message ?? e}`); }
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
    loadOverviewKpis(),
    loadBanned(),
    loadLastLogins(),
    loadServerEvents(),
    attacksRef.value?.refresh(),
    networkRef.value?.refresh(),
    integrityRef.value?.refresh(),
  ]);
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

function fmtDate(s: string): string { return new Date(s).toLocaleString("fr-FR"); }
function truncate(s: string, n: number): string { return s.length > n ? s.slice(0, n) + "…" : s; }

const totalFailedRequests = computed(() =>
  overviewTopIps.value.reduce((sum, ip) => sum + ip.failed, 0),
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

const tabs = [
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
  <AdminPageShell title="Sécurité serveur" icon="🛡️">
    <template #lede>
      Surveillance des attaques, intégrité et protections actives.
    </template>
    <template #actions>
      <button v-if="canManage" class="btn-secondary danger" :disabled="cleaning" @click="showCleanupModal = true">🗑 Tout nettoyer</button>
      <button class="btn-primary" :disabled="refreshing" @click="refreshAll">
        {{ refreshing ? "Actualisation…" : "↻ Actualiser tout" }}
      </button>
    </template>

    <AppTabs
      :model-value="currentTab"
      :tabs="tabs"
      variant="plain"
      class="security-tabs-wrap"
      @update:model-value="(k) => (currentTab = k as TabKey)"
    />

    <!-- ════════ OVERVIEW (inline, petit) ════════ -->
    <div v-if="currentTab === 'overview'" class="tab-content">
      <section class="kpis">
        <div class="kpi-card">
          <span class="kpi-label">Erreurs HTTP (1h)</span>
          <span class="kpi-value">{{ totalFailedRequests }}</span>
          <span class="kpi-hint">4xx + 5xx sur Top IPs</span>
        </div>
        <div class="kpi-card">
          <span class="kpi-label">Échecs auth (24h)</span>
          <span class="kpi-value">{{ overviewAuthFailures.length }}</span>
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

    <SecurityAttacksTab v-else-if="currentTab === 'attacks'" ref="attacksRef" />

    <!-- ════════ BANS (inline) ════════ -->
    <div v-else-if="currentTab === 'bans'" class="tab-content">
      <section class="card">
        <h2>🚫 IPs bannies (fail2ban)</h2>
        <div v-if="banned && !banned.installed" class="info-banner">
          <p>{{ banned.message }}</p>
          <p class="hint small">
            Setup rapide : <code>sudo bash infra/scripts/setup-host-security.sh fail2ban</code>
          </p>
        </div>
        <div v-else-if="banned && banned.jails.length > 0">
          <p class="muted small">Maj : {{ fmtDate(banned.updated_at!) }} · {{ banned.message }}</p>
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
                <button v-if="canManage" class="btn-secondary xs" @click="unbanIp(ip)" title="Débannir">↻ Débannir</button>
              </li>
            </ul>
            <div v-else class="muted small">Aucune IP actuellement bannie sur cette jail.</div>
          </div>
        </div>
        <div v-else class="empty-state">fail2ban actif mais aucune jail configurée.</div>
      </section>

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
        </ul>
      </section>

      <section class="card">
        <div class="card-head">
          <h2>✅ Derniers logins Discord OAuth ({{ lastLogins.length }})</h2>
          <button class="btn-secondary xs" @click="loadLastLogins">↻</button>
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
        <div v-else class="empty-state">Aucun login enregistré.</div>
      </section>
    </div>

    <SecurityNetworkTab v-else-if="currentTab === 'network'" ref="networkRef" />
    <SecurityIntegrityTab v-else-if="currentTab === 'integrity'" ref="integrityRef" />

    <!-- ════════ AUDIT (inline) ════════ -->
    <div v-else-if="currentTab === 'audit'" class="tab-content">
      <section class="card">
        <div class="card-head">
          <h2>📋 Events serveur ({{ serverEvents.length }})</h2>
          <div class="card-actions">
            <AppSelect v-model="eventsFilter" @change="loadServerEvents">
              <option value="all">Toutes actions</option>
              <option value="docker">Docker</option>
              <option value="security">Sécurité</option>
              <option value="rbac">RBAC</option>
            </AppSelect>
            <button class="btn-secondary xs" @click="loadServerEvents">↻</button>
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
        <div v-else class="empty-state">Aucun event serveur enregistré.</div>
      </section>
    </div>

    <!-- ════════ ALERTS (inline, info statique) ════════ -->
    <div v-else-if="currentTab === 'alerts'" class="tab-content">
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
          Anti-spam : chaque alerte n'est envoyée qu'une fois (clef composite),
          reset après 500 alertes accumulées.
        </p>
        <p class="muted small">
          Pour configurer : créer un webhook Discord (Paramètres canal →
          Intégrations → Webhooks) et ajouter <code>SECURITY_ALERTS_WEBHOOK=https://discord.com/api/webhooks/...</code>
          dans <code>.env</code>, puis <code>docker compose restart api</code>.
        </p>
      </section>
    </div>

    <!-- ── Modale cleanup (commun) ── -->
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
            <AppSelect v-model.number="cleanupOpts.days">
              <option :value="0">0 jours (TOUT supprimer)</option>
              <option :value="1">1 jour</option>
              <option :value="7">7 jours</option>
              <option :value="30">30 jours</option>
              <option :value="90">90 jours</option>
            </AppSelect>
          </label>
          <label class="checkbox">
            <input type="checkbox" v-model="cleanupOpts.includeAudit" />
            Inclure aussi les audit logs Discord (member_join, etc.)
          </label>
        </div>
        <div class="modal-actions">
          <button class="btn-secondary" :disabled="cleaning" @click="showCleanupModal = false">Annuler</button>
          <button class="btn-secondary danger" :disabled="cleaning" @click="runCleanup">
            {{ cleaning ? "Nettoyage…" : "🗑 Confirmer" }}
          </button>
        </div>
      </div>
    </div>
  </AdminPageShell>
</template>

<style scoped>
.muted { color: var(--text-secondary); margin: 0; }
.muted.small, .small { font-size: 11px; }
.mono { font-family: "JetBrains Mono", monospace; }

.btn-primary {
  padding: 7px 14px; border-radius: 8px; border: 1px solid var(--accent);
  background: var(--accent); color: white;
  font-size: 12px; font-weight: 600; cursor: pointer;
}
.btn-primary:hover:not(:disabled) { filter: brightness(1.1); }
.btn-primary:disabled { opacity: 0.5; cursor: not-allowed; }

.security-tabs-wrap { margin-bottom: 20px; overflow-x: auto; }
.tab-content { animation: fadeIn 0.15s ease; }
@keyframes fadeIn { from { opacity: 0; } to { opacity: 1; } }

/* KPIs (overview tab) */
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
  display: flex; flex-direction: column; gap: 4px;
}
.kpi-label {
  font-size: 11px; color: var(--text-secondary);
  text-transform: uppercase; letter-spacing: 0.5px; font-weight: 600;
}
.kpi-value { font-size: 28px; font-weight: 700; color: var(--text-primary); }
.kpi-value.warning { color: var(--warning, #e67e22); }
.kpi-value.danger { color: var(--danger); }
.kpi-value.ok { color: var(--success, #2ecc71); }
.kpi-hint { font-size: 11px; color: var(--text-secondary); }
.overview-hint {
  background: color-mix(in srgb, var(--accent) 6%, var(--bg-secondary));
  border-left: 3px solid var(--accent);
  padding: 14px 16px; border-radius: 4px;
  font-size: 13px; line-height: 1.5;
}

/* Cards */
.card {
  background: var(--bg-card); border: 1px solid var(--border); border-radius: 12px;
  padding: 18px 20px; margin-bottom: 16px;
}
.card h2 { margin: 0 0 12px; font-size: 16px; }

/* Jails (fail2ban) */
.jail-card {
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 12px 14px;
  margin-bottom: 10px;
}
.jail-head {
  display: flex; justify-content: space-between; align-items: center;
  flex-wrap: wrap; gap: 8px; margin-bottom: 8px;
}
.ip-list {
  list-style: none; margin: 0; padding: 0;
  display: flex; flex-wrap: wrap; gap: 6px;
}
.ip-pill {
  display: inline-flex; align-items: center; gap: 6px;
  background: var(--bg-card); border: 1px solid var(--border);
  padding: 4px 6px 4px 10px; border-radius: 6px;
}
.ip-list li code { font-family: "JetBrains Mono", monospace; font-size: 11px; }

.event-type {
  font-family: "JetBrains Mono", monospace; font-size: 11px;
  background: var(--bg-secondary); padding: 2px 6px; border-radius: 4px;
}

select {
  padding: 5px 8px; border-radius: 6px; border: 1px solid var(--border);
  background: var(--bg-secondary); color: var(--text-primary); font-size: 12px;
}

/* Modal */
.modal-backdrop {
  position: fixed; inset: 0; background: rgba(0,0,0,0.7);
  display: flex; align-items: center; justify-content: center;
  z-index: 1000; padding: 30px;
}
.modal-card {
  background: var(--bg-card); border: 1px solid var(--border);
  border-radius: 14px; padding: 24px 28px; max-width: 500px; width: 100%;
}
.modal-card h3 { margin: 0 0 12px; font-size: 17px; }
.modal-form { display: flex; flex-direction: column; gap: 14px; margin: 18px 0; }
.modal-form label {
  display: flex; flex-direction: column; gap: 6px;
  font-size: 12px; color: var(--text-secondary);
}
.modal-form label.checkbox { flex-direction: row; align-items: center; gap: 8px; }
.modal-form select {
  padding: 7px 10px; border-radius: 6px; border: 1px solid var(--border);
  background: var(--bg-secondary); color: var(--text-primary); font-size: 13px;
}
.modal-actions { display: flex; justify-content: flex-end; gap: 10px; }
</style>
