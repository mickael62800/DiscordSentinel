<script setup lang="ts">
import { onMounted, onUnmounted } from "vue";
import TopBar from "../organisms/TopBar.vue";
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
  <div class="main-wrapper">
    <TopBar />
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
  /* Mobile : empeche les pages enfants de pousser horizontalement le viewport
     a cause d'un overflow imprevisible (textes longs, embeds, etc.) */
  min-width: 0;
}

.main-content {
  flex: 1;
  overflow-y: auto;
  overflow-x: hidden;
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
    padding: 14px 10px;
  }
}
</style>
