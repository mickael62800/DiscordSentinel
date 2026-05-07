<script setup lang="ts">
import AdminPageShell from "@/components/layouts/AdminPageShell.vue";
import AutomodReviewsList from "@/components/organisms/AutomodReviewsList.vue";
import AutomodStatsGrid from "@/components/organisms/AutomodStatsGrid.vue";
import AutomodDetectionsTimeline from "@/components/organisms/AutomodDetectionsTimeline.vue";
</script>

<template>
  <AdminPageShell title="Automod" icon="🤖">
    <template #lede>
      Timeline des détections automatiques (spam, insultes, liens, phishing,
      mentions massives, fichiers suspects, unicode invisible…). La
      configuration des seuils et des détecteurs se fait dans
      <router-link to="/component-config">Component config</router-link> →
      <em>automod-bot</em>.
    </template>

    <AutomodReviewsList />
    <AutomodStatsGrid />
    <AutomodDetectionsTimeline />
  </AdminPageShell>
</template>
