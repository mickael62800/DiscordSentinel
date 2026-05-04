<script setup lang="ts">
import { useCoudeSocial } from "@/composables/useCoudeSocial";
import CoudeSocialLookup from "../organisms/CoudeSocialLookup.vue";
import CoudeSocialDetailsGrid from "../organisms/CoudeSocialDetailsGrid.vue";

const { lookupUserId, loading } = useCoudeSocial();
</script>

<template>
  <div class="page page--constrained">
    <header class="page-header">
      <h1>👊 Coude — Sous-systèmes sociaux</h1>
      <p class="lede">
        Vue admin sur les mécaniques toxic-fun du Coude : malédictions,
        primes collectives, coalitions et vendettas. Cherche par
        <code>user_id</code> Discord pour voir l'état complet d'un joueur.
      </p>
    </header>

    <CoudeSocialLookup />
    <CoudeSocialDetailsGrid v-if="lookupUserId && !loading" />
  </div>
</template>

<style scoped>
@import "./_moderation-advanced-shared.css";
</style>
