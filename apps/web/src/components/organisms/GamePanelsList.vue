<script setup lang="ts">
import type { GamePanel } from "@/services/gamesService";
import { useGuildSelector } from "@/composables/useGuildSelector";
import AppBadge from "@/components/atoms/AppBadge.vue";

defineProps<{
  panels: GamePanel[];
}>();

const { selectedGuildId } = useGuildSelector();

function jumpUrl(panel: GamePanel): string {
  const gid = selectedGuildId.value ?? "@me";
  return `https://discord.com/channels/${gid}/${panel.channel_id}/${panel.message_id}`;
}
</script>

<template>
  <section class="panels-section">
    <h2>Panels Discord</h2>
    <p class="hint">
      Les panels sont deployes dans Discord via la commande
      <code>/game-admin panel category:&lt;nom&gt;</code>. Ils affichent un dropdown
      qui assigne/retire automatiquement le role Discord associe a chaque jeu.
    </p>
    <div v-if="panels.length === 0" class="muted">
      Aucun panel deploye. Utilisez la commande ci-dessus dans Discord.
    </div>
    <div v-else class="panels-list">
      <div v-for="p in panels" :key="p.id" class="card panel-card">
        <div class="panel-head">
          <AppBadge :label="p.category ?? '(sans categorie)'" variant="info" />
          <a :href="jumpUrl(p)" target="_blank" rel="noopener" class="jump-link">
            Ouvrir dans Discord &rarr;
          </a>
        </div>
        <div class="panel-meta">
          <span>Salon : <code>{{ p.channel_id }}</code></span>
          <span>Message : <code>{{ p.message_id }}</code></span>
        </div>
      </div>
    </div>
  </section>
</template>

<style scoped>
.panels-section { margin-top: 32px; }
.panels-section h2 { font-size: 15px; font-weight: 600; margin-bottom: 10px; }

.hint { font-size: 13px; color: var(--text-secondary); margin-bottom: 14px; }
.hint code {
  background: var(--bg-secondary);
  padding: 2px 6px;
  border-radius: 4px;
  font-family: "JetBrains Mono", monospace;
  font-size: 12px;
}

.muted { color: var(--text-secondary); }

.panels-list {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
  gap: 12px;
}

.panel-card {
  border-radius: 10px;
  padding: 14px;
}

.panel-head {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 8px;
}

.jump-link {
  font-size: 12px;
  color: var(--accent);
  text-decoration: none;
}
.jump-link:hover { text-decoration: underline; }

.panel-meta {
  display: flex;
  flex-direction: column;
  gap: 4px;
  font-size: 11px;
  color: var(--text-secondary);
}
.panel-meta code { font-family: "JetBrains Mono", monospace; }
</style>
