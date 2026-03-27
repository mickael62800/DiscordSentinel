import { ref, onMounted, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { RolePanel, RolePanelDetail, AutoRoleConfig } from "../types";
import { useGuildSelector } from "./useGuildSelector";

export function useRolePanels() {
  const panels = ref<RolePanel[]>([]);
  const autoRoles = ref<AutoRoleConfig[]>([]);
  const selectedPanel = ref<RolePanelDetail | null>(null);
  const loading = ref(true);
  const { selectedGuildId } = useGuildSelector();

  async function fetchAll() {
    const guildId = selectedGuildId.value;
    if (!guildId) {
      panels.value = [];
      autoRoles.value = [];
      loading.value = false;
      return;
    }
    loading.value = true;
    try {
      const [p, ar] = await Promise.all([
        invoke<RolePanel[]>("get_role_panels", { guildId }),
        invoke<AutoRoleConfig[]>("get_auto_roles", { guildId }),
      ]);
      panels.value = p;
      autoRoles.value = ar;
    } catch (e) {
      console.error("Erreur chargement role panels:", e);
    } finally {
      loading.value = false;
    }
  }

  async function selectPanel(panelId: string) {
    try {
      selectedPanel.value = await invoke<RolePanelDetail>("get_role_panel_detail", { panelId });
    } catch (e) {
      console.error("Erreur chargement panel detail:", e);
    }
  }

  onMounted(fetchAll);
  watch(selectedGuildId, fetchAll);

  return { panels, autoRoles, selectedPanel, loading, fetchAll, selectPanel };
}
