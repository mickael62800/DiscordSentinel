<script setup lang="ts">
import { ref, onMounted, onUnmounted } from "vue";
import { getApiBaseUrl } from "../../utils/api";

const apiStatus = ref<"ok" | "down" | "checking">("checking");
let interval: ReturnType<typeof setInterval> | null = null;

async function checkApi() {
  try {
    const baseUrl = await getApiBaseUrl();
    const resp = await fetch(`${baseUrl}/health`, { signal: AbortSignal.timeout(3000) });
    apiStatus.value = resp.ok ? "ok" : "down";
  } catch {
    apiStatus.value = "down";
  }
}

onMounted(() => {
  checkApi();
  interval = setInterval(checkApi, 30000);
});

onUnmounted(() => {
  if (interval) clearInterval(interval);
});
</script>

<template>
  <div v-if="apiStatus === 'down'" class="connection-banner">
    <span class="banner-icon">!</span>
    <span class="banner-text">Connexion au serveur perdue. Certaines donnees peuvent etre indisponibles.</span>
    <button class="banner-retry" @click="checkApi">Verifier</button>
  </div>
</template>

<style scoped>
.connection-banner {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 20px;
  background: linear-gradient(90deg, rgba(239, 68, 68, 0.15), rgba(239, 68, 68, 0.05));
  border-bottom: 1px solid rgba(239, 68, 68, 0.3);
  font-size: 13px;
  color: #fca5a5;
}

.banner-icon {
  width: 20px;
  height: 20px;
  border-radius: 50%;
  background: #ef4444;
  color: white;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 12px;
  font-weight: 700;
  flex-shrink: 0;
}

.banner-text {
  flex: 1;
}

.banner-retry {
  background: transparent;
  border: 1px solid rgba(239, 68, 68, 0.5);
  color: #fca5a5;
  border-radius: 4px;
  padding: 4px 12px;
  font-size: 12px;
  cursor: pointer;
  white-space: nowrap;
}

.banner-retry:hover {
  background: rgba(239, 68, 68, 0.2);
}
</style>
