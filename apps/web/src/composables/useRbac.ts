import { onMounted, ref, watch, type Ref } from "vue";
import type { GuildUserRole, MyRole, RbacRole } from "../types";
import { useGuildSelector } from "./useGuildSelector";
import { useToast } from "./useToast";
import { rbacService } from "@/services/rbacService";

export function useRbac() {
  const { success: toastSuccess, error: toastError } = useToast();
  const { selectedGuildId } = useGuildSelector();

  const users: Ref<GuildUserRole[]> = ref([]);
  const myRole: Ref<MyRole | null> = ref(null);
  const loading = ref(false);
  const error = ref<string | null>(null);

  async function refresh() {
    const guildId = selectedGuildId.value;
    if (!guildId) {
      users.value = [];
      myRole.value = null;
      error.value = null;
      return;
    }

    loading.value = true;
    error.value = null;
    try {
      const [list, me] = await Promise.all([
        rbacService.listGuildUsers(guildId),
        rbacService.getMyRole(guildId).catch(() => null),
      ]);
      users.value = list;
      myRole.value = me;
    } catch (e) {
      error.value = String(e);
      toastError(`Echec chargement RBAC : ${e}`);
    } finally {
      loading.value = false;
    }
  }

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

  onMounted(() => { void refresh(); });
  watch(selectedGuildId, () => { void refresh(); });

  return { users, myRole, loading, error, refresh, grantRole, updateRole, revokeRole };
}
