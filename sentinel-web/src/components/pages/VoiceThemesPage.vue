<script setup lang="ts">
import { ref } from "vue";
import AdminPageShell from "@/components/layouts/AdminPageShell.vue";
import VoiceThemesTable from "../organisms/VoiceThemesTable.vue";
import VoiceThemeFormModal from "../organisms/VoiceThemeFormModal.vue";
import type { VoiceChannelTheme } from "@/types/voice-extended";

const showForm = ref(false);
const editing = ref<VoiceChannelTheme | null>(null);

function onCreate() {
  editing.value = null;
  showForm.value = true;
}
function onEdit(t: VoiceChannelTheme) {
  editing.value = t;
  showForm.value = true;
}
function onClose() {
  showForm.value = false;
  editing.value = null;
}
</script>

<template>
  <AdminPageShell title="Thèmes voice channels" icon="🎙️">
    <template #lede>
      Gabarits de salons vocaux temporaires (nom, limite, bitrate, visibilité,
      slowmode, queue, stage). Quand un membre rejoint le salon trigger
      configuré, le bot crée un salon dérivé du thème par défaut.
      Variables : <code>{username}</code>, <code>{theme}</code>.
    </template>

    <VoiceThemesTable @create="onCreate" @edit="onEdit" />
    <VoiceThemeFormModal :open="showForm" :editing="editing" @close="onClose" />
  </AdminPageShell>
</template>

<style scoped>
@import "./_admin-page-shared.css";
</style>
