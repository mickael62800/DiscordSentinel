<script setup lang="ts">
import { ref } from "vue";
import IdeasListPanel from "../organisms/IdeasListPanel.vue";
import IdeaDetailPanel from "../organisms/IdeaDetailPanel.vue";

const selectedId = ref<string | null>(null);

function selectIdea(id: string) {
  selectedId.value = id;
}

function backToList() {
  selectedId.value = null;
}
</script>

<template>
  <div class="ideas page--constrained">
    <IdeasListPanel v-if="!selectedId" @select="selectIdea" />
    <IdeaDetailPanel v-else :idea-id="selectedId" @back="backToList" />
  </div>
</template>
