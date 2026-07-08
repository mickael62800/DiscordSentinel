<script setup lang="ts">
import { ref } from "vue";
import { useSharedUserLookup } from "../../composables/useSharedUserLookup";
import AppTabs from "../molecules/AppTabs.vue";
import ModerationJournalTab from "../organisms/ModerationJournalTab.vue";
import ModerationBansTab from "../organisms/ModerationBansTab.vue";
import ModerationTrackingTab from "../organisms/ModerationTrackingTab.vue";
import ReviewPage from "./ReviewPage.vue";
import RemindersPage from "./RemindersPage.vue";

type TabKey = "journal" | "bans" | "tracking" | "review" | "reminders";

const activeTab = ref<TabKey>("journal");

const hubTabs = [
  { key: "journal", label: "Journal" },
  { key: "bans", label: "Bannis actifs" },
  { key: "tracking", label: "Suivi utilisateur" },
  { key: "review", label: "Revue AutoMod" },
  { key: "reminders", label: "Rappels" },
];

const { sharedUserId } = useSharedUserLookup();

// Incremente a chaque demande du Journal d'ouvrir Notes & Preuves : permet
// au TrackingTab de basculer son sous-onglet sans coupler les deux organismes.
const trackingJumpSignal = ref(0);

function handleOpenNotesEvidence(userId: string) {
  if (!userId) return;
  sharedUserId.value = userId;
  activeTab.value = "tracking";
  trackingJumpSignal.value++;
}
</script>

<template>
  <div class="moderation-hub page--constrained">
    <h1 class="page-title">Moderation</h1>

    <AppTabs
      :model-value="activeTab"
      :tabs="hubTabs"
      class="hub-tabs-wrap"
      @update:model-value="(k) => (activeTab = k as TabKey)"
    />

    <div class="tab-content">
      <ModerationJournalTab
        v-if="activeTab === 'journal'"
        @open-notes-evidence="handleOpenNotesEvidence"
      />
      <ModerationBansTab v-else-if="activeTab === 'bans'" />
      <ModerationTrackingTab
        v-else-if="activeTab === 'tracking'"
        :jump-to-notes-evidence="trackingJumpSignal"
      />
      <ReviewPage v-else-if="activeTab === 'review'" />
      <RemindersPage v-else-if="activeTab === 'reminders'" />
    </div>
  </div>
</template>

<style scoped>
.moderation-hub h1 {
  margin-bottom: 18px;
  font-size: 1.6rem;
  font-weight: 700;
  background: linear-gradient(
    90deg,
    var(--text-primary) 0%,
    color-mix(in srgb, var(--accent) 60%, var(--text-primary)) 50%,
    var(--text-primary) 100%
  );
  background-size: 200% auto;
  -webkit-background-clip: text;
  background-clip: text;
  -webkit-text-fill-color: transparent;
  color: transparent;
  animation: mod-title-shimmer 10s linear infinite;
  letter-spacing: 0.3px;
}
@keyframes mod-title-shimmer {
  0%   { background-position: 200% center; }
  100% { background-position: -200% center; }
}

.hub-tabs-wrap { margin-bottom: 24px; }

.tab-content { animation: fadeSlideIn 0.3s ease-out; }
@keyframes fadeSlideIn {
  from { opacity: 0; transform: translateY(6px); }
  to   { opacity: 1; transform: translateY(0); }
}

@media (prefers-reduced-motion: reduce) {
  .moderation-hub h1 {
    animation: none;
    background: none;
    -webkit-text-fill-color: var(--text-primary);
    color: var(--text-primary);
  }
  .tab-content { animation: none; }
}
</style>
