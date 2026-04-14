<script setup lang="ts">
import { onMounted, onUnmounted } from "vue";
import SidebarNav from "../organisms/SidebarNav.vue";
import ConnectionBanner from "../atoms/ConnectionBanner.vue";
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
  <div class="main-wrapper">
    <ConnectionBanner />
    <main class="main-content" @click="closePanel()">
      <slot />
    </main>
  </div>
</template>

<style scoped>
.main-wrapper {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.main-content {
  flex: 1;
  overflow-y: auto;
  padding: 32px;
  min-width: 0;
}

@media (max-width: 900px) {
  .main-content {
    padding: 20px 16px;
  }
}

@media (max-width: 600px) {
  .main-content {
    padding: 16px 12px;
  }
}
</style>
