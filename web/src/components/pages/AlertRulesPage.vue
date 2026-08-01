<script setup lang="ts">
import { onMounted, ref } from "vue";
import AdminPageShell from "../layouts/AdminPageShell.vue";
import AppButton from "../atoms/AppButton.vue";
import { alertRulesService, type AlertRule } from "@/services/alertRulesService";
import { useToast } from "@/composables/useToast";

const { success, error: showError } = useToast();

const rules = ref<AlertRule[]>([]);
const loading = ref(false);
const savingId = ref<string | null>(null);
const loadError = ref<string | null>(null);

const SEVERITIES = ["info", "warning", "critical"];

// Métriques booléennes : pas de seuil numérique à éditer.
function hasThreshold(r: AlertRule): boolean {
  return r.metric !== "service_offline" && r.metric !== "container_removed";
}

function unitFor(metric: string): string {
  if (metric.endsWith("_percent")) return "%";
  if (metric === "tls_expiry_days") return "jours";
  if (metric === "auth_failures_1h") return "/h";
  return "";
}

async function load() {
  loading.value = true;
  loadError.value = null;
  try {
    rules.value = await alertRulesService.list();
  } catch (e) {
    loadError.value = e instanceof Error ? e.message : "Erreur de chargement.";
  } finally {
    loading.value = false;
  }
}

async function save(r: AlertRule) {
  savingId.value = r.id;
  try {
    const updated = await alertRulesService.update(r.id, {
      enabled: r.enabled,
      threshold: hasThreshold(r) && r.threshold != null ? r.threshold : undefined,
      severity: r.severity,
      cooldown_secs: r.cooldown_secs,
    });
    const idx = rules.value.findIndex((x) => x.id === updated.id);
    if (idx >= 0) rules.value[idx] = updated;
    success(`Règle « ${r.label} » enregistrée.`);
  } catch (e) {
    showError(e instanceof Error ? e.message : "Échec de l'enregistrement.");
  } finally {
    savingId.value = null;
  }
}

onMounted(load);
</script>

<template>
  <AdminPageShell title="Règles d'alerte" icon="🔔">
    <template #lede>
      Supervision serveur : seuils déclenchant une alerte Discord (webhook). CPU,
      RAM, disque, services offline, échecs d'auth, certificat TLS, conteneurs.
    </template>
    <template #actions>
      <AppButton variant="secondary" @click="load">↻ Rafraîchir</AppButton>
    </template>

    <div v-if="loading" class="muted">Chargement…</div>
    <div v-else-if="loadError" class="error-box">{{ loadError }}</div>
    <div v-else-if="rules.length === 0" class="muted">Aucune règle.</div>

    <table v-else class="rules-table">
      <thead>
        <tr>
          <th>Alerte</th>
          <th>Active</th>
          <th>Seuil</th>
          <th>Sévérité</th>
          <th>Cooldown (s)</th>
          <th></th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="r in rules" :key="r.id" :class="{ disabled: !r.enabled }">
          <td>
            <strong>{{ r.label }}</strong>
            <div class="metric-hint">
              {{ r.metric }} <span v-if="hasThreshold(r)">({{ r.comparator === "lt" ? "<" : ">" }})</span>
            </div>
          </td>
          <td>
            <input v-model="r.enabled" type="checkbox" />
          </td>
          <td>
            <template v-if="hasThreshold(r)">
              <input
                v-model.number="r.threshold"
                type="number"
                class="num-input"
                min="0"
              />
              <span class="unit">{{ unitFor(r.metric) }}</span>
            </template>
            <span v-else class="muted">—</span>
          </td>
          <td>
            <select v-model="r.severity" class="sev-select">
              <option v-for="s in SEVERITIES" :key="s" :value="s">{{ s }}</option>
            </select>
          </td>
          <td>
            <input v-model.number="r.cooldown_secs" type="number" class="num-input" min="60" />
          </td>
          <td>
            <AppButton
              variant="primary"
              :disabled="savingId === r.id"
              @click="save(r)"
            >
              {{ savingId === r.id ? "…" : "Enregistrer" }}
            </AppButton>
          </td>
        </tr>
      </tbody>
    </table>

    <p class="muted small footer-hint">
      L'alerting nécessite la variable <code>SECURITY_ALERTS_WEBHOOK</code> côté API.
      La déduplication respecte le cooldown de chaque règle.
    </p>
  </AdminPageShell>
</template>

<style scoped>
@import "./_admin-page-shared.css";

.rules-table {
  width: 100%;
  border-collapse: collapse;
  margin-top: 8px;
}
.rules-table th,
.rules-table td {
  text-align: left;
  padding: 10px 12px;
  border-bottom: 1px solid var(--border);
  vertical-align: middle;
}
.rules-table th {
  font-size: 12px;
  text-transform: uppercase;
  letter-spacing: 0.4px;
  color: var(--text-secondary);
}
.rules-table tr.disabled {
  opacity: 0.55;
}
.metric-hint {
  font-size: 11px;
  color: var(--text-secondary);
  margin-top: 2px;
}
.num-input {
  width: 90px;
  padding: 6px 8px;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: var(--bg-primary);
  color: var(--text-primary);
}
.sev-select {
  padding: 6px 8px;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: var(--bg-primary);
  color: var(--text-primary);
}
.unit {
  margin-left: 4px;
  color: var(--text-secondary);
  font-size: 12px;
}
.error-box {
  color: var(--danger);
  padding: 10px;
  border: 1px solid var(--danger);
  border-radius: var(--radius-md);
}
.footer-hint {
  margin-top: 16px;
}
</style>
