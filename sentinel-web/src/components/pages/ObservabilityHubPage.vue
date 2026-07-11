<script setup lang="ts">
import { ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import AppTabs from "../molecules/AppTabs.vue";
import LogsPage from "./LogsPage.vue";
import SystemLogsPage from "./SystemLogsPage.vue";
import AuditPage from "./AuditPage.vue";

type TabKey = "business" | "system" | "audit";
const tabs = [
  { key: "business", label: "Journaux métier" },
  { key: "system", label: "Système" },
  { key: "audit", label: "Audit" },
];

const route = useRoute();
const router = useRouter();

// Onglet actif dérivé de l'URL : /system-logs -> système, /audit -> audit,
// sinon journaux métier. Les trois chemins restent bookmarkables.
function tabFromPath(path: string): TabKey {
  if (path.startsWith("/system-logs")) return "system";
  if (path.startsWith("/audit")) return "audit";
  return "business";
}
const activeTab = ref<TabKey>(tabFromPath(route.path));

watch(
  () => route.path,
  (p) => {
    activeTab.value = tabFromPath(p);
  },
);

function onTabChange(key: string) {
  const target =
    key === "system" ? "/system-logs" : key === "audit" ? "/audit" : "/logs";
  if (route.path !== target) router.replace(target);
  activeTab.value = key as TabKey;
}
</script>

<template>
  <div class="observability-hub">
    <div class="hub-tabs-wrap">
      <AppTabs :model-value="activeTab" :tabs="tabs" @update:model-value="onTabChange" />
    </div>
    <LogsPage v-if="activeTab === 'business'" />
    <SystemLogsPage v-else-if="activeTab === 'system'" />
    <AuditPage v-else />
  </div>
</template>

<style scoped>
.hub-tabs-wrap {
  margin-bottom: 20px;
}
</style>
