import { ref } from "vue";
import type { ModerationRule, UpdateRuleParams } from "../types";
import { useGuildFetch } from "./useGuildFetch";
import { useToast } from "./useToast";
import { rulesService } from "@/services/rulesService";

export function useRules() {
  const { data: rules, loading, error, refresh: fetchRules } = useGuildFetch<ModerationRule[]>(
    (guildId) => rulesService.getAll(guildId),
    [],
    { label: "regles de moderation" },
  );

  const { success, error: showError } = useToast();
  const editing = ref<ModerationRule | null>(null);

  async function toggleRule(rule: ModerationRule) {
    try {
      const newState = !rule.enabled;
      await rulesService.toggle(rule.id, newState);
      rule.enabled = newState;
      success(newState ? "Regle activee avec succes." : "Regle desactivee avec succes.");
    } catch (e) {
      console.error("Erreur lors du basculement de la regle :", e);
      showError("Erreur lors du basculement de la regle.");
    }
  }

  async function updateRule(params: UpdateRuleParams) {
    try {
      await rulesService.update(params);
      await fetchRules();
      success("Regle mise a jour avec succes.");
    } catch (e) {
      console.error("Erreur lors de la mise a jour de la regle :", e);
      showError("Erreur lors de la mise a jour de la regle.");
    }
  }

  function openEdit(rule: ModerationRule) { editing.value = { ...rule }; }
  function closeEdit() { editing.value = null; }

  return { rules, loading, error, editing, fetchRules, toggleRule, updateRule, openEdit, closeEdit };
}
