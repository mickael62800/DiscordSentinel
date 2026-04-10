import { onMounted, ref, watch, type Ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { GuildUserRole, MyRole, RbacRole } from "../types";
import { useGuildSelector } from "./useGuildSelector";
import { useToast } from "./useToast";

/**
 * Phase 7 B — Gestion RBAC fin cote desktop.
 *
 * Wrap les 5 commandes Tauri rbac_* et expose :
 *  - `users` : liste des users+roles de la guild courante (re-fetch sur changement de guild)
 *  - `myRole` : role effectif du caller (pour UI gating)
 *  - `refresh()` / `grantRole()` / `updateRole()` / `revokeRole()`
 *
 * Contrairement a `useGuildFetch`, on n'utilise pas le helper generique car
 * les endpoints RBAC prennent `guild_id: String` (obligatoire, pas Option).
 * Le composable no-op proprement quand aucune guild n'est selectionnee.
 */
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
      // 2 appels parallèles — la liste des users ET mon role
      const [list, me] = await Promise.all([
        invoke<GuildUserRole[]>("rbac_list_guild_users", { guildId }),
        invoke<MyRole>("rbac_get_my_role", { guildId }).catch(() => null),
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

  async function grantRole(
    userId: string,
    role: RbacRole,
    displayName?: string,
  ): Promise<boolean> {
    const guildId = selectedGuildId.value;
    if (!guildId) return false;
    try {
      await invoke("rbac_grant_role", {
        guildId,
        userId,
        role,
        displayName: displayName ?? null,
      });
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
      await invoke("rbac_update_role", { guildId, userId, role });
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
      await invoke("rbac_revoke_role", { guildId, userId });
      toastSuccess("Role revoque");
      await refresh();
      return true;
    } catch (e) {
      toastError(`Echec revoke : ${e}`);
      return false;
    }
  }

  // Initial fetch au mount : le watch ne se declenche PAS si selectedGuildId
  // a deja une valeur (depuis localStorage) au moment du mount. Sans ce
  // onMounted, la page RBAC reste vide jusqu'a ce que l'user change de guild.
  onMounted(() => {
    void refresh();
  });

  // Refresh auto a chaque changement de guild
  watch(selectedGuildId, () => {
    void refresh();
  });

  return {
    users,
    myRole,
    loading,
    error,
    refresh,
    grantRole,
    updateRole,
    revokeRole,
  };
}
