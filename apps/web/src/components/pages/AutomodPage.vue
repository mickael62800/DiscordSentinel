<script setup lang="ts">
import AutomodReviewsList from "@/components/organisms/AutomodReviewsList.vue";
import AutomodStatsGrid from "@/components/organisms/AutomodStatsGrid.vue";
import AutomodDetectionsTimeline from "@/components/organisms/AutomodDetectionsTimeline.vue";
</script>

<template>
  <div class="automod-page">
    <header class="page-header">
      <h1>🤖 Automod</h1>
      <p class="lede">
        Timeline des détections automatiques (spam, insultes, liens, phishing,
        mentions massives, fichiers suspects, unicode invisible…). La
        configuration des seuils et des détecteurs se fait dans
        <router-link to="/component-config">Component config</router-link> →
        <em>automod-bot</em>.
      </p>
    </header>

    <AutomodReviewsList />
    <AutomodStatsGrid />
    <AutomodDetectionsTimeline />
  </div>
</template>

<style scoped>
.automod-page {
  max-width: 1200px;
  margin: 0 auto;
  padding: 24px;
}
.page-header { margin-bottom: 24px; }
.page-header h1 {
  margin: 0 0 8px 0;
  font-size: 1.6rem;
}
.lede {
  color: var(--text-secondary);
  margin: 0;
}
.lede a {
  color: var(--accent);
  text-decoration: none;
}
.lede a:hover { text-decoration: underline; }
</style>
