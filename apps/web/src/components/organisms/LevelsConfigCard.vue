<script setup lang="ts">
import { reactive, ref, watch } from "vue";
import { useGuildSelector } from "@/composables/useGuildSelector";
import { useToast } from "@/composables/useToast";
import { levelsService } from "@/services/levelsService";
import type { LevelConfig } from "@/types";

const { guildIdFilter } = useGuildSelector();
const { success, error: showError } = useToast();

const cfg = ref<LevelConfig | null>(null);
const draft = reactive({
  xp_per_message: 15,
  xp_per_voice_minute: 5,
  xp_cooldown_secs: 60,
  level_up_channel_id: "",
  level_up_message: "",
  excluded_channels: "",
  enabled: true,
});
const saving = ref(false);

async function fetchCfg() {
  if (!guildIdFilter.value) return;
  try {
    cfg.value = await levelsService.getConfig(guildIdFilter.value);
    Object.assign(draft, {
      xp_per_message: cfg.value.xp_per_message,
      xp_per_voice_minute: cfg.value.xp_per_voice_minute,
      xp_cooldown_secs: cfg.value.xp_cooldown_secs,
      level_up_channel_id: cfg.value.level_up_channel_id ?? "",
      level_up_message: cfg.value.level_up_message,
      excluded_channels: (cfg.value.excluded_channels ?? []).join(","),
      enabled: cfg.value.enabled,
    });
  } catch (e) {
    console.error(e);
    showError("Erreur chargement config niveaux.");
  }
}

async function save() {
  if (!guildIdFilter.value) return;
  saving.value = true;
  try {
    cfg.value = await levelsService.saveConfig({
      guild_id: guildIdFilter.value,
      xp_per_message: draft.xp_per_message,
      xp_per_voice_minute: draft.xp_per_voice_minute,
      xp_cooldown_secs: draft.xp_cooldown_secs,
      level_up_channel_id: draft.level_up_channel_id || null,
      level_up_message: draft.level_up_message,
      excluded_channels: draft.excluded_channels.split(",").map((s) => s.trim()).filter(Boolean),
      enabled: draft.enabled,
    });
    success("Config niveaux enregistrée.");
  } catch (e) {
    console.error(e);
    showError("Erreur sauvegarde config niveaux.");
  } finally {
    saving.value = false;
  }
}

watch(guildIdFilter, fetchCfg, { immediate: true });
</script>

<template>
  <section class="card">
    <h2>📈 Niveaux & XP</h2>
    <form @submit.prevent="save" class="form">
      <label class="toggle full">
        <input v-model="draft.enabled" type="checkbox" />
        Système actif
      </label>
      <label>XP par message
        <input v-model.number="draft.xp_per_message" type="number" min="0" />
      </label>
      <label>XP par minute vocale
        <input v-model.number="draft.xp_per_voice_minute" type="number" min="0" />
      </label>
      <label>Cooldown XP (s)
        <input v-model.number="draft.xp_cooldown_secs" type="number" min="0" />
      </label>
      <label>Salon level-up (ID)
        <input v-model="draft.level_up_channel_id" placeholder="vide = pas d'annonce" />
      </label>
      <label class="full">Message level-up (variables {user}, {level})
        <input v-model="draft.level_up_message" />
      </label>
      <label class="full">Salons exclus (IDs séparés par virgules)
        <input v-model="draft.excluded_channels" placeholder="ID1,ID2,..." />
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
.form input, .form select {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 4px;
  padding: 6px 10px;
  color: inherit;
  font-family: inherit;
}
.toggle { flex-direction: row !important; align-items: center; gap: 8px; cursor: pointer; }
@media (max-width: 640px) {
  .form { grid-template-columns: 1fr; gap: 10px; }
  .form label.full { grid-column: 1; }
}
</style>
