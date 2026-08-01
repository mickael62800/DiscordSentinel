<script setup lang="ts">
import AppSelect from "@/components/atoms/AppSelect.vue";
import AppInput from "@/components/atoms/AppInput.vue";
import { ref, watch } from "vue";
import { useVoiceThemes } from "@/composables/useVoiceThemes";
import type { CreateThemePayload, VoiceChannelTheme } from "@/types/voice-extended";
import NumberInputWithUnit from "@/components/atoms/NumberInputWithUnit.vue";

const props = defineProps<{
  open: boolean;
  editing: VoiceChannelTheme | null;
}>();
const emit = defineEmits<{ (e: "close"): void }>();

const { create, update } = useVoiceThemes();

const VISIBILITIES = [
  { key: "public", label: "Public" },
  { key: "private", label: "Privé" },
  { key: "muted", label: "Muet (read-only)" },
];

function emptyDraft(): CreateThemePayload {
  return {
    name: "",
    emoji: "",
    channel_name_template: "{username}",
    member_limit: null,
    visibility: "public",
    locked: false,
    queue_enabled: false,
    bitrate: null,
    slowmode_secs: null,
    stage_enabled: false,
    is_default: false,
    sort_order: 0,
  };
}

const draft = ref<CreateThemePayload>(emptyDraft());

watch(
  () => props.editing,
  (t) => {
    if (t) {
      draft.value = {
        name: t.name,
        emoji: t.emoji,
        channel_name_template: t.channel_name_template,
        member_limit: t.member_limit,
        visibility: t.visibility,
        locked: t.locked,
        queue_enabled: t.queue_enabled,
        bitrate: t.bitrate,
        slowmode_secs: t.slowmode_secs,
        stage_enabled: t.stage_enabled,
        is_default: t.is_default,
        sort_order: t.sort_order,
      };
    } else {
      draft.value = emptyDraft();
    }
  },
  { immediate: true },
);

async function onSave() {
  if (!draft.value.name?.trim()) return;
  if (props.editing) {
    await update(props.editing.id, draft.value);
  } else {
    await create(draft.value);
  }
  emit("close");
}
</script>

<template>
  <div v-if="open" class="modal-backdrop" @click.self="emit('close')">
    <div class="modal">
      <h3>{{ editing ? "Modifier le thème" : "Nouveau thème" }}</h3>
      <form @submit.prevent="onSave" class="form-grid">
        <label>Nom *
          <AppInput v-model="draft.name" required />
        </label>
        <label>Emoji
          <AppInput v-model="draft.emoji" placeholder="🎮" />
        </label>
        <label class="full">Template du nom (variables {username}, {theme})
          <AppInput v-model="draft.channel_name_template" />
        </label>
        <label>Visibilité
          <AppSelect v-model="draft.visibility">
            <option v-for="v in VISIBILITIES" :key="v.key" :value="v.key">{{ v.label }}</option>
          </AppSelect>
        </label>
        <label>Limite de membres
          <NumberInputWithUnit v-model.number="draft.member_limit" :min="0" placeholder="0 = illimité" />
        </label>
        <label>Bitrate (bps)
          <NumberInputWithUnit v-model.number="draft.bitrate" unit="bps" placeholder="64000" />
        </label>
        <label>Slowmode (s)
          <NumberInputWithUnit v-model.number="draft.slowmode_secs" :min="0" unit="s" />
        </label>
        <label>Sort order
          <NumberInputWithUnit v-model.number="draft.sort_order" />
        </label>

        <div class="flags-row full">
          <label class="toggle"><input v-model="draft.locked" type="checkbox" /> Verrouillé (admin only)</label>
          <label class="toggle"><input v-model="draft.queue_enabled" type="checkbox" /> Queue activée</label>
          <label class="toggle"><input v-model="draft.stage_enabled" type="checkbox" /> Stage channel</label>
          <label class="toggle"><input v-model="draft.is_default" type="checkbox" /> Thème par défaut</label>
        </div>

        <div class="actions full">
          <button type="button" class="btn-secondary" @click="emit('close')">Annuler</button>
          <button type="submit" class="btn-primary">{{ editing ? "Enregistrer" : "Créer" }}</button>
        </div>
      </form>
    </div>
  </div>
</template>

<style scoped>
@import "../pages/_admin-page-shared.css";
.modal-backdrop {
  position: fixed; inset: 0;
  background: rgba(0, 0, 0, 0.7);
  display: flex; align-items: center; justify-content: center;
  z-index: 100;
}
.modal {
  background: var(--bg-secondary);
  border-radius: var(--radius-md);
  padding: 24px;
  width: 90%; max-width: 700px;
  max-height: 90vh; overflow-y: auto;
}
.modal h3 { margin: 0 0 16px 0; }
.form-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 12px; }
.form-grid label { display: flex; flex-direction: column; gap: 4px; font-size: 0.9rem; }
.form-grid label.full { grid-column: span 2; }
.form-grid input, .form-grid select {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  padding: 6px 10px;
  color: inherit;
  font-family: inherit;
}
.toggle { flex-direction: row !important; align-items: center; gap: 8px; cursor: pointer; }
.flags-row { display: flex; gap: 16px; flex-wrap: wrap; }
</style>
