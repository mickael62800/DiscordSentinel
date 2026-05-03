<script setup lang="ts">
import { ref } from "vue";
import { useTauntsConfig } from "@/composables/useTauntsConfig";
import { useConfirm } from "@/composables/useConfirm";
import AppButton from "@/components/atoms/AppButton.vue";
import EmptyState from "@/components/atoms/EmptyState.vue";

const { config, removeOptOut } = useTauntsConfig();
const { confirm } = useConfirm();

const removing = ref<string | null>(null);

async function onRemove(userId: string) {
  const ok = await confirm({
    title: "Retirer l'opt-out",
    message: `Forcer la reactivation des railleries pour l'utilisateur ${userId} ? Il pourra de nouveau ere raille automatiquement jusqu'a ce qu'il refasse /no-taunts on.`,
  });
  if (!ok) return;
  removing.value = userId;
  try {
    await removeOptOut(userId);
  } finally {
    removing.value = null;
  }
}
</script>

<template>
  <div v-if="config" class="card card--lg opt-outs-card">
    <h2>Joueurs opt-out ({{ config.opt_outs.length }})</h2>
    <p class="hint">
      Ces joueurs ont tape <code>/no-taunts on</code>. Tu peux forcer le
      retrait de leur opt-out ci-dessous (ils devront re-taper la commande
      pour se re-proteger).
    </p>

    <EmptyState v-if="config.opt_outs.length === 0" message="Aucun joueur n'a opt-out." />

    <ul v-else class="opt-outs-list">
      <li v-for="userId in config.opt_outs" :key="userId">
        <span class="user-id">{{ userId }}</span>
        <AppButton variant="danger" :disabled="removing === userId" @click="onRemove(userId)">
          {{ removing === userId ? "Retrait…" : "Retirer" }}
        </AppButton>
      </li>
    </ul>
  </div>
</template>

<style scoped>
.opt-outs-card {
  display: flex;
  flex-direction: column;
  gap: var(--space-lg);
}
.opt-outs-card h2 { margin: 0 0 var(--space-xs); font-size: 18px; }
.hint {
  margin: 0;
  font-size: 13px;
  color: var(--text-secondary);
  line-height: 1.5;
}
.hint code {
  background: var(--bg-elevated);
  padding: 2px 6px;
  border-radius: 4px;
  font-size: 12px;
}
.opt-outs-list {
  list-style: none; padding: 0; margin: 0;
  display: flex; flex-direction: column; gap: 8px;
}
.opt-outs-list li {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 10px 14px;
  background: var(--bg-elevated);
  border-radius: 8px;
}
.user-id { font-family: var(--font-mono, monospace); font-size: 13px; }
</style>
