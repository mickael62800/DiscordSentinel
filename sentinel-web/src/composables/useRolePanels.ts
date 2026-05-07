import { ref, onMounted, watch } from "vue";
import type { RolePanel, RolePanelDetail, AutoRoleConfig } from "../types";
import { useGuildSelector } from "./useGuildSelector";
import { useToast } from "./useToast";
import {
  rolePanelsService,
  type CreateRolePanelPayload,
  type CreateAutoRolePayload,
} from "@/services/rolePanelsService";

export function useRolePanels() {
  const { success, error: showError } = useToast();
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
        rolePanelsService.getAll(guildId),
        rolePanelsService.getAutoRoles(guildId),
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
      selectedPanel.value = await rolePanelsService.getDetail(panelId);
    } catch (e) {
      console.error("Erreur lors du chargement du detail du panneau :", e);
      showError("Erreur lors du chargement du detail du panneau.");
    }
  }

  // ── CRUD Phase 7 ────────────────────────────────────────────
  async function createPanel(payload: CreateRolePanelPayload) {
    try {
      const detail = await rolePanelsService.create(payload);
      await fetchAll();
      success("Panel créé.");
      return detail;
    } catch (e) {
      console.error("Erreur création panel :", e);
      showError("Erreur lors de la création du panel.");
      throw e;
    }
  }

  async function deletePanel(panelId: string) {
    try {
      await rolePanelsService.remove(panelId);
      panels.value = panels.value.filter((p) => p.id !== panelId);
      if (selectedPanel.value?.panel.id === panelId) selectedPanel.value = null;
      success("Panel supprimé.");
    } catch (e) {
      console.error("Erreur suppression panel :", e);
      showError("Erreur lors de la suppression du panel.");
    }
  }

  async function addAutoRole(payload: CreateAutoRolePayload) {
    try {
      const role = await rolePanelsService.addAutoRole(payload);
      autoRoles.value.push(role);
      success("Auto-role ajouté.");
    } catch (e) {
      console.error("Erreur ajout auto-role :", e);
      showError("Erreur lors de l'ajout de l'auto-role.");
      throw e;
    }
  }

  async function removeAutoRole(guildId: string, roleId: string) {
    try {
      await rolePanelsService.removeAutoRole(guildId, roleId);
      autoRoles.value = autoRoles.value.filter((r) => r.role_id !== roleId);
      success("Auto-role supprimé.");
    } catch (e) {
      console.error("Erreur suppression auto-role :", e);
      showError("Erreur lors de la suppression.");
    }
  }

  onMounted(fetchAll);
  watch(selectedGuildId, fetchAll);

  return {
    panels,
    autoRoles,
    selectedPanel,
    loading,
    fetchAll,
    selectPanel,
    createPanel,
    deletePanel,
    addAutoRole,
    removeAutoRole,
  };
}
