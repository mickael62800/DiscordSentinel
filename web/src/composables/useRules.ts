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

  const { error: showError } = useToast();

  async function toggleRule(rule: ModerationRule) {
    try {
      const newState = !rule.enabled;
      await rulesService.toggle(rule.id, newState);
      rule.enabled = newState;
    } catch (e) {
      console.error("Erreur lors du basculement de la regle :", e);
      showError("Erreur lors du basculement de la regle.");
      throw e;
    }
  }

  async function updateRule(params: UpdateRuleParams) {
    await rulesService.update(params);
    await fetchRules();
  }

  return { rules, loading, error, fetchRules, toggleRule, updateRule };
}
