<script setup lang="ts">
import { useRules } from "../../composables/useRules";
import type { UpdateRuleParams } from "../../types";
import RuleCard from "../organisms/RuleCard.vue";
import RuleEditModal from "../organisms/RuleEditModal.vue";

const { rules, loading, editing, toggleRule, updateRule, openEdit, closeEdit } = useRules();

async function handleSave(params: UpdateRuleParams) {
  await updateRule(params);
  closeEdit();
}
</script>

<template>
  <div class="rules">
    <h1>Regles de moderation</h1>

    <div v-if="loading" class="loading">Chargement...</div>

    <div v-else class="rules-grid">
      <RuleCard
        v-for="rule in rules"
        :key="rule.id"
        :rule="rule"
        @toggle="toggleRule"
        @edit="openEdit"
      />
    </div>

    <RuleEditModal
      v-if="editing"
      :rule="editing"
      @save="handleSave"
      @close="closeEdit"
    />
  </div>
</template>

<style scoped>
.rules h1 {
  margin-bottom: 24px;
}

.rules-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
  gap: 16px;
}
</style>
