<script setup lang="ts">
import type { TableColumn } from "../../types";
import LogsTemplate from "../organisms/LogsTemplate.vue";

const columns: TableColumn[] = [
  { key: "timestamp", label: "Heure" },
  { key: "level", label: "Niveau" },
  { key: "bot", label: "Bot" },
  { key: "message", label: "Message" },
  { key: "details", label: "Details" },
];
</script>

<template>
  <LogsTemplate
    title="Journaux Bots"
    category="bot"
    :columns="columns"
    empty-message="Aucun journal bot"
    show-source-filter
    source-label="Tous les bots"
    show-clear-button
    clear-confirm-message="Supprimer tous les journaux bots ?"
  />
</template>
