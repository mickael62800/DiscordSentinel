<script setup lang="ts">
import AdminPageShell from "../layouts/AdminPageShell.vue";
import RolePanelForm from "../organisms/RolePanelForm.vue";
import AutoRolesPanel from "../organisms/AutoRolesPanel.vue";
</script>

<template>
  <AdminPageShell title="Nouveau panel de rôles" icon="🎨">
    <template #lede>
      Crée un panneau de sélection de rôles auto-réactif. Une fois créé, déploie-le
      sur Discord avec <code>/roles-panel deploy panel_id:&lt;UUID&gt;</code>.
    </template>
    <template #actions>
      <router-link to="/role-panels" class="back-link">← Retour aux panels</router-link>
    </template>

    <RolePanelForm />
    <AutoRolesPanel />
  </AdminPageShell>
</template>

<style scoped>
@import "./_admin-page-shared.css";
.back-link {
  color: var(--text-secondary);
  text-decoration: none;
  font-size: 0.9rem;
}
.back-link:hover { color: var(--accent); }
</style>
