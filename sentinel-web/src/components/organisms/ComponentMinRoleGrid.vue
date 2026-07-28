<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { errMsg } from "@/utils/errMsg";
import { rbacService, type ComponentMinRoleEntry } from "@/services/rbacService";
import { useGuildSelector } from "@/composables/useGuildSelector";
import { useToast } from "@/composables/useToast";
import { useComponentVisibility } from "@/composables/useComponentVisibility";
import type { RbacRole } from "@/types";
import { ROLE_RANK } from "@/rbac/componentRegistry";

const ROLES: RbacRole[] = ["viewer", "moderator", "admin", "owner"];

const { selectedGuildId } = useGuildSelector();
const { success, error: toastError } = useToast();
const { reload } = useComponentVisibility();

const entries = ref<ComponentMinRoleEntry[]>([]);
const loading = ref(false);
const saving = ref<string | null>(null);

// Libelles humains pour chaque cle (mappage local — alternative a un appel
// extra). Les cles inconnues retombent sur la cle elle-meme.
const LABELS: Record<string, string> = {
  "db.purge.audit_logs": "Purger les audit logs",
  "db.purge.security_events": "Purger les events sécurité",
  "db.purge.voice_history": "Purger l'historique vocal",
  "db.purge.voice_channel": "Purger un salon vocal archivé",
  "db.purge.blackjack": "Purger les données blackjack",
  "db.reset.wallets": "Reset bulk des wallets",
};

const sortedEntries = computed(() =>
  [...entries.value].sort((a, b) => a.component_key.localeCompare(b.component_key)),
);

async function load() {
  if (!selectedGuildId.value) return;
  loading.value = true;
  try {
    entries.value = await rbacService.listComponentMinRoles(selectedGuildId.value);
  } catch (e) {
    toastError(`Chargement échoué: ${errMsg(e)}`);
  } finally {
    loading.value = false;
  }
}

function rolesAvailableForKey(e: ComponentMinRoleEntry): RbacRole[] {
  // Le user ne peut pas descendre sous le floor.
  return ROLES.filter((r) => ROLE_RANK[r] >= ROLE_RANK[e.floor_role]);
}

async function setRole(e: ComponentMinRoleEntry, newRole: RbacRole) {
  if (!selectedGuildId.value) return;
  if (newRole === e.effective_role) return;
  saving.value = e.component_key;
  try {
    await rbacService.upsertComponentMinRole(
      selectedGuildId.value,
      e.component_key,
      newRole,
    );
    success(`${LABELS[e.component_key] ?? e.component_key} → ${newRole}`);
    await load();
    await reload();
  } catch (err) {
    toastError(`Sauvegarde échouée: ${errMsg(err)}`);
  } finally {
    saving.value = null;
  }
}

async function resetToDefault(e: ComponentMinRoleEntry) {
  if (!selectedGuildId.value) return;
  if (!e.override_role) return;
  saving.value = e.component_key;
  try {
    await rbacService.deleteComponentMinRole(selectedGuildId.value, e.component_key);
    success(`Reset → ${e.default_role} (défaut)`);
    await load();
    await reload();
  } catch (err) {
    toastError(`Reset échoué: ${errMsg(err)}`);
  } finally {
    saving.value = null;
  }
}

onMounted(load);
</script>

<template>
  <section class="min-role-grid">
    <header class="header">
      <h2>Permissions sensibles</h2>
      <p class="hint">
        Pour chaque action destructive, choisis le rôle minimum requis. Le
        <strong>floor</strong> est la borne plancher non-contournable (sécurité).
        Les changements s'appliquent immédiatement, côté UI et API.
      </p>
    </header>

    <div v-if="loading" class="empty">Chargement…</div>
    <div v-else-if="entries.length === 0" class="empty">
      Aucun gate configurable pour cette guild.
    </div>

    <table v-else class="grid-table">
      <thead>
        <tr>
          <th>Action</th>
          <th>Rôle requis</th>
          <th>Floor</th>
          <th>État</th>
          <th></th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="e in sortedEntries" :key="e.component_key">
          <td>
            <div class="row-label">{{ LABELS[e.component_key] ?? e.component_key }}</div>
            <code class="row-key">{{ e.component_key }}</code>
          </td>
          <td>
            <select
              :value="e.effective_role"
              :disabled="saving === e.component_key"
              class="role-select"
              @change="setRole(e, ($event.target as HTMLSelectElement).value as RbacRole)"
            >
              <option
                v-for="r in rolesAvailableForKey(e)"
                :key="r"
                :value="r"
              >
                {{ r }}
              </option>
            </select>
          </td>
          <td><span class="floor-badge">≥ {{ e.floor_role }}</span></td>
          <td>
            <span v-if="e.override_role" class="badge override">override</span>
            <span v-else class="badge default">défaut ({{ e.default_role }})</span>
          </td>
          <td>
            <button
              v-if="e.override_role"
              class="btn-reset"
              :disabled="saving === e.component_key"
              title="Revenir au rôle par défaut"
              @click="resetToDefault(e)"
            >
              ↺ Reset
            </button>
          </td>
        </tr>
      </tbody>
    </table>
  </section>
</template>

<style scoped>
.min-role-grid {
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg, 12px);
  padding: var(--space-lg, 16px);
  margin-top: var(--space-lg, 16px);
}

.header {
  margin-bottom: var(--space-md, 12px);
}

.header h2 {
  margin: 0 0 4px 0;
  font-size: 1.05rem;
  color: var(--text-primary);
}

.hint {
  margin: 0;
  font-size: 12px;
  color: var(--text-secondary);
  line-height: 1.4;
}

.empty {
  padding: var(--space-lg);
  text-align: center;
  color: var(--text-secondary);
  font-size: 13px;
}

.grid-table {
  width: 100%;
  border-collapse: collapse;
  font-size: 13px;
}

.grid-table th {
  text-align: left;
  padding: 8px 10px;
  font-weight: 600;
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  color: var(--text-secondary);
  border-bottom: 1px solid var(--border);
}

.grid-table td {
  padding: 10px;
  border-bottom: 1px solid color-mix(in srgb, var(--border) 50%, transparent);
  vertical-align: middle;
}

.row-label {
  font-weight: 500;
  color: var(--text-primary);
}

.row-key {
  display: block;
  font-size: 11px;
  color: var(--text-secondary);
  font-family: monospace;
  margin-top: 2px;
}

.role-select {
  padding: 6px 10px;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm, 6px);
  background: var(--bg-card);
  color: var(--text-primary);
  font-size: 13px;
  cursor: pointer;
  min-width: 110px;
}

.role-select:focus {
  outline: none;
  border-color: var(--accent);
}

.role-select:disabled {
  opacity: 0.5;
}

.floor-badge {
  display: inline-block;
  padding: 2px 8px;
  background: var(--muted-bg, rgba(148, 149, 176, 0.15));
  color: var(--text-secondary);
  border-radius: 999px;
  font-size: 11px;
  font-family: monospace;
}

.badge {
  display: inline-block;
  padding: 2px 8px;
  border-radius: 999px;
  font-size: 11px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.04em;
}

.badge.default {
  background: var(--muted-bg, rgba(148, 149, 176, 0.15));
  color: var(--text-secondary);
}

.badge.override {
  background: var(--warning-bg, rgba(254, 231, 92, 0.15));
  color: var(--warning, #fee75c);
}

.btn-reset {
  padding: 4px 10px;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm, 6px);
  background: transparent;
  color: var(--text-secondary);
  font-size: 12px;
  cursor: pointer;
  transition: all 0.12s;
}

.btn-reset:hover:not(:disabled) {
  border-color: var(--accent);
  color: var(--accent);
}

.btn-reset:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

@media (max-width: 640px) {
  /* Convertit la table en cards verticales : 1 row = 1 card empilee. */
  .grid-table thead {
    display: none;
  }
  .grid-table,
  .grid-table tbody,
  .grid-table tr {
    display: block;
    width: 100%;
  }
  .grid-table tr {
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: 8px;
    margin-bottom: 8px;
    padding: 10px 12px;
  }
  .grid-table td {
    display: block;
    padding: 4px 0;
    border-bottom: none;
  }
  .role-select {
    min-width: 0;
    width: 100%;
  }
  .row-label {
    font-size: 13px;
  }
  .row-key {
    font-size: 10px;
  }
}
</style>
