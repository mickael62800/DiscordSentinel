<script setup lang="ts">
import AppSelect from "@/components/atoms/AppSelect.vue";
import AppInput from "@/components/atoms/AppInput.vue";
import { reactive, ref } from "vue";
import { useGuildSelector } from "@/composables/useGuildSelector";
import { useToast } from "@/composables/useToast";
import { levelsService, type AddXpPayload } from "@/services/levelsService";
import NumberInputWithUnit from "@/components/atoms/NumberInputWithUnit.vue";

const { guildIdFilter } = useGuildSelector();
const { success, error: showError } = useToast();

const draft = reactive<AddXpPayload>({
  guild_id: "",
  user_id: "",
  username: "",
  amount: 100,
  source: "text",
});
const granting = ref(false);

async function grant() {
  if (!guildIdFilter.value || !draft.user_id.trim() || draft.amount === 0) {
    showError("user_id et montant requis.");
    return;
  }
  granting.value = true;
  try {
    await levelsService.addXp({
      guild_id: guildIdFilter.value,
      user_id: draft.user_id.trim(),
      username: draft.username.trim() || draft.user_id.trim(),
      amount: draft.amount,
      source: draft.source,
    });
    success(`${draft.amount > 0 ? "+" : ""}${draft.amount} XP attribués.`);
    draft.user_id = "";
    draft.username = "";
    draft.amount = 100;
  } catch (e) {
    console.error(e);
    showError("Erreur attribution XP.");
  } finally {
    granting.value = false;
  }
}
</script>

<template>
  <section class="card">
    <h2>🎁 Attribuer XP manuel</h2>
    <p class="hint">
      Ajoute (ou retire si négatif) des points XP à un utilisateur.
      Permet de corriger un farming abusif ou récompenser manuellement.
    </p>
    <form @submit.prevent="grant" class="form">
      <label>User ID *
        <AppInput v-model="draft.user_id" required />
      </label>
      <label>Username
        <AppInput v-model="draft.username" placeholder="(optionnel)" />
      </label>
      <label>Montant *
        <NumberInputWithUnit v-model.number="draft.amount" required unit="xp" />
      </label>
      <label>Source
        <AppSelect v-model="draft.source">
          <option value="text">Texte</option>
          <option value="voice">Vocal</option>
        </AppSelect>
      </label>
      <div class="actions full">
        <button type="submit" class="btn-primary" :disabled="granting">
          {{ granting ? "…" : "Attribuer" }}
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
.form input, .form select {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 4px;
  padding: 6px 10px;
  color: inherit;
  font-family: inherit;
}
.hint { font-size: 0.85rem; color: var(--text-secondary); margin-bottom: 12px; }
@media (max-width: 640px) {
  .form { grid-template-columns: 1fr; gap: 10px; }
  .form label.full { grid-column: 1; }
}
</style>
