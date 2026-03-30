<script setup lang="ts">
import type { TableColumn } from "../../types";
import LogsTemplate from "../organisms/LogsTemplate.vue";

const columns: TableColumn[] = [
  { key: "timestamp", label: "Heure" },
  { key: "level", label: "Niveau" },
  { key: "bot", label: "Source" },
  { key: "server", label: "Serveur" },
  { key: "message", label: "Message" },
];
</script>

<template>
  <LogsTemplate
    title="Journaux Discord"
    category="discord"
    :columns="columns"
    empty-message="Aucun journal Discord"
    show-source-filter
    source-label="Toutes les sources"
  />
</template>
