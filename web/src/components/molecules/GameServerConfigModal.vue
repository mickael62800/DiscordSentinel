<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { errMsg } from "@/utils/errMsg";
import {
  gamePortalService,
  type ConfigField,
  type GameTemplate,
} from "@/services/gamePortalService";
import { useToast } from "@/composables/useToast";
import AppModal from "../atoms/AppModal.vue";
import AppButton from "../atoms/AppButton.vue";

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

const values = ref<Record<string, string>>({});
const saving = ref(false);

watch(
  () => [props.open, props.serverId, props.template],
  () => {
    if (props.open && props.template) {
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
    toastError(`Échec : ${errMsg(e)}`);
  } finally {
    saving.value = false;
  }
}
</script>

<template>
  <AppModal :visible="open" size="lg" @close="emit('close')">
    <template #header>
      <div>
        <h2>Configuration — {{ serverName }}</h2>
        <p class="modal-sub" v-if="template">
          Template <code>{{ template.slug }}</code> — {{ fields.length }} champ(s)
        </p>
      </div>
    </template>

    <div v-if="!template" class="modal-empty">Template introuvable.</div>
    <div v-else-if="fields.length === 0" class="modal-empty">
      Aucun champ configurable pour ce template.
    </div>
    <div v-else class="fields-stack">
      <div v-for="f in fields" :key="f.key" class="field">
        <label :for="`cfg-${f.key}`" class="field-label">
          {{ f.label }}
          <code class="field-key">{{ f.key }}</code>
        </label>

        <label v-if="f.type === 'boolean'" class="check-row">
          <input
            type="checkbox"
            :checked="isBoolTrue(f.key)"
            @change="setBool(f.key, ($event.target as HTMLInputElement).checked)"
          />
          <span>{{ isBoolTrue(f.key) ? "Activé" : "Désactivé" }}</span>
        </label>

        <select
          v-else-if="f.type === 'enum' && f.options"
          :id="`cfg-${f.key}`"
          v-model="values[f.key]"
          class="field-input"
        >
          <option v-for="opt in f.options" :key="opt" :value="opt">{{ opt }}</option>
        </select>

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

    <template #footer>
      <AppButton variant="secondary" :disabled="saving" @click="emit('close')">
        Annuler
      </AppButton>
      <AppButton variant="primary" :disabled="saving" @click="save">
        {{ saving ? "Sauvegarde…" : "Enregistrer" }}
      </AppButton>
    </template>
  </AppModal>
</template>

<style scoped>
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

.modal-empty {
  padding: var(--space-2xl);
  text-align: center;
  color: var(--text-secondary);
}

.fields-stack { display: flex; flex-direction: column; gap: 14px; }

.field { display: flex; flex-direction: column; gap: 6px; }

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
.field-input:focus { outline: none; border-color: var(--accent); }

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
</style>
