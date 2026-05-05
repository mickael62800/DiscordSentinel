<script setup lang="ts">
import { computed } from "vue";
import { useRbac } from "@/composables/useRbac";
import { useGuildSelector } from "@/composables/useGuildSelector";
import EmptyState from "../atoms/EmptyState.vue";
import AppBadge from "../atoms/AppBadge.vue";
import AdminPageShell from "../layouts/AdminPageShell.vue";
import RbacUsersTable from "../organisms/RbacUsersTable.vue";
import ComponentVisibilityGrid from "../organisms/ComponentVisibilityGrid.vue";
import ComponentMinRoleGrid from "../organisms/ComponentMinRoleGrid.vue";
import InvitationsManager from "../organisms/InvitationsManager.vue";
import type { RbacRole } from "@/types";

const { myRole } = useRbac();
const { selectedGuild } = useGuildSelector();

const canEdit = computed(() => myRole.value?.role === "owner");
const canView = computed(() => {
  const r = myRole.value?.role;
  return r === "owner" || r === "admin";
});

type BadgeVariant = "default" | "success" | "error" | "info" | "warning" | "danger";
function roleVariant(role: RbacRole): BadgeVariant {
  switch (role) {
    case "owner": return "danger";
    case "admin": return "warning";
    case "moderator": return "info";
    case "viewer": return "default";
  }
}
</script>

<template>
  <AdminPageShell title="Gestion RBAC" icon="🔐">
    <template #lede>
      Gerez les roles applicatifs des utilisateurs pour
      <strong>{{ selectedGuild?.name ?? "cette guild" }}</strong>
    </template>
    <template v-if="myRole" #actions>
      <span class="my-role">
        Votre role :
        <AppBadge :label="myRole.role" :variant="roleVariant(myRole.role)" />
      </span>
    </template>

    <div v-if="!selectedGuild" class="no-guild">
      Selectionnez une guild dans la barre laterale pour gerer les acces.
    </div>

    <template v-else-if="!canView">
      <EmptyState
        icon="🔒"
        title="Acces refuse"
        message="Vous avez besoin du role admin ou owner pour voir la gestion RBAC."
      />
    </template>

    <template v-else>
      <RbacUsersTable />
      <InvitationsManager v-if="canEdit" />
      <ComponentMinRoleGrid v-if="canEdit" />
      <ComponentVisibilityGrid v-if="canEdit" />
    </template>
  </AdminPageShell>
</template>

<style scoped>
.my-role {
  display: inline-flex;
  align-items: center;
  gap: 0.5rem;
  font-size: 0.9rem;
  color: var(--text-secondary);
}
.no-guild {
  padding: 2rem;
  text-align: center;
  color: var(--text-secondary);
  font-style: italic;
}
</style>
