<script setup lang="ts">
import { computed, ref, watch } from "vue";
import {
  gamePortalService,
  type ConfigField,
  type GameTemplate,
} from "@/services/gamePortalService";
import { useToast } from "@/composables/useToast";

const props = defineProps<{
  open: boolean;
  serverId: string | null;
  serverName: string;
  template: GameTemplate | null;
  initialConfig: Record<string, string>;
  actorId?: string;
}>();

const emit = defineEmits<{
  (e: "close"): void;
  (e: "saved"): void;
}>();

const { success, error: toastError } = useToast();

// Etat editable : key -> string (sera serialise vers le backend)
const values = ref<Record<string, string>>({});
const saving = ref(false);

watch(
  () => [props.open, props.serverId, props.template],
  () => {
    if (props.open && props.template) {
      // Pre-remplit avec : initialConfig (override) || default du schema
      const next: Record<string, string> = {};
      for (const f of props.template.config_schema) {
        const fromOverride = props.initialConfig[f.key];
        if (fromOverride !== undefined) {
          next[f.key] = fromOverride;
        } else if (f.default !== undefined) {
          next[f.key] = String(f.default);
        } else {
          next[f.key] = "";
        }
      }
      values.value = next;
    }
  },
  { immediate: true },
);

const fields = computed<ConfigField[]>(
  () => props.template?.config_schema ?? [],
);

function setBool(key: string, val: boolean) {
  values.value[key] = val ? "true" : "false";
}

function isBoolTrue(key: string): boolean {
  return values.value[key] === "true";
}

async function save() {
  if (!props.serverId) return;
  // Filtre : on n'envoie que les clefs qui different du default OU sont
  // explicitement non-vides. Permet de "reset au default" en effacant un
  // champ manuellement.
  const payload: Record<string, string> = {};
  for (const f of fields.value) {
    const v = values.value[f.key];
    if (v !== undefined && v !== "") {
      payload[f.key] = v;
    }
  }
  saving.value = true;
  try {
    await gamePortalService.updateConfig(
      props.serverId,
      payload,
      props.actorId,
    );
    success("Configuration sauvegardée. Redémarre le serveur pour appliquer.");
    emit("saved");
    emit("close");
  } catch (e) {
    toastError(`Échec : ${e instanceof Error ? e.message : String(e)}`);
  } finally {
    saving.value = false;
  }
}
</script>

<template>
  <div v-if="open" class="modal-overlay" @click.self="emit('close')">
    <div class="modal">
      <header class="modal-head">
        <div>
          <h2>Configuration — {{ serverName }}</h2>
          <p class="modal-sub" v-if="template">
            Template <code>{{ template.slug }}</code> — {{ fields.length }} champ(s)
          </p>
        </div>
        <button class="btn-close" @click="emit('close')" aria-label="Fermer">×</button>
      </header>

      <div v-if="!template" class="modal-empty">Template introuvable.</div>
      <div v-else-if="fields.length === 0" class="modal-empty">
        Aucun champ configurable pour ce template.
      </div>
      <div v-else class="modal-body">
        <div v-for="f in fields" :key="f.key" class="field">
          <label :for="`cfg-${f.key}`" class="field-label">
            {{ f.label }}
            <code class="field-key">{{ f.key }}</code>
          </label>

          <!-- Boolean -->
          <label v-if="f.type === 'boolean'" class="check-row">
            <input
              type="checkbox"
              :checked="isBoolTrue(f.key)"
              @change="setBool(f.key, ($event.target as HTMLInputElement).checked)"
            />
            <span>{{ isBoolTrue(f.key) ? "Activé" : "Désactivé" }}</span>
          </label>

          <!-- Enum -->
          <select
            v-else-if="f.type === 'enum' && f.options"
            :id="`cfg-${f.key}`"
            v-model="values[f.key]"
            class="field-input"
          >
            <option v-for="opt in f.options" :key="opt" :value="opt">{{ opt }}</option>
          </select>

          <!-- Number -->
          <input
            v-else-if="f.type === 'number'"
            :id="`cfg-${f.key}`"
            v-model="values[f.key]"
            type="number"
            :min="f.min"
            :max="f.max"
            :placeholder="f.default !== undefined ? String(f.default) : ''"
            class="field-input"
          />

          <!-- Text -->
          <input
            v-else
            :id="`cfg-${f.key}`"
            v-model="values[f.key]"
            type="text"
            :maxlength="f.max_length"
            :placeholder="f.default !== undefined ? String(f.default) : ''"
            class="field-input"
          />
        </div>
      </div>

      <footer class="modal-foot">
        <button class="btn-cancel" :disabled="saving" @click="emit('close')">
          Annuler
        </button>
        <button class="btn-save" :disabled="saving" @click="save">
          {{ saving ? "Sauvegarde…" : "Enregistrer" }}
        </button>
      </footer>
    </div>
  </div>
</template>

<style scoped>
.modal-overlay {
  position: fixed;
  inset: 0;
  background: var(--modal-overlay, rgba(0, 0, 0, 0.6));
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
  padding: 16px;
}

.modal {
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg, 12px);
  width: 100%;
  max-width: 640px;
  max-height: 90vh;
  display: flex;
  flex-direction: column;
  box-shadow: var(--shadow-xl);
}

.modal-head {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  padding: var(--space-lg);
  border-bottom: 1px solid var(--border);
}

.modal-head h2 {
  margin: 0;
  font-size: 16px;
}

.modal-sub {
  margin: 4px 0 0;
  font-size: 12px;
  color: var(--text-secondary);
}

.modal-sub code {
  background: var(--bg-card);
  padding: 1px 6px;
  border-radius: 4px;
  font-family: monospace;
}

.btn-close {
  width: 32px;
  height: 32px;
  border: none;
  background: transparent;
  color: var(--text-secondary);
  font-size: 22px;
  cursor: pointer;
  border-radius: var(--radius-sm);
}

.btn-close:hover {
  background: var(--bg-hover);
  color: var(--text-primary);
}

.modal-body {
  padding: var(--space-lg);
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 14px;
  flex: 1;
  min-height: 0;
}

.modal-empty {
  padding: var(--space-2xl);
  text-align: center;
  color: var(--text-secondary);
}

.field {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.field-label {
  font-size: 12px;
  font-weight: 600;
  color: var(--text-primary);
  display: flex;
  align-items: center;
  gap: 8px;
}

.field-key {
  font-family: monospace;
  font-size: 10px;
  color: var(--text-secondary);
  background: var(--bg-card);
  padding: 1px 6px;
  border-radius: 3px;
  font-weight: normal;
}

.field-input {
  width: 100%;
  padding: 8px 12px;
  border: 1px solid var(--border);
  border-radius: 6px;
  background: var(--bg-card);
  color: var(--text-primary);
  font-size: 13px;
}

.field-input:focus {
  outline: none;
  border-color: var(--accent);
}

.check-row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 6px 0;
  font-size: 13px;
  color: var(--text-primary);
  cursor: pointer;
  user-select: none;
}

.modal-foot {
  display: flex;
  gap: 10px;
  justify-content: flex-end;
  padding: var(--space-md) var(--space-lg);
  border-top: 1px solid var(--border);
}

.btn-cancel,
.btn-save {
  padding: 8px 18px;
  border-radius: 6px;
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  border: 1px solid var(--border);
}

.btn-cancel {
  background: transparent;
  color: var(--text-secondary);
}

.btn-cancel:hover:not(:disabled) {
  background: var(--bg-hover);
  color: var(--text-primary);
}

.btn-save {
  background: var(--accent);
  border-color: var(--accent);
  color: #fff;
}

.btn-save:hover:not(:disabled) {
  background: var(--accent-hover);
}

.btn-cancel:disabled,
.btn-save:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

@media (max-width: 600px) {
  .modal {
    max-height: 95vh;
  }
}
</style>
