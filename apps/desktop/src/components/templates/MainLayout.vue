<script setup lang="ts">
import { onMounted, onUnmounted } from "vue";
import SidebarNav from "../organisms/SidebarNav.vue";
import { useRealtime } from "../../composables/useRealtime";
import { useNotifications } from "../../composables/useNotifications";

const { init: initWs, disconnect, cleanup: cleanupWs } = useRealtime();
const { startListening, stopListening, closePanel } = useNotifications();

onMounted(async () => {
  await startListening();
  await initWs();
});

onUnmounted(() => {
  stopListening();
  cleanupWs();
  disconnect();
});
</script>

<template>
  <SidebarNav />
  <main class="main-content" @click="closePanel()">
    <slot />
  </main>
</template>

<style scoped>
.main-content {
  flex: 1;
  overflow-y: auto;
  padding: 32px;
}
</style>
