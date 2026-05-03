<script setup lang="ts">
import { ref } from "vue";
import { coudeService } from "@/services/coudeService";
import { useGuildSelector } from "../../composables/useGuildSelector";
import { useConfirm } from "../../composables/useConfirm";
import { useToast } from "../../composables/useToast";
import { useComponentVisibility } from "@/composables/useComponentVisibility";
import AppButton from "../atoms/AppButton.vue";
import AppTabs from "../molecules/AppTabs.vue";
import CoudeCombatsTab from "../organisms/CoudeCombatsTab.vue";
import CoudePlayersTab from "../organisms/CoudePlayersTab.vue";

const { visible } = useComponentVisibility();
const { selectedGuildId } = useGuildSelector();
const { confirm } = useConfirm();
const { success, error: toastError } = useToast();

type TabKey = "combats" | "players";
const activeTab = ref<TabKey>("combats");

const mainTabs = [
  { key: "combats", label: "⚔️ Combats" },
  { key: "players", label: "📊 Stats joueurs" },
];

const combatsTabRef = ref<InstanceType<typeof CoudeCombatsTab> | null>(null);
const playersTabRef = ref<InstanceType<typeof CoudePlayersTab> | null>(null);
const purging = ref(false);

function refreshActive() {
  if (activeTab.value === "combats") combatsTabRef.value?.refresh();
  else playersTabRef.value?.refresh();
}

async function handlePurgeAll() {
  if (!selectedGuildId.value) return;
  const ok1 = await confirm({
    title: "Reset total Coup de Coude",
    message:
      "Supprimer DEFINITIVEMENT toutes les donnees Coup de Coude (joueurs, combats, paris, assurances, inventaire, primes) pour cette guild ?",
  });
  if (!ok1) return;
  const ok2 = await confirm({
    title: "Confirmation finale",
    message: "Cette action est IRREVERSIBLE. Tout sera remis a zero.",
  });
  if (!ok2) return;
  purging.value = true;
  try {
    const res = await coudeService.purgeAll(selectedGuildId.value);
    const total = Object.values(res).reduce((a, b) => a + b, 0);
    success(`${total} ligne(s) supprimee(s).`);
    combatsTabRef.value?.refresh();
    playersTabRef.value?.refresh();
  } catch (e) {
    toastError(String(e));
  } finally {
    purging.value = false;
  }
}
</script>

<template>
  <div class="coude-page page--wide">
    <header class="hero">
      <div class="hero-text">
        <h1 class="hero-title">
          <span class="hero-icon">⚔️</span>
          Coup de Coude
        </h1>
        <p class="hero-subtitle">
          Administration du jeu — suivi des combats et statistiques des joueurs.
          Les coins sont geres via la page <strong>Wallet</strong>.
        </p>
      </div>
      <div class="hero-actions">
        <AppButton variant="secondary" @click="refreshActive">
          ↻ Rafraichir
        </AppButton>
        <button
          v-if="visible('db.purge.coude')"
          class="danger-btn"
          :disabled="purging"
          @click="handlePurgeAll"
          title="Supprime DEFINITIVEMENT toutes les donnees coude de cette guild (owner uniquement)"
        >
          {{ purging ? "Purge…" : "🗑 Reset total" }}
        </button>
      </div>
    </header>

    <AppTabs
      :model-value="activeTab"
      :tabs="mainTabs"
      class="main-tabs"
      @update:model-value="(k) => (activeTab = k as TabKey)"
    />

    <CoudeCombatsTab v-if="activeTab === 'combats'" ref="combatsTabRef" />
    <CoudePlayersTab v-else ref="playersTabRef" />
  </div>
</template>

<style scoped>
.coude-page {
  padding: 24px;
  display: flex;
  flex-direction: column;
  gap: 20px;
}

.hero {
  display: flex;
  justify-content: space-between;
  align-items: flex-end;
  padding-bottom: 16px;
  border-bottom: 1px solid var(--border);
}

.hero-actions { display: flex; gap: 8px; align-items: center; }

.danger-btn {
  background: transparent;
  color: var(--danger);
  border: 1px solid var(--danger);
  border-radius: 6px;
  padding: 8px 14px;
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  white-space: nowrap;
  transition: all var(--transition-fast);
}
.danger-btn:hover:not(:disabled) { background: var(--danger); color: white; }
.danger-btn:disabled { opacity: 0.5; cursor: not-allowed; }

.hero-title {
  display: flex;
  align-items: center;
  gap: 12px;
  margin: 0 0 6px;
  font-size: 2rem;
  font-weight: 700;
}
.hero-icon { font-size: 2rem; }
.hero-subtitle {
  margin: 0;
  color: var(--text-secondary);
  font-size: 0.95rem;
}

.main-tabs { width: 100%; }

@media (max-width: 768px) {
  .hero {
    flex-direction: column;
    align-items: flex-start;
    gap: 12px;
  }
  .hero-actions {
    width: 100%;
    flex-wrap: wrap;
  }
  .hero-actions > * { flex: 1; }
}
</style>
