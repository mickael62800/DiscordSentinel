<script setup lang="ts">
import { ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import AppTabs from "../molecules/AppTabs.vue";
import RolePanelsPage from "./RolePanelsPage.vue";
import DiscordRolesPage from "./DiscordRolesPage.vue";

type TabKey = "panels" | "roles";
const tabs = [
  { key: "panels", label: "Panneaux de rôles" },
  { key: "roles", label: "Rôles Discord" },
];

const route = useRoute();
const router = useRouter();

// L'onglet actif est dérivé de l'URL : /discord-roles ouvre "roles", sinon
// "panels". Ainsi le lien croisé "Voir tous les rôles" continue de fonctionner,
// et l'onglet reste bookmarkable.
function tabFromPath(path: string): TabKey {
  return path.startsWith("/discord-roles") ? "roles" : "panels";
}
const activeTab = ref<TabKey>(tabFromPath(route.path));

watch(
  () => route.path,
  (p) => {
    activeTab.value = tabFromPath(p);
  },
);

// Cliquer un onglet met à jour l'URL (sans empiler l'historique).
function onTabChange(key: string) {
  const target = key === "roles" ? "/discord-roles" : "/role-panels";
  if (route.path !== target) router.replace(target);
  activeTab.value = key as TabKey;
}
</script>

<template>
  <div class="roles-hub">
    <div class="hub-tabs-wrap">
      <AppTabs :model-value="activeTab" :tabs="tabs" @update:model-value="onTabChange" />
    </div>
    <RolePanelsPage v-if="activeTab === 'panels'" />
    <DiscordRolesPage v-else />
  </div>
</template>

<style scoped>
.hub-tabs-wrap {
  margin-bottom: 20px;
}
</style>
