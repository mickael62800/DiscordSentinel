<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useTauntsConfig } from "@/composables/useTauntsConfig";
import AppButton from "@/components/atoms/AppButton.vue";
import AppSelect from "@/components/atoms/AppSelect.vue";
import AppToggle from "@/components/atoms/AppToggle.vue";
import FormField from "@/components/atoms/FormField.vue";

const { config, channels, save } = useTauntsConfig();

const channelInput = ref("");
const enabledInput = ref(true);
const renameEnabledInput = ref(true);
const messagesEnabledInput = ref(true);
const saving = ref(false);

const channelOptions = computed(() => [
  { value: "", label: "— Aucun (desactive) —" },
  ...channels.value.map((c) => ({ value: c.id, label: `# ${c.name}` })),
]);

watch(
  config,
  (cfg) => {
    if (!cfg) return;
    channelInput.value = cfg.channel_id ?? "";
    enabledInput.value = cfg.enabled;
    renameEnabledInput.value = cfg.rename_enabled;
    messagesEnabledInput.value = cfg.messages_enabled;
  },
  { immediate: true },
);

async function onSave() {
  saving.value = true;
  try {
    await save({
      channel_id: channelInput.value.length > 0 ? channelInput.value : null,
      enabled: enabledInput.value,
      rename_enabled: renameEnabledInput.value,
      messages_enabled: messagesEnabledInput.value,
    });
  } finally {
    saving.value = false;
  }
}
</script>

<template>
  <div v-if="config" class="card card--lg config-card">
    <h2>Configuration</h2>

    <FormField
      label="Salon des railleries"
      hint="Choisir un salon texte. Selectionner « Aucun » desactive la feature."
    >
      <AppSelect v-model="channelInput" :options="channelOptions" />
    </FormField>

    <FormField label="Active">
      <div class="toggle-row">
        <AppToggle v-model="enabledInput" />
        <span class="toggle-label">
          {{ enabledInput ? "Les railleries sont postees" : "Feature desactivee globalement" }}
        </span>
      </div>
    </FormField>

    <div class="toggles-grid">
      <FormField label="Messages de raillerie">
        <div class="toggle-row">
          <AppToggle v-model="messagesEnabledInput" :disabled="!enabledInput" />
          <span class="toggle-label">
            {{ messagesEnabledInput ? "Les messages sont postes dans le salon" : "Aucun message poste" }}
          </span>
        </div>
      </FormField>

      <FormField label="Renommer les pseudos">
        <div class="toggle-row">
          <AppToggle v-model="renameEnabledInput" :disabled="!enabledInput" />
          <span class="toggle-label">
            {{ renameEnabledInput ? "Les pseudos sont renommes sur palier" : "Aucun rename applique" }}
          </span>
        </div>
      </FormField>
    </div>

    <div class="actions">
      <AppButton :disabled="saving" @click="onSave">
        {{ saving ? "Sauvegarde…" : "Enregistrer" }}
      </AppButton>
    </div>
  </div>
</template>

<style scoped>
.config-card {
  display: flex;
  flex-direction: column;
  gap: var(--space-lg);
}
.config-card h2 { margin: 0 0 var(--space-xs); font-size: 18px; }
.toggle-row { display: flex; align-items: center; gap: 12px; }
.toggles-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(260px, 1fr));
  gap: var(--space-lg);
}
.toggle-label { font-size: 14px; color: var(--text-secondary); }
.actions { display: flex; justify-content: flex-end; margin-top: 8px; }
</style>
