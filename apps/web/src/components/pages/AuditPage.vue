<script setup lang="ts">
import AuditFilters from "../organisms/AuditFilters.vue";
import AuditEntriesList from "../organisms/AuditEntriesList.vue";
</script>

<template>
  <div class="audit page--xs">
    <h1>Journal d'audit</h1>
    <AuditFilters />
    <AuditEntriesList />
  </div>
</template>

<style scoped>
.audit h1 { margin-bottom: 20px; }
</style>
