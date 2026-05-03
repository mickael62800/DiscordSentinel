<script setup lang="ts">
import RolePanelForm from "../organisms/RolePanelForm.vue";
import AutoRolesPanel from "../organisms/AutoRolesPanel.vue";
</script>

<template>
  <div class="page page--constrained">
    <header class="page-header">
      <h1>🎨 Nouveau panel de rôles</h1>
      <p class="lede">
        Crée un panneau de sélection de rôles auto-réactif. Une fois créé, déploie-le
        sur Discord avec <code>/roles-panel deploy panel_id:&lt;UUID&gt;</code>.
      </p>
      <router-link to="/role-panels" class="back-link">← Retour aux panels</router-link>
    </header>

    <RolePanelForm />
    <AutoRolesPanel />
  </div>
</template>

<style scoped>
@import "./_moderation-advanced-shared.css";
.back-link {
  display: inline-block;
  margin-top: 8px;
  color: var(--text-secondary);
  text-decoration: none;
  font-size: 0.9rem;
}
.back-link:hover { color: var(--accent); }
</style>
