import { ref, onMounted, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { RolePanel, RolePanelDetail, AutoRoleConfig } from "../types";
import { useGuildSelector } from "./useGuildSelector";
import { useToast } from "./useToast";

export function useRolePanels() {
  const { error: showError } = useToast();
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
      console.error("Erreur lors du chargement des panneaux de roles :", e);
      showError("Erreur lors du chargement des panneaux de roles.");
    } finally {
      loading.value = false;
    }
  }

  async function selectPanel(panelId: string) {
    try {
      selectedPanel.value = await invoke<RolePanelDetail>("get_role_panel_detail", { panelId });
    } catch (e) {
      console.error("Erreur lors du chargement du detail du panneau :", e);
      showError("Erreur lors du chargement du detail du panneau.");
    }
  }

  onMounted(fetchAll);
  watch(selectedGuildId, fetchAll);

  return { panels, autoRoles, selectedPanel, loading, fetchAll, selectPanel };
}
