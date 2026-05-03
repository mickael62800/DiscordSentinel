<script setup lang="ts">
import StrikesConfigForm from "../organisms/StrikesConfigForm.vue";
import StrikesUserLookup from "../organisms/StrikesUserLookup.vue";
</script>

<template>
  <div class="strikes-page">
    <header class="page-header">
      <h1>⚖️ Strikes</h1>
      <p class="lede">
        Système d'escalade automatique : N strikes accumulés sur une fenêtre →
        action automatique (warn / mute / kick / ban). Les strikes expirent
        au bout de la fenêtre.
      </p>
    </header>

    <StrikesConfigForm />
    <StrikesUserLookup />
  </div>
</template>

<style scoped>
.strikes-page { max-width: 1100px; margin: 0 auto; padding: 24px; }
.page-header { margin-bottom: 24px; }
.page-header h1 { margin: 0 0 8px 0; font-size: 1.6rem; }
.lede { color: var(--text-secondary); margin: 0; }
</style>
