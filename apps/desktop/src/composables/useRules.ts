import { ref, onMounted, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { ModerationRule, UpdateRuleParams } from "../types";
import { useGuildSelector } from "./useGuildSelector";

export function useRules() {
  const rules = ref<ModerationRule[]>([]);
  const loading = ref(true);
  const editing = ref<ModerationRule | null>(null);
  const { guildIdFilter } = useGuildSelector();

  async function fetchRules() {
    loading.value = true;
    try {
      rules.value = await invoke<ModerationRule[]>("get_rules", { guildId: guildIdFilter.value ?? null });
    } catch (e) {
      console.error("Erreur chargement regles:", e);
    } finally {
      loading.value = false;
    }
  }

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

  onMounted(fetchRules);
  watch(guildIdFilter, fetchRules);

  return { rules, loading, editing, fetchRules, toggleRule, updateRule, openEdit, closeEdit };
}
