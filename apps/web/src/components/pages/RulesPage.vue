<script setup lang="ts">
import { useRules } from "../../composables/useRules";
import { useGuildSelector } from "../../composables/useGuildSelector";
import { useToast } from "../../composables/useToast";
import type { UpdateRuleParams } from "../../types";
import RuleCard from "../organisms/RuleCard.vue";

const { success, error: showError } = useToast();
const { selectedGuildId } = useGuildSelector();
const { rules, loading, toggleRule, updateRule } = useRules();

async function handleSave(params: UpdateRuleParams) {
  try {
    await updateRule(params);
    success("Regle mise a jour avec succes");
  } catch (e) {
    console.error("Erreur mise a jour regle:", e);
    showError("Erreur lors de la mise a jour de la regle");
  }
}

async function handleToggle(rule: Parameters<typeof toggleRule>[0]) {
  try {
    await toggleRule(rule);
    success(rule.enabled ? "Regle desactivee" : "Regle activee");
  } catch (e) {
    console.error("Erreur activation/desactivation regle:", e);
    showError("Erreur lors du changement d'etat de la regle");
  }
}
</script>

<template>
  <div class="rules">
    <h1>Regles de moderation</h1>

    <div v-if="loading" class="loading">Chargement...</div>
    <div v-else-if="!selectedGuildId" class="empty">
      Selectionne un serveur pour voir ses regles.
    </div>
    <div v-else class="rules-grid">
      <RuleCard
        v-for="rule in rules"
        :key="rule.id"
        :rule="rule"
        :guild-id="selectedGuildId"
        @toggle="handleToggle"
        @save="handleSave"
      />
    </div>
  </div>
</template>

<style scoped>
.rules h1 {
  margin-bottom: 24px;
}

.rules-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(360px, 1fr));
  gap: 16px;
}

.loading,
.empty {
  color: var(--text-secondary);
  padding: 40px;
  text-align: center;
}
</style>
