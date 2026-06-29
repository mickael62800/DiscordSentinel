<script setup lang="ts">
import AppSelect from "@/components/atoms/AppSelect.vue";
import AppInput from "@/components/atoms/AppInput.vue";
import AppButton from "../../atoms/AppButton.vue";
import { useToast } from "@/composables/useToast";
import type { AnnouncementButton } from "@/services/announcementsService";

const buttons = defineModel<AnnouncementButton[]>({ required: true });

const { error: toastErr } = useToast();

function addButton() {
  if (buttons.value.length >= 5) {
    toastErr("Maximum 5 boutons par annonce (limite Discord).");
    return;
  }
  buttons.value.push({
    label: "",
    style: "primary",
    custom_id: `btn_${buttons.value.length + 1}`,
    url: null,
    emoji: null,
  });
}

function removeButton(idx: number) {
  buttons.value.splice(idx, 1);
}
</script>

<template>
  <div class="buttons-section">
    <div class="section-head">
      <h4>🔘 Boutons interactifs (max 5)</h4>
      <AppButton variant="secondary" size="sm" @click="addButton">+ Ajouter</AppButton>
    </div>
    <p class="muted small">
      Boutons cliquables sous l'annonce. Chaque clic est tracé (visible dans l'historique).
    </p>
    <div v-if="buttons.length === 0" class="muted small">Aucun bouton.</div>
    <div v-else class="button-list">
      <div v-for="(btn, idx) in buttons" :key="idx" class="button-row">
        <AppInput v-model="btn.label" type="text" placeholder="Label" maxlength="80" class="btn-label" />
        <AppSelect v-model="btn.style" class="btn-style">
          <option value="primary">Bleu</option>
          <option value="secondary">Gris</option>
          <option value="success">Vert</option>
          <option value="danger">Rouge</option>
          <option value="link">Lien</option>
        </AppSelect>
        <input
          v-if="btn.style === 'link'"
          v-model="btn.url"
          type="url"
          placeholder="https://..."
          class="btn-url"
        />
        <input
          v-else
          v-model="btn.custom_id"
          type="text"
          placeholder="ID action (ex: rsvp_yes)"
          class="btn-cid"
          maxlength="80"
        />
        <input
          v-model="btn.emoji"
          type="text"
          placeholder="🎉"
          class="btn-emoji"
          maxlength="32"
        />
        <AppButton variant="danger" size="sm" @click="removeButton(idx)">🗑</AppButton>
      </div>
    </div>
  </div>
</template>

<style scoped>
.muted { color: var(--text-secondary); }
.small { font-size: 12px; }

/* Section boutons */
.buttons-section { margin-bottom: 18px; }
.section-head { display: flex; justify-content: space-between; align-items: center; margin-bottom: 6px; }
.section-head h4 { margin: 0; font-size: 13px; }
.button-list { display: flex; flex-direction: column; gap: 6px; }
.button-row {
  display: grid;
  grid-template-columns: 1.4fr 0.8fr 1.2fr 0.5fr auto;
  gap: 6px;
  align-items: center;
}
.button-row input, .button-row select {
  width: 100%; box-sizing: border-box;
  padding: 6px 8px;
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: 5px;
  color: var(--text-primary);
  font-size: 12px;
}
.btn-emoji { text-align: center; }
</style>
