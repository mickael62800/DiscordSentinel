<script setup lang="ts">
import { ref } from "vue";
import TicketsListPanel from "../organisms/TicketsListPanel.vue";
import TicketDetailPanel from "../organisms/TicketDetailPanel.vue";

const selectedId = ref<string | null>(null);

function selectTicket(id: string) {
  selectedId.value = id;
}

function backToList() {
  selectedId.value = null;
}
</script>

<template>
  <div class="tickets page--constrained">
    <TicketsListPanel v-if="!selectedId" @select="selectTicket" />
    <TicketDetailPanel v-else :ticket-id="selectedId" @back="backToList" />
  </div>
</template>
