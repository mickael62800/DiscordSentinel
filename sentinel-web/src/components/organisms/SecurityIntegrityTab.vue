<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import {
  serverSecurityService,
  type ContainerChangesResponse,
  type FileIntegrityResponse,
  type TlsCertInfo,
  type TrivyResponse,
} from "@/services/serverSecurityService";
import { useFormatDate } from "@/composables/useFormatDate";

const { formatDateTimeShort: fmtDate } = useFormatDate();

const tls = ref<TlsCertInfo | null>(null);
const tlsError = ref<string | null>(null);
const trivy = ref<TrivyResponse | null>(null);
const trivyError = ref<string | null>(null);
const integrity = ref<FileIntegrityResponse | null>(null);
const integrityError = ref<string | null>(null);
const containers = ref<ContainerChangesResponse | null>(null);
const containersError = ref<string | null>(null);

async function loadTls() {
  tlsError.value = null;
  try { tls.value = await serverSecurityService.tlsCert(); }
  catch (e: any) { tlsError.value = e?.message ?? String(e); tls.value = null; }
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
async function loadContainers() {
  containersError.value = null;
  try { containers.value = await serverSecurityService.containerChanges(); }
  catch (e: any) { containersError.value = e?.message ?? String(e); containers.value = null; }
}

const tlsBadgeClass = computed(() => {
  if (!tls.value) return "";
  if (tls.value.is_expired) return "danger";
  if (tls.value.is_warning) return "warning";
  return "ok";
});

async function refresh() {
  await Promise.allSettled([loadTls(), loadTrivy(), loadIntegrity(), loadContainers()]);
}

defineExpose({ refresh });

function truncate(s: string, n: number): string { return s.length > n ? s.slice(0, n) + "…" : s; }

onMounted(refresh);
</script>

<template>
  <div class="tab-content">
    <!-- Cert TLS -->
    <section class="card">
      <h2>🔐 Certificat TLS</h2>
      <div v-if="tlsError" class="error-banner">
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
      <div v-else class="empty-state">Chargement…</div>
    </section>

    <!-- Container changes -->
    <section class="card">
      <div class="card-head">
        <h2>🐳 Conteneurs surveillés</h2>
        <button class="btn-secondary xs" @click="loadContainers">↻</button>
      </div>
      <div v-if="containersError" class="info-banner">
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
        <button class="btn-secondary xs" @click="loadTrivy">↻</button>
      </div>
      <div v-if="trivyError" class="info-banner">
        <p class="small">{{ trivyError }}</p>
        <p class="hint small">Setup : <code>sudo bash sentinel-infrastructure/scripts/setup-host-security.sh trivy</code></p>
      </div>
      <div v-else-if="trivy">
        <p class="muted small">Maj {{ fmtDate(trivy.updated_at) }} · Scan automatique 1×/jour à 3h</p>
        <div class="cve-summary">
          <div class="cve-stat danger"><span class="lbl">Critical</span><strong>{{ trivy.critical }}</strong></div>
          <div class="cve-stat warning"><span class="lbl">High</span><strong>{{ trivy.high }}</strong></div>
          <div class="cve-stat"><span class="lbl">Medium</span><strong>{{ trivy.medium }}</strong></div>
          <div class="cve-stat ok"><span class="lbl">Low</span><strong>{{ trivy.low }}</strong></div>
        </div>
        <table v-if="trivy.vulnerabilities.length > 0" class="data-table">
          <thead><tr><th>Image</th><th>CVE</th><th>Sévérité</th><th>Package</th><th>Fix</th></tr></thead>
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
        <div v-else class="empty-state">✅ Aucune vulnérabilité critique/high détectée.</div>
      </div>
    </section>

    <!-- Intégrité fichiers -->
    <section class="card">
      <div class="card-head">
        <h2>📁 Intégrité fichiers critiques ({{ integrity?.files.length ?? 0 }})</h2>
        <button class="btn-secondary xs" @click="loadIntegrity">↻</button>
      </div>
      <div v-if="integrityError" class="info-banner">
        <p class="small">{{ integrityError }}</p>
        <p class="hint small">Setup : <code>sudo bash sentinel-infrastructure/scripts/setup-host-security.sh file-integrity</code></p>
      </div>
      <div v-else-if="integrity">
        <p class="muted small">
          Maj {{ fmtDate(integrity.updated_at) }} ·
          <strong v-if="integrity.modified_count > 0" class="alert">⚠️ {{ integrity.modified_count }} fichier(s) modifié(s)</strong>
          <span v-else>✅ Tous les fichiers correspondent au baseline</span>
        </p>
        <table v-if="integrity.files.length > 0" class="data-table">
          <thead><tr><th>Chemin</th><th>SHA256</th><th>Modifié</th><th>Statut</th></tr></thead>
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

.tls-info { display: flex; flex-direction: column; gap: 10px; }
.tls-row {
  display: grid;
  grid-template-columns: 130px 1fr;
  gap: 12px;
  align-items: center;
}
.tls-row .lbl {
  color: var(--text-secondary);
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 0.4px;
}
</style>
