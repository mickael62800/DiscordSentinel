<script setup lang="ts">
import type { TableColumn } from "../../types";
import LogsTemplate from "../organisms/LogsTemplate.vue";

const columns: TableColumn[] = [
  { key: "timestamp", label: "Heure" },
  { key: "level", label: "Niveau" },
  { key: "bot", label: "Worker" },
  { key: "message", label: "Message" },
  { key: "details", label: "Details" },
];
</script>

<template>
  <LogsTemplate
    title="Journaux Workers"
    category="worker"
    :columns="columns"
    empty-message="Aucun journal worker"
    show-source-filter
    source-label="Tous les workers"
    show-clear-button
    clear-confirm-message="Supprimer tous les journaux workers ?"
  />
</template>
