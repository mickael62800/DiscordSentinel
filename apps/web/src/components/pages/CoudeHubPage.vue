<script setup lang="ts">
import { ref } from "vue";
import CoudePage from "./CoudePage.vue";
import CoudeSocialPage from "./CoudeSocialPage.vue";
import TournamentPage from "./TournamentPage.vue";

type SubTab = "stats" | "social" | "tournament";
const activeTab = ref<SubTab>("stats");

const tabs: Array<{ key: SubTab; label: string; icon: string }> = [
  { key: "stats", label: "Stats", icon: "📊" },
  { key: "social", label: "Social", icon: "👥" },
  { key: "tournament", label: "Tournoi", icon: "🏆" },
];
</script>

<template>
  <div class="coude-hub page--constrained">
    <h1>⚔️ Coup de Coude</h1>

    <div class="hub-tabs">
      <button
        v-for="tab in tabs"
        :key="tab.key"
        :class="['hub-tab', { active: activeTab === tab.key }]"
        @click="activeTab = tab.key"
      >
        <span class="tab-icon">{{ tab.icon }}</span> {{ tab.label }}
      </button>
    </div>

    <div class="tab-content">
      <CoudePage v-if="activeTab === 'stats'" />
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

.hub-tabs {
  display: flex;
  gap: 4px;
  margin-bottom: 20px;
  border-bottom: 1px solid var(--border);
}
.hub-tab {
  background: transparent;
  border: 0;
  padding: 10px 18px;
  font-size: 14px;
  color: var(--text-secondary);
  cursor: pointer;
  border-bottom: 2px solid transparent;
  font-weight: 600;
  display: inline-flex;
  align-items: center;
  gap: 6px;
}
.hub-tab:hover { color: var(--text-primary); }
.hub-tab.active {
  color: var(--accent);
  border-bottom-color: var(--accent);
}
.tab-icon { font-size: 16px; }
.tab-content { min-height: 200px; }
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
