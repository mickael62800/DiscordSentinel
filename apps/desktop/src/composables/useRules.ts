import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { ModerationRule, UpdateRuleParams } from "../types";
import { useGuildFetch } from "./useGuildFetch";

export function useRules() {
  const { data: rules, loading, refresh: fetchRules } = useGuildFetch<ModerationRule[]>(
    "get_rules",
    [],
  );

  const editing = ref<ModerationRule | null>(null);

  async function toggleRule(rule: ModerationRule) {
    const newState = !rule.enabled;
    await invoke<boolean>("toggle_rule", { id: rule.id, enabled: newState });
    rule.enabled = newState;
  }

  async function updateRule(params: UpdateRuleParams) {
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
  }

  function openEdit(rule: ModerationRule) {
    editing.value = { ...rule };
  }

  function closeEdit() {
    editing.value = null;
  }

  return { rules, loading, editing, fetchRules, toggleRule, updateRule, openEdit, closeEdit };
}
