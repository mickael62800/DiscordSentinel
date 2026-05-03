<script setup lang="ts">
import { reactive, ref, watch } from "vue";
import { useGuildSelector } from "@/composables/useGuildSelector";
import { useToast } from "@/composables/useToast";
import { conductService } from "@/services/conductService";
import type { ConductConfig } from "@/types";

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
        <input v-model.number="draft.max_points" type="number" min="1" />
      </label>
      <label>Regen (points par tick)
        <input v-model.number="draft.regen_amount" type="number" min="0" />
      </label>
      <label>Intervalle regen (ISO 8601)
        <input v-model="draft.regen_interval" placeholder="P1D" />
      </label>
      <label>Pénalité warn
        <input v-model.number="draft.penalty_warn" type="number" min="0" />
      </label>
      <label>Pénalité delete
        <input v-model.number="draft.penalty_delete" type="number" min="0" />
      </label>
      <label>Pénalité mute
        <input v-model.number="draft.penalty_mute" type="number" min="0" />
      </label>
      <label>Pénalité ban
        <input v-model.number="draft.penalty_ban" type="number" min="0" />
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
@import "../pages/_moderation-advanced-shared.css";
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
