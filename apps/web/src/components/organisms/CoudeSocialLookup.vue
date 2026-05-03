<script setup lang="ts">
import { useCoudeSocial } from "@/composables/useCoudeSocial";

const { lookupUserId, loading, lookup } = useCoudeSocial();
</script>

<template>
  <section class="card">
    <h2>Recherche par utilisateur</h2>
    <div class="lookup">
      <input
        v-model="lookupUserId"
        placeholder="ID Discord du joueur"
        @keyup.enter="lookup"
      />
      <button class="btn-secondary" @click="lookup">Rechercher</button>
    </div>
    <div v-if="loading" class="loading">Chargement…</div>
  </section>
</template>

<style scoped>
@import "../pages/_moderation-advanced-shared.css";
</style>
