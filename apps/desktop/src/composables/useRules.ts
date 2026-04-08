import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { ModerationRule, UpdateRuleParams } from "../types";
import { useGuildFetch } from "./useGuildFetch";
import { useToast } from "./useToast";

export function useRules() {
  const { data: rules, loading, error, refresh: fetchRules } = useGuildFetch<ModerationRule[]>(
    "get_rules",
    [],
  );

  const { success, error: showError } = useToast();
  const editing = ref<ModerationRule | null>(null);

  async function toggleRule(rule: ModerationRule) {
    try {
      const newState = !rule.enabled;
      await invoke<boolean>("toggle_rule", { id: rule.id, enabled: newState });
      rule.enabled = newState;
      success(newState ? "Regle activee avec succes." : "Regle desactivee avec succes.");
    } catch (e) {
      console.error("Erreur lors du basculement de la regle :", e);
      showError("Erreur lors du basculement de la regle.");
    }
  }

  async function updateRule(params: UpdateRuleParams) {
    try {
      await invoke("update_rule", {
        guildId: params.guild_id,
        flagType: params.flag_type,
        weight: params.weight,
        thresholdWarn: params.threshold_warn,
        thresholdDelete: params.threshold_delete,
        thresholdMute: params.threshold_mute,
        thresholdBan: params.threshold_ban,
        enabled: params.enabled,
      });
      await fetchRules();
      success("Regle mise a jour avec succes.");
    } catch (e) {
      console.error("Erreur lors de la mise a jour de la regle :", e);
      showError("Erreur lors de la mise a jour de la regle.");
    }
  }

  function openEdit(rule: ModerationRule) {
    editing.value = { ...rule };
  }

  function closeEdit() {
    editing.value = null;
  }

  return { rules, loading, error, editing, fetchRules, toggleRule, updateRule, openEdit, closeEdit };
}
