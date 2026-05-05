<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { storeToRefs } from "pinia";
import type { BotDefinition } from "../../types";
import { useGuildSelector } from "../../composables/useGuildSelector";
import { useBotDefinitions } from "../../composables/useBotDefinitions";
import { useBotEnabledStatus } from "../../composables/useBotEnabledStatus";
import { useBotEnabledStatusStore } from "@/stores/botEnabledStatusStore";
import AdminPageShell from "../layouts/AdminPageShell.vue";
import ComponentSelectorSection from "../organisms/ComponentSelectorSection.vue";
import ComponentConfigForm from "../organisms/ComponentConfigForm.vue";
import AutomodAnalysisHistory from "../organisms/AutomodAnalysisHistory.vue";

const { selectedGuildId, selectedGuild } = useGuildSelector();
const { fetchConfigs } = useBotEnabledStatus();

// Une seule source de verite : le store. La page ne fait pas de
// fetch separe. Le store est deja charge par useAppInit + le watch
// dans useBotEnabledStatus a la selection de guild.
const botEnabledStore = useBotEnabledStatusStore();
const { configs } = storeToRefs(botEnabledStore);

const definitions = ref<BotDefinition[]>([]);
const selectedComponent = ref<string | null>(null);

function isWorker(botName: string): boolean {
  return botName.endsWith("-worker");
}

const moduleDefinitions = computed(() =>
  definitions.value.filter((d) => !isWorker(d.bot_name)),
);
const workerDefinitions = computed(() =>
  definitions.value.filter((d) => isWorker(d.bot_name)),
);

const selectedDefinition = computed(() =>
  definitions.value.find((d) => d.bot_name === selectedComponent.value) ?? null,
);

async function fetchDefinitions() {
  try {
    const { ensure } = useBotDefinitions();
    definitions.value = await ensure();
  } catch (e) {
    console.error("Erreur chargement definitions:", e);
  }
}

function selectComponent(name: string) {
  selectedComponent.value = name;
}

// fetchConfigs() invalide + recharge le store (la seule source).
// Appele apres un save dans le formulaire.
async function reloadAfterSave() {
  await fetchConfigs();
}

onMounted(() => {
  fetchDefinitions();
});
</script>

<template>
  <AdminPageShell title="Configuration des composants">
    <template #lede>
      Parametrer chaque composant pour le serveur selectionne
    </template>

    <div v-if="!selectedGuildId" class="empty-state">
      <p>Selectionnez un serveur dans la barre laterale pour configurer les composants.</p>
    </div>

    <template v-else>
      <div class="server-info">
        <span class="server-label">Serveur :</span>
        <span class="server-name">{{ selectedGuild?.name }}</span>
      </div>

      <ComponentSelectorSection
        title="Modules"
        :definitions="moduleDefinitions"
        :selected-key="selectedComponent"
        @select="selectComponent"
      />

      <ComponentSelectorSection
        title="Workers"
        :definitions="workerDefinitions"
        :selected-key="selectedComponent"
        @select="selectComponent"
      />

      <ComponentConfigForm
        v-if="selectedDefinition"
        :definition="selectedDefinition"
        :configs="configs"
        :guild-id="selectedGuildId"
        @saved="reloadAfterSave"
      />

      <!-- Vue debug temporaire : historique des analyses automod. -->
      <AutomodAnalysisHistory
        v-if="selectedComponent === 'automod-bot' && selectedGuildId"
        :guild-id="selectedGuildId"
      />
    </template>
  </AdminPageShell>
</template>

<style scoped>
.empty-state {
  text-align: center;
  padding: 60px 20px;
  color: var(--text-secondary);
  font-size: 15px;
}

.server-info {
  margin-bottom: 20px;
  padding: 10px 16px;
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: 8px;
  font-size: 14px;
}
.server-label { color: var(--text-secondary); margin-right: 8px; }
.server-name { font-weight: 600; color: var(--text-primary); }
</style>
