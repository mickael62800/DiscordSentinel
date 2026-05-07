<script setup lang="ts">
import { ref } from "vue";
import AppTabs from "../molecules/AppTabs.vue";
import ReviewPage from "../pages/ReviewPage.vue";
import RemindersPage from "../pages/RemindersPage.vue";

const subTab = ref<"review" | "reminders">("review");

const subTabs = [
  { key: "review", label: "Revue AutoMod" },
  { key: "reminders", label: "Rappels" },
];
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
    <ReviewPage v-if="subTab === 'review'" />
    <RemindersPage v-else-if="subTab === 'reminders'" />
  </div>
</template>

<style scoped>
.sub-tabs-wrap {
  margin-bottom: 16px;
}
</style>
