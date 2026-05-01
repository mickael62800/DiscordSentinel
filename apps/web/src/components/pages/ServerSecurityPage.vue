<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import {
  serverSecurityService,
  type AuditEntry,
  type AuthFailureEntry,
  type BannedIpsResponse,
  type SecurityWindow,
  type TlsCertInfo,
  type TopIpEntry,
} from "@/services/serverSecurityService";
import { useGuildSelector } from "@/composables/useGuildSelector";
import { useToast } from "@/composables/useToast";

const { error: showError } = useToast();
const { selectedGuildId } = useGuildSelector();

const refreshing = ref(false);
const cleaning = ref(false);
const cleanupOpts = ref({ days: 0, includeAudit: false });
const showCleanupModal = ref(false);

// Sections data
const topIps = ref<TopIpEntry[]>([]);
const topIpsWindow = ref<SecurityWindow>("1h");
const authFailures = ref<AuthFailureEntry[]>([]);
const authWindow = ref<SecurityWindow>("24h");
const banned = ref<BannedIpsResponse | null>(null);
const auditLogs = ref<AuditEntry[]>([]);
const auditFilter = ref<"all" | "rbac" | "docker" | "moderation" | "ban">("all");
const tls = ref<TlsCertInfo | null>(null);
const tlsError = ref<string | null>(null);

// ── Loaders ──
async function loadTopIps() {
  try {
    topIps.value = await serverSecurityService.topIps(topIpsWindow.value, 20);
  } catch (e: any) {
    showError(`Top IPs : ${e?.message ?? e}`);
  }
}
async function loadAuthFailures() {
  try {
    authFailures.value = await serverSecurityService.authFailures(authWindow.value, 100);
  } catch (e: any) {
    showError(`Echecs auth : ${e?.message ?? e}`);
  }
}
async function loadBanned() {
  try {
    banned.value = await serverSecurityService.bannedIps();
  } catch (e: any) {
    showError(`Bans : ${e?.message ?? e}`);
  }
}
async function loadAudit() {
  try {
    const prefix = auditFilter.value === "all" ? undefined : auditFilter.value;
    auditLogs.value = await serverSecurityService.auditLogs({
      guild_id: selectedGuildId.value ?? undefined,
      event_type_prefix: prefix,
      limit: 100,
    });
  } catch (e: any) {
    showError(`Audit logs : ${e?.message ?? e}`);
  }
}
async function loadTls() {
  tlsError.value = null;
  try {
    tls.value = await serverSecurityService.tlsCert();
  } catch (e: any) {
    tlsError.value = e?.message ?? String(e);
    tls.value = null;
  }
}

async function refreshAll() {
  refreshing.value = true;
  await Promise.allSettled([loadTopIps(), loadAuthFailures(), loadBanned(), loadAudit(), loadTls()]);
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
    alert(`✅ ${r.message}`);
  } catch (e: any) {
    showError(`Echec cleanup : ${e?.message ?? e}`);
  } finally {
    cleaning.value = false;
  }
}

onMounted(refreshAll);

// ── Helpers ──
function fmtDate(s: string): string {
  return new Date(s).toLocaleString("fr-FR");
}
function truncate(s: string, n: number): string {
  return s.length > n ? s.slice(0, n) + "…" : s;
}

const tlsBadgeClass = computed(() => {
  if (!tls.value) return "";
  if (tls.value.is_expired) return "danger";
  if (tls.value.is_warning) return "warning";
  return "ok";
});
</script>

<template>
  <div class="security-page">
    <div class="page-header">
      <div>
        <h1>🛡️ Sécurité serveur</h1>
        <p class="muted small">
          Surveillance des attaques, échecs d'authentification, IPs bannies et
          audit des actions administratives.
        </p>
      </div>
      <div class="header-actions">
        <button class="btn danger" :disabled="cleaning" @click="showCleanupModal = true">
          🗑 Tout nettoyer
        </button>
        <button class="btn primary" :disabled="refreshing" @click="refreshAll">
          {{ refreshing ? "Actualisation…" : "↻ Actualiser tout" }}
        </button>
      </div>
    </div>

    <!-- Modale confirmation cleanup -->
    <div v-if="showCleanupModal" class="modal-backdrop" @click.self="showCleanupModal = false">
      <div class="modal-card">
        <h3>🗑 Nettoyer les logs de sécurité</h3>
        <p class="muted">
          Supprime les entrées de logs API (Top IPs / Échecs auth) selon le critère choisi.
          Optionnellement aussi les audit logs Discord.
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

    <!-- ── 1. Top IPs ── -->
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
          <tr><th>IP</th><th class="num">Total</th><th class="num">4xx/5xx</th><th>Dernier</th></tr>
        </thead>
        <tbody>
          <tr v-for="ip in topIps" :key="ip.client_ip" :class="{ alert: ip.failed > 10 }">
            <td><code>{{ ip.client_ip }}</code></td>
            <td class="num">{{ ip.total }}</td>
            <td class="num" :class="{ danger: ip.failed > 10 }">{{ ip.failed }}</td>
            <td class="muted small">{{ fmtDate(ip.last_seen) }}</td>
          </tr>
        </tbody>
      </table>
      <div v-else class="empty">Aucune requête sur la fenêtre sélectionnée.</div>
    </section>

    <!-- ── 2. Échecs d'auth ── -->
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
            <th>Quand</th>
            <th>Code</th>
            <th>Méthode</th>
            <th>Route</th>
            <th>IP</th>
            <th>User-Agent</th>
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
          </tr>
        </tbody>
      </table>
      <div v-else class="empty">Aucun échec d'authentification 🎉</div>
    </section>

    <!-- ── 3. IPs bannies ── -->
    <section class="card">
      <h2>🚫 IPs bannies (fail2ban)</h2>
      <div v-if="banned && !banned.installed" class="info">
        <p>{{ banned.message }}</p>
        <details class="hint">
          <summary>Setup rapide fail2ban</summary>
          <pre>sudo apt install fail2ban
sudo systemctl enable --now fail2ban

# Jail nginx (rate limit)
sudo tee /etc/fail2ban/jail.local &lt;&lt;EOF
[sshd]
enabled = true
maxretry = 3
bantime = 1h

[nginx-http-auth]
enabled = true
maxretry = 5
bantime = 1h
EOF

sudo systemctl restart fail2ban
sudo fail2ban-client status</pre>
        </details>
      </div>
      <div v-else-if="banned && banned.bans.length > 0">
        <ul>
          <li v-for="ip in banned.bans" :key="ip"><code>{{ ip }}</code></li>
        </ul>
      </div>
      <div v-else class="empty">Aucune IP actuellement bannie.</div>
    </section>

    <!-- ── 4. Audit log admin ── -->
    <section class="card">
      <div class="card-head">
        <h2>📋 Audit log ({{ auditLogs.length }})</h2>
        <div class="card-actions">
          <select v-model="auditFilter" @change="loadAudit">
            <option value="all">Tous événements</option>
            <option value="rbac">RBAC</option>
            <option value="docker">Docker admin</option>
            <option value="moderation">Modération</option>
            <option value="ban">Bans</option>
          </select>
          <button class="btn xs" @click="loadAudit">↻</button>
        </div>
      </div>
      <table v-if="auditLogs.length > 0" class="data-table">
        <thead>
          <tr><th>Quand</th><th>Événement</th><th>Acteur</th><th>Cible</th></tr>
        </thead>
        <tbody>
          <tr v-for="e in auditLogs" :key="e.id">
            <td class="small muted">{{ fmtDate(e.created_at) }}</td>
            <td><code class="event-type">{{ e.event_type }}</code></td>
            <td class="small">
              <span v-if="e.actor_name">{{ e.actor_name }}</span>
              <code v-if="e.actor_id" class="muted small">{{ e.actor_id }}</code>
              <span v-if="!e.actor_id && !e.actor_name" class="muted">—</span>
            </td>
            <td class="small">
              <span v-if="e.target_name">{{ e.target_name }}</span>
              <code v-if="e.target_id" class="muted small">{{ e.target_id }}</code>
              <span v-if="!e.target_id && !e.target_name" class="muted">—</span>
            </td>
          </tr>
        </tbody>
      </table>
      <div v-else class="empty">Aucun événement d'audit pour ce filtre.</div>
    </section>

    <!-- ── 5. Certificat TLS ── -->
    <section class="card">
      <h2>🔐 Certificat TLS</h2>
      <div v-if="tlsError" class="error">
        Erreur lecture cert : <code>{{ tlsError }}</code>
        <p class="small muted">
          Vérifie que le volume <code>letsencrypt_etc</code> est bien monté en RO sur l'API.
        </p>
      </div>
      <div v-else-if="tls" class="tls-info">
        <div class="tls-row">
          <span class="lbl">Domaine</span>
          <strong>{{ tls.domain }}</strong>
        </div>
        <div class="tls-row">
          <span class="lbl">Émetteur</span>
          <span class="small">{{ tls.issuer }}</span>
        </div>
        <div class="tls-row">
          <span class="lbl">Valide depuis</span>
          <span class="small">{{ tls.not_before }}</span>
        </div>
        <div class="tls-row">
          <span class="lbl">Expire le</span>
          <span class="small">{{ tls.not_after }}</span>
        </div>
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
  </div>
</template>

<style scoped>
.security-page { padding: 16px; }
.page-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  flex-wrap: wrap;
  gap: 16px;
  margin-bottom: 20px;
}
.page-header h1 { margin: 0 0 4px; font-size: 1.6rem; }
.muted { color: var(--text-secondary); margin: 0; }
.muted.small { font-size: 12px; }
.small { font-size: 11px; }
.mono { font-family: "JetBrains Mono", monospace; }

.card {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 12px;
  padding: 18px 20px;
  margin-bottom: 16px;
}
.card h2 { margin: 0 0 12px; font-size: 16px; }
.card-head {
  display: flex;
  justify-content: space-between;
  align-items: center;
  flex-wrap: wrap;
  gap: 10px;
  margin-bottom: 12px;
}
.card-head h2 { margin: 0; }
.card-actions { display: flex; gap: 8px; align-items: center; }

.btn {
  padding: 7px 14px;
  border-radius: 8px;
  border: 1px solid var(--border);
  background: var(--bg-secondary);
  color: var(--text-primary);
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
}
.btn:hover:not(:disabled) { border-color: var(--accent); color: var(--accent); }
.btn:disabled { opacity: 0.5; cursor: not-allowed; }
.btn.primary { background: var(--accent); color: white; border-color: var(--accent); }
.btn.primary:hover:not(:disabled) { filter: brightness(1.1); color: white; }
.btn.danger { border-color: color-mix(in srgb, var(--danger) 50%, var(--border)); color: var(--danger); }
.btn.danger:hover:not(:disabled) { background: color-mix(in srgb, var(--danger) 15%, var(--bg-secondary)); }

.header-actions { display: flex; gap: 8px; align-items: center; }

.modal-backdrop {
  position: fixed;
  inset: 0;
  background: rgba(0,0,0,0.7);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
  padding: 30px;
}
.modal-card {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 14px;
  padding: 24px 28px;
  max-width: 500px;
  width: 100%;
}
.modal-card h3 { margin: 0 0 12px; font-size: 17px; }
.modal-form {
  display: flex;
  flex-direction: column;
  gap: 14px;
  margin: 18px 0;
}
.modal-form label {
  display: flex;
  flex-direction: column;
  gap: 6px;
  font-size: 12px;
  color: var(--text-secondary);
}
.modal-form label.checkbox {
  flex-direction: row;
  align-items: center;
  gap: 8px;
}
.modal-form select {
  padding: 7px 10px;
  border-radius: 6px;
  border: 1px solid var(--border);
  background: var(--bg-secondary);
  color: var(--text-primary);
  font-size: 13px;
}
.modal-actions {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
}
.btn.xs { padding: 3px 8px; font-size: 11px; }

select {
  padding: 5px 8px;
  border-radius: 6px;
  border: 1px solid var(--border);
  background: var(--bg-secondary);
  color: var(--text-primary);
  font-size: 12px;
}

.data-table {
  width: 100%;
  border-collapse: collapse;
  font-size: 12px;
}
.data-table th, .data-table td {
  padding: 8px 10px;
  text-align: left;
  border-bottom: 1px solid color-mix(in srgb, var(--border) 60%, transparent);
}
.data-table th {
  font-size: 10px;
  text-transform: uppercase;
  color: var(--text-secondary);
  letter-spacing: 0.5px;
}
.data-table .num { text-align: right; }
.data-table .danger { color: var(--danger); font-weight: 700; }
.data-table tr.alert { background: color-mix(in srgb, var(--danger) 6%, transparent); }
.data-table .ua { max-width: 350px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }

.code-pill {
  display: inline-block;
  padding: 2px 8px;
  border-radius: 6px;
  font-size: 10px;
  font-family: "JetBrains Mono", monospace;
  font-weight: 700;
  background: var(--bg-secondary);
  color: var(--text-secondary);
}
.code-pill.warn { background: color-mix(in srgb, var(--warning, #e67e22) 18%, var(--bg-secondary)); color: var(--warning, #e67e22); }
.code-pill.danger { background: color-mix(in srgb, var(--danger) 18%, var(--bg-secondary)); color: var(--danger); }

.event-type {
  font-family: "JetBrains Mono", monospace;
  font-size: 11px;
  background: var(--bg-secondary);
  padding: 2px 6px;
  border-radius: 4px;
}

.empty {
  padding: 20px;
  text-align: center;
  color: var(--text-secondary);
  font-size: 12px;
  font-style: italic;
}

.info {
  padding: 14px;
  background: color-mix(in srgb, var(--accent) 6%, var(--bg-secondary));
  border-left: 3px solid var(--accent);
  border-radius: 4px;
}
.info p { margin: 0 0 8px; font-size: 13px; }
.hint summary { cursor: pointer; font-size: 12px; color: var(--text-secondary); }
.hint pre {
  margin-top: 8px;
  padding: 12px;
  background: #0e1116;
  color: #d4d4d8;
  border-radius: 6px;
  font-size: 11px;
  overflow-x: auto;
}

.error {
  padding: 12px;
  background: color-mix(in srgb, var(--danger) 8%, transparent);
  border-left: 3px solid var(--danger);
  border-radius: 4px;
  font-size: 13px;
}
.error code { background: rgba(0,0,0,0.2); padding: 1px 6px; border-radius: 3px; }

.tls-info { display: flex; flex-direction: column; gap: 10px; }
.tls-row {
  display: grid;
  grid-template-columns: 130px 1fr;
  gap: 12px;
  align-items: center;
}
.tls-row .lbl { color: var(--text-secondary); font-size: 11px; text-transform: uppercase; letter-spacing: 0.4px; }
.badge {
  display: inline-block;
  padding: 4px 10px;
  border-radius: 12px;
  font-size: 12px;
  font-weight: 600;
}
.badge.ok { background: color-mix(in srgb, var(--success, #2ecc71) 18%, transparent); color: var(--success, #2ecc71); }
.badge.warning { background: color-mix(in srgb, var(--warning, #e67e22) 18%, transparent); color: var(--warning, #e67e22); }
.badge.danger { background: color-mix(in srgb, var(--danger) 18%, transparent); color: var(--danger); }
</style>
