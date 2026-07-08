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
import CoudeSocialPage from "./CoudeSocialPage.vue";
import TournamentPage from "./TournamentPage.vue";

const { visible } = useComponentVisibility();
const { selectedGuildId } = useGuildSelector();
const { confirm } = useConfirm();
const { success, error: toastError } = useToast();

type TabKey = "combats" | "players" | "social" | "tournament";
const activeTab = ref<TabKey>("combats");

const tabs = [
  { key: "combats", label: "Combats", icon: "⚔️" },
  { key: "players", label: "Stats joueurs", icon: "📊" },
  { key: "social", label: "Social", icon: "👥" },
  { key: "tournament", label: "Tournoi", icon: "🏆" },
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
  <div class="coude-hub page--constrained">
    <h1 class="page-title">⚔️ Coup de Coude</h1>

    <AppTabs
      :model-value="activeTab"
      :tabs="tabs"
      class="hub-tabs-wrap"
      @update:model-value="(k) => (activeTab = k as TabKey)"
    />

    <div class="tab-content">
      <AdminPageShell
        v-if="activeTab === 'combats' || activeTab === 'players'"
        title="Coup de Coude"
        icon="⚔️"
      >
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

        <CoudeCombatsTab v-if="activeTab === 'combats'" ref="combatsTabRef" />
        <CoudePlayersTab v-else ref="playersTabRef" />
      </AdminPageShell>

      <CoudeSocialPage v-else-if="activeTab === 'social'" />
      <TournamentPage v-else-if="activeTab === 'tournament'" />
    </div>

    <p class="muted small footer-hint">
      Les railleries automatiques (Coude + Blackjack) sont configurées sur
      la page <router-link to="/taunts">Railleries</router-link> — canal
      partagé entre les deux jeux.
    </p>
  </div>
</template>

<style scoped>
.coude-hub h1 { margin: 0 0 18px 0; font-size: 24px; }

.hub-tabs-wrap { margin-bottom: 20px; }

.tab-content { min-height: 200px; }

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

.footer-hint {
  margin-top: 30px;
  padding-top: 16px;
  border-top: 1px solid var(--border);
  text-align: center;
}
.footer-hint a { color: var(--accent); text-decoration: none; }
.footer-hint a:hover { text-decoration: underline; }
.muted { color: var(--text-secondary); }
.small { font-size: 12px; }
</style>
