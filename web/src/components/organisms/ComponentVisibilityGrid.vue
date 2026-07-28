<script setup lang="ts">
import { errMsg } from "@/utils/errMsg";
import { computed, onMounted, ref, watch } from "vue";
import type { ComponentVisibilityEntry, RbacRole } from "@/types";
import { COMPONENT_REGISTRY, ROLES_ORDER, ROLE_RANK, type ComponentDef } from "@/rbac/componentRegistry";
import { rbacService } from "@/services/rbacService";
import { useGuildSelector } from "@/composables/useGuildSelector";
import { useToast } from "@/composables/useToast";
import { useComponentVisibility } from "@/composables/useComponentVisibility";

const { selectedGuildId } = useGuildSelector();
const { success, error: showError } = useToast();
const { reload } = useComponentVisibility();

// Etat local de la grille : Map[guild][key][role] -> bool (visible)
const matrix = ref<Record<string, Record<RbacRole, boolean>>>({});
const loading = ref(false);
const saving = ref(false);

const groupedByCategory = computed(() => {
  const groups = new Map<string, ComponentDef[]>();
  for (const c of COMPONENT_REGISTRY) {
    if (!groups.has(c.category)) groups.set(c.category, []);
    groups.get(c.category)!.push(c);
  }
  return Array.from(groups.entries());
});

function defaultVisibleFor(def: ComponentDef, role: RbacRole): boolean {
  return ROLE_RANK[role] >= ROLE_RANK[def.minRole];
}

async function load() {
  const gid = selectedGuildId.value;
  if (!gid) return;
  loading.value = true;
  try {
    const overrides = await rbacService.listComponentVisibility(gid);
    // Initialise depuis defauts puis applique overrides
    const m: Record<string, Record<RbacRole, boolean>> = {};
    for (const def of COMPONENT_REGISTRY) {
      m[def.key] = {} as Record<RbacRole, boolean>;
      for (const r of ROLES_ORDER) {
        m[def.key]![r] = defaultVisibleFor(def, r);
      }
    }
    for (const o of overrides) {
      if (m[o.component_key]) {
        m[o.component_key]![o.role] = o.visible;
      }
    }
    matrix.value = m;
  } catch (e) {
    showError(`Echec chargement visibilite : ${errMsg(e)}`);
  } finally {
    loading.value = false;
  }
}

async function save() {
  const gid = selectedGuildId.value;
  if (!gid) return;
  saving.value = true;
  try {
    const entries: ComponentVisibilityEntry[] = [];
    for (const def of COMPONENT_REGISTRY) {
      for (const r of ROLES_ORDER) {
        const cur = matrix.value[def.key]?.[r] ?? defaultVisibleFor(def, r);
        // On envoie tout — l'API upsert. Ca permet aussi de "reinitialiser"
        // un override en remettant la valeur par defaut.
        entries.push({
          component_key: def.key,
          role: r,
          visible: cur,
        });
      }
    }
    await rbacService.upsertComponentVisibility(gid, entries);
    success(`${entries.length} regles enregistrees.`);
    await reload();
  } catch (e) {
    showError(`Echec sauvegarde : ${errMsg(e)}`);
  } finally {
    saving.value = false;
  }
}

function resetToDefaults() {
  for (const def of COMPONENT_REGISTRY) {
    for (const r of ROLES_ORDER) {
      matrix.value[def.key]![r] = defaultVisibleFor(def, r);
    }
  }
}

function toggle(def: ComponentDef, role: RbacRole) {
  if (role === "owner") return; // Owner ne peut pas etre cache
  const cur = matrix.value[def.key]?.[role] ?? defaultVisibleFor(def, role);
  matrix.value[def.key]![role] = !cur;
}

function isOverride(def: ComponentDef, role: RbacRole): boolean {
  const cur = matrix.value[def.key]?.[role];
  if (cur === undefined) return false;
  return cur !== defaultVisibleFor(def, role);
}

onMounted(load);
watch(selectedGuildId, load);
</script>

<template>
  <section class="vis-section">
    <div class="vis-head">
      <div>
        <h3>🔐 Visibilité des composants par rôle</h3>
        <p class="muted">
          Coche pour afficher, décoche pour cacher. <strong>Owner</strong> voit toujours tout (modification verrouillée).
          <strong>Superadmin</strong> bypass cette grille. Les cases colorées sont des overrides du défaut.
        </p>
      </div>
      <div class="vis-actions">
        <button class="btn" :disabled="loading || saving" @click="resetToDefaults">Réinitialiser</button>
        <button class="btn primary" :disabled="saving" @click="save">
          {{ saving ? "Enregistrement…" : "💾 Enregistrer" }}
        </button>
      </div>
    </div>

    <div v-if="loading" class="muted">Chargement…</div>

    <template v-else>
      <div v-for="[cat, defs] in groupedByCategory" :key="cat" class="vis-group">
        <h4 class="vis-cat">{{ cat }}</h4>
        <table class="vis-table">
          <thead>
            <tr>
              <th class="comp-col">Composant</th>
              <th class="min-col">Min. défaut</th>
              <th v-for="r in ROLES_ORDER" :key="r" class="role-col">{{ r }}</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="def in defs" :key="def.key">
              <td class="comp-cell">
                <strong>{{ def.label }}</strong>
                <code class="key-tag">{{ def.key }}</code>
              </td>
              <td class="muted small center">{{ def.minRole }}</td>
              <td
                v-for="r in ROLES_ORDER"
                :key="r"
                class="cell"
                :data-role="r"
                :class="{
                  on: matrix[def.key]?.[r],
                  off: !matrix[def.key]?.[r],
                  override: isOverride(def, r),
                  locked: r === 'owner',
                }"
                @click="toggle(def, r)"
                :title="r === 'owner' ? 'Owner toujours visible' : (matrix[def.key]?.[r] ? 'Visible — clic pour cacher' : 'Caché — clic pour afficher')"
              >
                <span class="cell-role-label">{{ r }}</span>
                <span class="cell-icon">{{ matrix[def.key]?.[r] ? "✓" : "✗" }}</span>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </template>
  </section>
</template>

<style scoped>
.vis-section {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 12px;
  padding: 18px 20px;
  margin-top: 20px;
}
.vis-head {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  flex-wrap: wrap;
  gap: 14px;
  margin-bottom: 16px;
}
.vis-head h3 { margin: 0 0 4px; font-size: 15px; }
.muted { color: var(--text-secondary); font-size: 12px; margin: 0; }
.vis-actions { display: flex; gap: 8px; }

.vis-group { margin-bottom: 18px; }
.vis-cat {
  margin: 0 0 8px;
  font-size: 12px;
  font-weight: 700;
  text-transform: uppercase;
  color: var(--accent);
  letter-spacing: 0.5px;
}
.vis-table {
  width: 100%;
  border-collapse: collapse;
  font-size: 12px;
}
.vis-table th, .vis-table td {
  padding: 6px 10px;
  border-bottom: 1px solid color-mix(in srgb, var(--border) 60%, transparent);
}
.vis-table th {
  font-size: 10px;
  text-transform: uppercase;
  color: var(--text-secondary);
  letter-spacing: 0.4px;
}
.comp-col { width: 50%; }
.role-col, .min-col { text-align: center; width: 80px; }
.comp-cell strong { display: block; }
.key-tag {
  display: inline-block;
  margin-top: 2px;
  font-size: 10px;
  color: var(--text-secondary);
  font-family: "JetBrains Mono", monospace;
}
.center { text-align: center; }
.small { font-size: 11px; }

.cell {
  text-align: center;
  cursor: pointer;
  font-weight: 700;
  user-select: none;
  transition: background 0.15s ease;
}
.cell.on { background: color-mix(in srgb, var(--success, #2ecc71) 14%, transparent); color: var(--success, #2ecc71); }
.cell.off { background: color-mix(in srgb, var(--danger) 12%, transparent); color: var(--danger); }
.cell.override { box-shadow: inset 0 0 0 2px var(--accent); }
.cell.locked { cursor: not-allowed; opacity: 0.6; }
.cell:hover:not(.locked) { filter: brightness(1.2); }

.btn {
  padding: 8px 14px;
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
.btn:disabled { opacity: 0.5; cursor: not-allowed; }
.btn.primary {
  background: var(--accent);
  color: white;
  border-color: var(--accent);
}
.btn.primary:hover:not(:disabled) { filter: brightness(1.1); color: white; }

/* Label rôle visible uniquement en mobile (desktop = colonnes thead) */
.cell-role-label { display: none; }

@media (max-width: 768px) {
  /* Convertit la table en cards verticales */
  .vis-table thead { display: none; }
  .vis-table,
  .vis-table tbody,
  .vis-table tr {
    display: block;
    width: 100%;
  }
  .vis-table tr {
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 10px 12px;
    margin-bottom: 8px;
  }
  .vis-table td {
    display: block;
    border-bottom: none;
    padding: 4px 0;
  }
  .comp-cell {
    margin-bottom: 8px;
  }
  /* Les 4 cells de rôle alignées en grid 2x2 */
  .cell {
    display: flex !important;
    align-items: center;
    justify-content: space-between;
    padding: 8px 10px !important;
    border-radius: 6px;
    text-align: left;
  }
  .cell-role-label {
    display: inline-block;
    text-transform: uppercase;
    font-size: 11px;
    letter-spacing: 0.5px;
    font-weight: 700;
  }
  .cell-icon {
    font-size: 14px;
  }
  /* Espacement entre les rôles */
  .cell + .cell {
    margin-top: 4px;
  }
  .min-col,
  .role-col {
    width: auto;
    text-align: left;
  }
  .vis-head {
    flex-direction: column;
  }
  .vis-actions {
    width: 100%;
  }
  .vis-actions .btn {
    flex: 1;
  }
}
</style>
