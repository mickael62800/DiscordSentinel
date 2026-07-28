<script setup lang="ts">
import { ref } from "vue";
import AppTabs from "../molecules/AppTabs.vue";
import StatsPage from "./StatsPage.vue";
import ModstatsPage from "./ModstatsPage.vue";

type TabKey = "server" | "moderation";
const activeTab = ref<TabKey>("server");
const tabs = [
  { key: "server", label: "Serveur" },
  { key: "moderation", label: "Modération" },
];
</script>

<template>
  <div class="stats-hub">
    <div class="hub-tabs-wrap">
      <AppTabs v-model="activeTab" :tabs="tabs" />
    </div>
    <StatsPage v-if="activeTab === 'server'" />
    <ModstatsPage v-else />
  </div>
</template>

<style scoped>
.hub-tabs-wrap {
  margin-bottom: 20px;
}
</style>
