<script setup lang="ts">
/**
 * Éditeur d'accès d'un salon du plan : qui voit quoi, et à quel titre.
 *
 * On choisit une intention par rôle — aucun accès, lecture seule, participation,
 * modération — et le domaine la traduit en permissions Discord cohérentes pour
 * le type de salon. Discord fait l'inverse : une grille de quarante
 * interrupteurs, dont beaucoup sans effet sur le salon qu'on règle.
 */
import { computed, ref } from "vue";
import type { LiveRole, PlanItem } from "@/services/guildStructureService";
import type { AccessMode } from "@/services/guildStructureService";
import { ACCESS_MODES, setAccess, removeAccess } from "@/composables/useServerBuilder";

const props = defineProps<{
  item: PlanItem;
  roles: LiveRole[];
  /// ID du serveur = ID du rôle @everyone (convention Discord).
  guildId: string;
  disabled?: boolean;
}>();

const open = ref(false);
const rules = computed(() => props.item.access ?? []);

function roleName(id: string): string {
  if (id === props.guildId) return "@everyone";
  return props.roles.find((r) => r.id === id)?.name ?? `Rôle inconnu (${id})`;
}

/// Rôles encore proposables : ceux qui n'ont pas déjà une règle. Les rôles
/// gérés par une intégration (bots) sont écartés — leurs permissions sont du
/// ressort de l'intégration, pas du nôtre.
const available = computed(() => {
  const used = new Set(rules.value.map((r) => r.role_id));
  const list: { id: string; name: string }[] = [];
  if (!used.has(props.guildId)) list.push({ id: props.guildId, name: "@everyone" });
  for (const r of props.roles) {
    if (r.id === props.guildId || r.managed || used.has(r.id)) continue;
    list.push({ id: r.id, name: r.name });
  }
  return list;
});

const toAdd = ref("");

function add() {
  if (!toAdd.value) return;
  // « Participation » par défaut : on ajoute un rôle pour lui ouvrir un salon
  // bien plus souvent que pour l'en exclure.
  setAccess(props.item, toAdd.value, "write");
  toAdd.value = "";
  open.value = true;
}

function drop(roleId: string) {
  removeAccess(props.item, roleId);
}

function update(roleId: string, mode: AccessMode) {
  setAccess(props.item, roleId, mode);
}
</script>

<template>
  <div class="access">
    <button
      class="toggle"
      :class="{ active: rules.length > 0 }"
      :aria-expanded="open"
      :aria-label="`Accès de ${item.name || 'ce salon'} : ${rules.length} règle(s)`"
      @click="open = !open"
    >
      🔐 Accès<span v-if="rules.length"> ({{ rules.length }})</span>
      <span class="caret" aria-hidden="true">{{ open ? "▾" : "▸" }}</span>
    </button>

    <div v-if="open" class="panel">
      <p v-if="!rules.length" class="hint">
        Aucune règle : le salon hérite des permissions de sa catégorie, ou à défaut
        de celles du serveur.
      </p>

      <div v-for="rule in rules" :key="rule.role_id" class="rule">
        <span class="role-name" :title="rule.role_id">{{ roleName(rule.role_id) }}</span>
        <select
          class="input mode-select"
          :value="rule.mode"
          :aria-label="`Niveau d'accès de ${roleName(rule.role_id)}`"
          :disabled="disabled"
          @change="update(rule.role_id, ($event.target as HTMLSelectElement).value as AccessMode)"
        >
          <option v-for="m in ACCESS_MODES" :key="m.value" :value="m.value" :title="m.hint">
            {{ m.icon }} {{ m.label }}
          </option>
        </select>
        <button
          class="del"
          :aria-label="`Retirer la règle d'accès de ${roleName(rule.role_id)}`"
          title="Retirer cette règle"
          :disabled="disabled"
          @click="drop(rule.role_id)"
        >✕</button>
      </div>

      <div v-if="!disabled" class="add">
        <select
          v-model="toAdd"
          class="input role-select"
          :aria-label="`Ajouter un rôle à ${item.name || 'ce salon'}`"
        >
          <option value="">Ajouter un rôle…</option>
          <option v-for="r in available" :key="r.id" :value="r.id">{{ r.name }}</option>
        </select>
        <button class="mini-add" :disabled="!toAdd" @click="add">+ Ajouter</button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.access { width: 100%; }
.toggle {
  background: none; border: 1px dashed var(--border); border-radius: var(--radius-pill);
  padding: 2px 10px; font-size: 11px; cursor: pointer; color: var(--text-secondary);
}
.toggle.active { border-style: solid; border-color: var(--accent); color: var(--accent); }
.caret { margin-left: 4px; opacity: 0.7; }

.panel {
  margin: 6px 0 4px 12px; padding: 8px 10px;
  border-left: 2px solid var(--border); display: flex; flex-direction: column; gap: 6px;
}
.hint { margin: 0; font-size: 11px; color: var(--text-secondary); line-height: 1.5; }

.rule { display: flex; align-items: center; gap: 8px; }
.role-name {
  flex: 1; font-size: 12px; overflow: hidden;
  text-overflow: ellipsis; white-space: nowrap;
}
.add { display: flex; align-items: center; gap: 6px; }

/* Aligné sur le standard d'inputs du repo (global.css .app-input-base). */
.input {
  background: var(--bg-card); color: var(--text-primary);
  border: 1px solid var(--border); border-radius: 6px;
  padding: 8px 12px; font-size: 13px; font-family: inherit; outline: none;
}
.input:focus { border-color: var(--accent); box-shadow: var(--focus-ring); }
.mode-select { width: 170px; }
.role-select { flex: 1; min-width: 120px; }

.del {
  background: none; border: none; cursor: pointer; opacity: 0.6;
  font-size: 0.85rem; padding: 8px; line-height: 1; color: inherit;
  border-radius: var(--radius-sm);
}
.del:hover:not(:disabled) { opacity: 1; color: var(--danger); }
.del:focus-visible { outline: 2px solid var(--danger); outline-offset: 1px; opacity: 1; }
.del:disabled { opacity: 0.25; cursor: default; }

.mini-add {
  background: none; border: 1px dashed var(--border); border-radius: var(--radius-pill);
  padding: 6px 12px; font-size: 11px; cursor: pointer; color: var(--text-secondary);
}
.mini-add:hover:not(:disabled) { border-color: var(--accent); color: var(--accent); }
.mini-add:disabled { opacity: 0.4; cursor: default; }
</style>
