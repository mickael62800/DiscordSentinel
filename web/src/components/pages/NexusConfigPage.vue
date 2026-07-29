<script setup lang="ts">
// Configuration des modules de la plateforme jeux Nexus.
//
// Volontairement minimale : le formulaire est entierement pilote par le
// `config_schema` stocke en base Nexus (table bot_definitions). Ajouter un
// reglage cote backend le fait apparaitre ici sans toucher au front.

import { computed, ref, watch } from "vue";
import type { BotDefinition, BotGuildConfig } from "../../types";
import { useGuildSelector } from "../../composables/useGuildSelector";
import { nexusConfigService } from "@/services/nexusConfigService";
import AdminPageShell from "../layouts/AdminPageShell.vue";
import ComponentConfigForm from "../organisms/ComponentConfigForm.vue";

const { selectedGuildId, selectedGuild } = useGuildSelector();

const definitions = ref<BotDefinition[]>([]);
const configs = ref<BotGuildConfig[]>([]);
const selectedBot = ref<string | null>(null);
const loading = ref(false);
const errorMessage = ref("");

const selectedDefinition = computed(
  () => definitions.value.find((d) => d.bot_name === selectedBot.value) ?? null,
);

async function loadDefinitions() {
  if (!selectedGuildId.value) return;
  loading.value = true;
  errorMessage.value = "";
  try {
    definitions.value = await nexusConfigService.getDefinitions(selectedGuildId.value);
    if (!selectedBot.value && definitions.value.length) {
      selectedBot.value = definitions.value[0].bot_name;
    }
    await loadConfigs();
  } catch (e) {
    errorMessage.value = e instanceof Error ? e.message : "Chargement impossible";
    definitions.value = [];
  } finally {
    loading.value = false;
  }
}

async function loadConfigs() {
  if (!selectedGuildId.value || !selectedBot.value) return;
  try {
    configs.value = await nexusConfigService.getGuildConfig(
      selectedGuildId.value,
      selectedBot.value,
    );
  } catch (e) {
    errorMessage.value = e instanceof Error ? e.message : "Chargement impossible";
    configs.value = [];
  }
}

watch(selectedGuildId, loadDefinitions, { immediate: true });
watch(selectedBot, loadConfigs);
</script>

<template>
  <AdminPageShell
    title="Configuration Nexus"
    :subtitle="selectedGuild?.name ?? 'Aucun serveur selectionne'"
  >
    <p v-if="!selectedGuildId" class="nexus-hint">
      Selectionne un serveur Discord pour configurer la plateforme jeux.
    </p>

    <p v-else-if="errorMessage" class="nexus-error">{{ errorMessage }}</p>

    <p v-else-if="loading" class="nexus-hint">Chargement…</p>

    <template v-else>
      <div v-if="definitions.length > 1" class="nexus-modules">
        <button
          v-for="d in definitions"
          :key="d.bot_name"
          type="button"
          class="nexus-module"
          :class="{ active: d.bot_name === selectedBot }"
          @click="selectedBot = d.bot_name"
        >
          {{ d.display_name }}
        </button>
      </div>

      <p v-if="selectedDefinition?.description" class="nexus-desc">
        {{ selectedDefinition.description }}
      </p>

      <ComponentConfigForm
        v-if="selectedDefinition && selectedGuildId"
        :definition="selectedDefinition"
        :configs="configs"
        :guild-id="selectedGuildId"
        :persistence="nexusConfigService"
        @saved="loadConfigs"
      />
    </template>
  </AdminPageShell>
</template>

<style scoped>
.nexus-hint,
.nexus-desc {
  color: var(--text-secondary);
  margin: 0 0 var(--space-md);
}

.nexus-error {
  color: var(--danger);
  margin: 0 0 var(--space-md);
}

.nexus-modules {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-sm);
  margin-bottom: var(--space-md);
}

.nexus-module {
  background: var(--bg-card);
  border: 1px solid var(--bg-hover);
  color: var(--text-secondary);
  border-radius: var(--radius-md);
  padding: var(--space-xs) var(--space-md);
  cursor: pointer;
  transition: var(--transition-fast);
}

.nexus-module:hover {
  color: var(--text-primary);
}

.nexus-module.active {
  border-color: var(--accent);
  color: var(--text-primary);
}
</style>
