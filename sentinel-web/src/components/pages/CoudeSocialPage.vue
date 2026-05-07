<script setup lang="ts">
import { useCoudeSocial } from "@/composables/useCoudeSocial";
import AdminPageShell from "@/components/layouts/AdminPageShell.vue";
import CoudeSocialLookup from "../organisms/CoudeSocialLookup.vue";
import CoudeSocialDetailsGrid from "../organisms/CoudeSocialDetailsGrid.vue";

const { lookupUserId, loading } = useCoudeSocial();
</script>

<template>
  <AdminPageShell title="Coude — Sous-systèmes sociaux" icon="👊">
    <template #lede>
      Vue admin sur les mécaniques toxic-fun du Coude : malédictions,
      primes collectives, coalitions et vendettas. Cherche par
      <code>user_id</code> Discord pour voir l'état complet d'un joueur.
    </template>

    <CoudeSocialLookup />
    <CoudeSocialDetailsGrid v-if="lookupUserId && !loading" />
  </AdminPageShell>
</template>

<style scoped>
@import "./_admin-page-shared.css";
</style>
