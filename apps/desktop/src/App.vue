<script setup lang="ts">
import { useRoute } from "vue-router";
import MainLayout from "./components/templates/MainLayout.vue";

const route = useRoute();
</script>

<template>
  <!-- Login page: no sidebar layout -->
  <router-view v-if="route.meta.public" />

  <!-- Authenticated pages: sidebar layout -->
  <MainLayout v-else>
    <router-view />
  </MainLayout>
</template>
