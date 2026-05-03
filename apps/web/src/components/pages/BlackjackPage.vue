<script setup lang="ts">
import { ref } from "vue";
import { blackjackService } from "@/services/blackjackService";
import { useGuildSelector } from "../../composables/useGuildSelector";
import { useConfirm } from "../../composables/useConfirm";
import { useToast } from "../../composables/useToast";
import { useComponentVisibility } from "@/composables/useComponentVisibility";
import AppButton from "../atoms/AppButton.vue";
import BlackjackTablesSection from "../organisms/BlackjackTablesSection.vue";
import BlackjackGamesSection from "../organisms/BlackjackGamesSection.vue";

const { visible } = useComponentVisibility();
const { selectedGuildId } = useGuildSelector();
const { confirm } = useConfirm();
const { success, error: toastError } = useToast();

const tablesRef = ref<InstanceType<typeof BlackjackTablesSection> | null>(null);
const gamesRef = ref<InstanceType<typeof BlackjackGamesSection> | null>(null);
const purging = ref(false);

function refreshAll() {
  tablesRef.value?.refresh();
  gamesRef.value?.refresh();
}

async function handlePurgeAll() {
  if (!selectedGuildId.value) return;
  const ok1 = await confirm({
    title: "Reset total Blackjack",
    message: "Supprimer DEFINITIVEMENT toutes les parties et tables blackjack pour cette guild ?",
  });
  if (!ok1) return;
  const ok2 = await confirm({
    title: "Confirmation finale",
    message: "Cette action est IRREVERSIBLE. Aucun remboursement ne sera effectue.",
  });
  if (!ok2) return;
  purging.value = true;
  try {
    const res = await blackjackService.purgeAll(selectedGuildId.value);
    success(`${res.deleted_games} partie(s) et ${res.deleted_tables} table(s) supprimee(s).`);
    refreshAll();
  } catch (e) {
    toastError(String(e));
  } finally {
    purging.value = false;
  }
}
</script>

<template>
  <div class="blackjack-page page--wide">
    <header class="hero">
      <div class="hero-text">
        <h1 class="hero-title">
          <span class="hero-icon">🎰</span>
          Blackjack
        </h1>
        <p class="hero-subtitle">
          Administration des parties — surveillance, historique, annulation avec remboursement
        </p>
      </div>
      <div class="hero-actions">
        <AppButton variant="secondary" @click="refreshAll">↻ Rafraichir</AppButton>
        <button
          v-if="visible('db.purge.blackjack')"
          class="danger-btn"
          :disabled="purging"
          @click="handlePurgeAll"
          title="Supprime DEFINITIVEMENT toutes les parties blackjack de cette guild (owner uniquement)"
        >
          {{ purging ? "Purge…" : "🗑 Reset total" }}
        </button>
      </div>
    </header>

    <BlackjackTablesSection ref="tablesRef" />
    <BlackjackGamesSection ref="gamesRef" />
  </div>
</template>

<style scoped>
.blackjack-page {
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
