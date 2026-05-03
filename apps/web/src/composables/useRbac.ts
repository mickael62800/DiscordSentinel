import { ref, watch, type Ref } from "vue";
import type { GuildUserRole, RbacRole } from "../types";
import { useGuildSelector } from "./useGuildSelector";
import { useMyRole } from "./useMyRole";
import { useToast } from "./useToast";
import { rbacService } from "@/services/rbacService";

// Singleton module-scoped : un seul cache partage entre la page et les organisms.
const { selectedGuildId } = useGuildSelector();
const { myRole, reload: reloadMyRole } = useMyRole();

const users: Ref<GuildUserRole[]> = ref([]);
const loading = ref(false);
const error = ref<string | null>(null);

async function refresh() {
  const { error: toastError } = useToast();
  const guildId = selectedGuildId.value;
  if (!guildId) {
    users.value = [];
    error.value = null;
    return;
  }

  loading.value = true;
  error.value = null;
  try {
    const [list] = await Promise.all([
      rbacService.listGuildUsers(guildId),
      reloadMyRole(),
    ]);
    users.value = list;
  } catch (e) {
    error.value = String(e);
    toastError(`Echec chargement RBAC : ${e}`);
  } finally {
    loading.value = false;
  }
}

watch(selectedGuildId, refresh, { immediate: true });

export function useRbac() {
  const { success: toastSuccess, error: toastError } = useToast();

  async function grantRole(userId: string, role: RbacRole, displayName?: string): Promise<boolean> {
    const guildId = selectedGuildId.value;
    if (!guildId) return false;
    try {
      await rbacService.grantRole(guildId, userId, role, displayName ?? null);
      toastSuccess(`Role ${role} attribue`);
      await refresh();
      return true;
    } catch (e) {
      toastError(`Echec grant : ${e}`);
      return false;
    }
  }

  async function updateRole(userId: string, role: RbacRole): Promise<boolean> {
    const guildId = selectedGuildId.value;
    if (!guildId) return false;
    try {
      await rbacService.updateRole(guildId, userId, role);
      toastSuccess(`Role change en ${role}`);
      await refresh();
      return true;
    } catch (e) {
      toastError(`Echec update : ${e}`);
      return false;
    }
  }

  async function revokeRole(userId: string): Promise<boolean> {
    const guildId = selectedGuildId.value;
    if (!guildId) return false;
    try {
      await rbacService.revokeRole(guildId, userId);
      toastSuccess("Role revoque");
      await refresh();
      return true;
    } catch (e) {
      toastError(`Echec revoke : ${e}`);
      return false;
    }
  }

  return { users, myRole, loading, error, refresh, grantRole, updateRole, revokeRole };
}
