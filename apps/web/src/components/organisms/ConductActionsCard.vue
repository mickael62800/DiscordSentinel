<script setup lang="ts">
import { ref } from "vue";
import { useToast } from "@/composables/useToast";
import { conductService } from "@/services/conductService";

const { success, error: showError } = useToast();

const tickRunning = ref(false);
const syncingBans = ref(false);

async function runRegenTick() {
  tickRunning.value = true;
  try {
    await conductService.runRegenTick();
    success("Tick de régénération exécuté.");
  } catch (e) {
    console.error(e);
    showError("Erreur lors du tick.");
  } finally {
    tickRunning.value = false;
  }
}

async function syncBans() {
  syncingBans.value = true;
  try {
    const r = await conductService.syncBanProposals();
    success(`${r.created} proposition(s) de ban créée(s).`);
  } catch (e) {
    console.error(e);
    showError("Erreur sync ban proposals.");
  } finally {
    syncingBans.value = false;
  }
}
</script>

<template>
  <section class="card">
    <h2>⚡ Actions conduite</h2>
    <p class="hint">
      Le worker exécute ces tâches périodiquement. Les boutons ci-dessous
      permettent de forcer manuellement (debug / déblocage).
    </p>
    <div class="action-buttons">
      <button class="btn-secondary" @click="runRegenTick" :disabled="tickRunning">
        {{ tickRunning ? "…" : "Forcer le tick de régénération" }}
      </button>
      <button class="btn-warn" @click="syncBans" :disabled="syncingBans">
        {{ syncingBans ? "…" : "Sync ban proposals manuel" }}
      </button>
    </div>
  </section>
</template>

<style scoped>
@import "../pages/_moderation-advanced-shared.css";
.action-buttons { display: flex; flex-direction: column; gap: 8px; }
.hint { font-size: 0.85rem; color: var(--text-secondary); margin-bottom: 12px; }
</style>
