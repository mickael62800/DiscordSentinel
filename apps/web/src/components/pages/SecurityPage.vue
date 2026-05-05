<script setup lang="ts">
import AdminPageShell from "../layouts/AdminPageShell.vue";
import SecurityStatsGrid from "../organisms/SecurityStatsGrid.vue";
import SecurityEventsList from "../organisms/SecurityEventsList.vue";
</script>

<template>
  <AdminPageShell title="Evenements de securite">
    <template #lede>
      Detection des raids, comptes suspects et actions massives
    </template>

    <SecurityStatsGrid />
    <SecurityEventsList />
  </AdminPageShell>
</template>
