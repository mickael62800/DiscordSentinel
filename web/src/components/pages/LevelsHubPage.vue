<script setup lang="ts">
import { ref } from "vue";
import AppTabs from "../molecules/AppTabs.vue";
import LevelsPage from "./LevelsPage.vue";
import LevelsConfigPage from "./LevelsConfigPage.vue";

type TabKey = "leaderboard" | "config";
const activeTab = ref<TabKey>("leaderboard");
const tabs = [
  { key: "leaderboard", label: "Classement" },
  { key: "config", label: "Configuration" },
];
</script>

<template>
  <div class="levels-hub">
    <div class="hub-tabs-wrap">
      <AppTabs v-model="activeTab" :tabs="tabs" />
    </div>
    <LevelsPage v-if="activeTab === 'leaderboard'" />
    <LevelsConfigPage v-else />
  </div>
</template>

<style scoped>
.hub-tabs-wrap {
  margin-bottom: 20px;
}
</style>
