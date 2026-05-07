<script setup lang="ts">
import AppInput from "@/components/atoms/AppInput.vue";
import { reactive, ref, watch } from "vue";
import { useGuildSelector } from "@/composables/useGuildSelector";
import { useToast } from "@/composables/useToast";
import { conductService } from "@/services/conductService";
import type { ConductConfig } from "@/types";
import NumberInputWithUnit from "@/components/atoms/NumberInputWithUnit.vue";

const { guildIdFilter } = useGuildSelector();
const { success, error: showError } = useToast();

const cfg = ref<ConductConfig | null>(null);
const draft = reactive({
  max_points: 12,
  regen_amount: 1,
  regen_interval: "P1D",
  penalty_warn: 1,
  penalty_delete: 1,
  penalty_mute: 3,
  penalty_ban: 6,
});
const saving = ref(false);

async function fetchCfg() {
  if (!guildIdFilter.value) return;
  try {
    cfg.value = await conductService.getConfig(guildIdFilter.value);
    Object.assign(draft, cfg.value);
  } catch (e) {
    console.error(e);
    showError("Erreur chargement config conduite.");
  }
}

async function save() {
  if (!guildIdFilter.value) return;
  saving.value = true;
  try {
    cfg.value = await conductService.saveConfig({
      guild_id: guildIdFilter.value,
      ...draft,
    });
    success("Config conduite enregistrée.");
  } catch (e) {
    console.error(e);
    showError("Erreur sauvegarde config conduite.");
  } finally {
    saving.value = false;
  }
}

watch(guildIdFilter, fetchCfg, { immediate: true });
</script>

<template>
  <section class="card">
    <h2>🛡️ Système de conduite</h2>
    <form @submit.prevent="save" class="form">
      <label>Points max
        <NumberInputWithUnit v-model.number="draft.max_points" :min="1" />
      </label>
      <label>Regen (points par tick)
        <NumberInputWithUnit v-model.number="draft.regen_amount" :min="0" />
      </label>
      <label>Intervalle regen (ISO 8601)
        <AppInput v-model="draft.regen_interval" placeholder="P1D" />
      </label>
      <label>Pénalité warn
        <NumberInputWithUnit v-model.number="draft.penalty_warn" :min="0" />
      </label>
      <label>Pénalité delete
        <NumberInputWithUnit v-model.number="draft.penalty_delete" :min="0" />
      </label>
      <label>Pénalité mute
        <NumberInputWithUnit v-model.number="draft.penalty_mute" :min="0" />
      </label>
      <label>Pénalité ban
        <NumberInputWithUnit v-model.number="draft.penalty_ban" :min="0" />
      </label>
      <div class="actions full">
        <button type="submit" class="btn-primary" :disabled="saving">
          {{ saving ? "Enregistrement…" : "Enregistrer" }}
        </button>
      </div>
    </form>
  </section>
</template>

<style scoped>
@import "../pages/_admin-page-shared.css";
.form { display: grid; grid-template-columns: 1fr 1fr; gap: 12px; }
.form label { display: flex; flex-direction: column; gap: 4px; font-size: 0.9rem; }
.form label.full { grid-column: span 2; }
.form input {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 4px;
  padding: 6px 10px;
  color: inherit;
  font-family: inherit;
}
@media (max-width: 640px) {
  .form { grid-template-columns: 1fr; gap: 10px; }
  .form label.full { grid-column: 1; }
}
</style>
