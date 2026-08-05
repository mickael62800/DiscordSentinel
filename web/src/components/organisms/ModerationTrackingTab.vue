<script setup lang="ts">
import AppButton from "../atoms/AppButton.vue";
import { ref, watch } from "vue";
import AppTabs from "../molecules/AppTabs.vue";
import StrikesPage from "../pages/StrikesPage.vue";
import NotesPage from "../pages/NotesPage.vue";
import EvidencePage from "../pages/EvidencePage.vue";
import { useSharedUserLookup } from "@/composables/useSharedUserLookup";

const props = defineProps<{
  /** Increment a chaque demande externe (Journal) d'ouvrir Notes & Preuves. */
  jumpToNotesEvidence?: number;
}>();

const subTab = ref<"strikes" | "notes-evidence">("strikes");

const subTabs = [
  { key: "strikes", label: "Avertissements" },
  { key: "notes-evidence", label: "Notes & Preuves" },
];

const { sharedUserId } = useSharedUserLookup();

watch(
  () => props.jumpToNotesEvidence,
  (v) => {
    if (v !== undefined) subTab.value = "notes-evidence";
  },
);
</script>

<template>
  <div>
    <AppTabs
      :model-value="subTab"
      :tabs="subTabs"
      variant="plain"
      class="sub-tabs-wrap"
      @update:model-value="(k) => (subTab = k as typeof subTab)"
    />

    <StrikesPage v-if="subTab === 'strikes'" />

    <div v-else-if="subTab === 'notes-evidence'">
      <section class="card shared-user-bar">
        <h2>👤 Utilisateur ciblé</h2>
        <p class="muted small">
          Saisis un ID Discord — les notes et les preuves de modération
          de cet utilisateur s'affichent dans les deux panneaux ci-dessous.
        </p>
        <div class="lookup">
          <input
            v-model="sharedUserId"
            placeholder="ID Discord de l'utilisateur (ex: 123456789012345678)"
            type="text"
          />
          <AppButton variant="secondary" v-if="sharedUserId" @click="sharedUserId = ''">
            Effacer
          </AppButton>
        </div>
      </section>

      <div v-if="sharedUserId.trim()" class="stacked-sections">
        <section class="stacked-block">
          <h2 class="stacked-title">📝 Notes de modération</h2>
          <p class="stacked-hint">Notes libres attachées à cet utilisateur (visibles entre modérateurs).</p>
          <NotesPage embedded />
        </section>
        <section class="stacked-block">
          <h2 class="stacked-title">📎 Preuves</h2>
          <p class="stacked-hint">Pièces jointes (URL/description) attachées à une action de modération précise.</p>
          <EvidencePage embedded />
        </section>
      </div>
      <div v-else class="empty-shared">
        Saisis un ID utilisateur ci-dessus pour voir ses notes et preuves.
      </div>
    </div>
  </div>
</template>

<style scoped>
.sub-tabs-wrap { margin-bottom: 16px; }

.shared-user-bar { margin-bottom: 20px; }
.shared-user-bar .lookup { display: flex; gap: 10px; margin-top: 12px; }
.shared-user-bar .lookup input {
  flex: 1;
  padding: 10px 14px;
  border-radius: var(--radius-md);
  border: 1px solid var(--border);
  background: var(--bg-card);
  color: var(--text-primary);
  font-size: 14px;
  font-family: "JetBrains Mono", monospace;
}
.shared-user-bar .lookup input:focus { outline: 1px solid var(--accent); border-color: var(--accent); }

.empty-shared {
  padding: 60px 20px;
  text-align: center;
  color: var(--text-secondary);
  font-style: italic;
  border: 1px dashed var(--border);
  border-radius: var(--radius-md);
}

.stacked-sections {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 24px;
  align-items: start;
}
@media (max-width: 1100px) {
  .stacked-sections { grid-template-columns: 1fr; }
}
.stacked-block {
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  padding: 20px;
  background: var(--bg-secondary);
}
.stacked-title {
  font-size: 16px;
  font-weight: 700;
  margin: 0 0 6px 0;
  color: var(--text-primary);
}
.stacked-hint {
  font-size: 12px;
  color: var(--text-secondary);
  margin: 0 0 14px 0;
}
</style>
