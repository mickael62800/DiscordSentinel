<script setup lang="ts">
import { ref } from "vue";
import AppTabs from "../molecules/AppTabs.vue";
import VoiceChannelsPage from "./VoiceChannelsPage.vue";
import VoiceThemesPage from "./VoiceThemesPage.vue";

type TabKey = "channels" | "themes";
const activeTab = ref<TabKey>("channels");
const tabs = [
  { key: "channels", label: "Salons" },
  { key: "themes", label: "Thèmes" },
];
</script>

<template>
  <div class="voice-hub">
    <div class="hub-tabs-wrap">
      <AppTabs v-model="activeTab" :tabs="tabs" />
    </div>
    <VoiceChannelsPage v-if="activeTab === 'channels'" />
    <VoiceThemesPage v-else />
  </div>
</template>

<style scoped>
.hub-tabs-wrap {
  margin-bottom: 20px;
}
</style>
