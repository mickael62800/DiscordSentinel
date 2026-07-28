<script setup lang="ts">
import AdminPageShell from "@/components/layouts/AdminPageShell.vue";
import StrikesConfigForm from "../organisms/StrikesConfigForm.vue";
import StrikesUserLookup from "../organisms/StrikesUserLookup.vue";
</script>

<template>
  <AdminPageShell title="Strikes" icon="⚖️">
    <template #lede>
      Système d'escalade automatique : N strikes accumulés sur une fenêtre →
      action automatique (warn / mute / kick / ban). Les strikes expirent
      au bout de la fenêtre.
    </template>

    <StrikesConfigForm />
    <StrikesUserLookup />
  </AdminPageShell>
</template>
