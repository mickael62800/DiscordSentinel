<script setup lang="ts">
import { ref } from "vue";
import { blackjackService } from "@/services/blackjackService";
import { useGuildSelector } from "../../composables/useGuildSelector";
import { useConfirm } from "../../composables/useConfirm";
import { useToast } from "../../composables/useToast";
import { useComponentVisibility } from "@/composables/useComponentVisibility";
import AppButton from "../atoms/AppButton.vue";
import AdminPageShell from "../layouts/AdminPageShell.vue";
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
  <AdminPageShell title="Blackjack" icon="🎰">
    <template #lede>
      Administration des parties — surveillance, historique, annulation avec remboursement
    </template>
    <template #actions>
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
    </template>

    <BlackjackTablesSection ref="tablesRef" />
    <BlackjackGamesSection ref="gamesRef" />
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
</style>
