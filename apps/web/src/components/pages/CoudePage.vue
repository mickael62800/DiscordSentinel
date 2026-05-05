<script setup lang="ts">
import { ref } from "vue";
import { coudeService } from "@/services/coudeService";
import { useGuildSelector } from "../../composables/useGuildSelector";
import { useConfirm } from "../../composables/useConfirm";
import { useToast } from "../../composables/useToast";
import { useComponentVisibility } from "@/composables/useComponentVisibility";
import AppButton from "../atoms/AppButton.vue";
import AdminPageShell from "../layouts/AdminPageShell.vue";
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
  <AdminPageShell title="Coup de Coude" icon="⚔️">
    <template #lede>
      Administration du jeu — suivi des combats et statistiques des joueurs.
      Les coins sont geres via la page <strong>Wallet</strong>.
    </template>
    <template #actions>
      <AppButton variant="secondary" @click="refreshActive">↻ Rafraichir</AppButton>
      <button
        v-if="visible('db.purge.coude')"
        class="danger-btn"
        :disabled="purging"
        @click="handlePurgeAll"
        title="Supprime DEFINITIVEMENT toutes les donnees coude de cette guild (owner uniquement)"
      >
        {{ purging ? "Purge…" : "🗑 Reset total" }}
      </button>
    </template>

    <AppTabs
      :model-value="activeTab"
      :tabs="mainTabs"
      class="main-tabs"
      @update:model-value="(k) => (activeTab = k as TabKey)"
    />

    <CoudeCombatsTab v-if="activeTab === 'combats'" ref="combatsTabRef" />
    <CoudePlayersTab v-else ref="playersTabRef" />
  </AdminPageShell>
</template>

<style scoped>
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

.main-tabs { width: 100%; }
</style>
