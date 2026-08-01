<script setup lang="ts">
import AppButton from "../atoms/AppButton.vue";
import { useVoiceThemes } from "@/composables/useVoiceThemes";
import { useConfirm } from "@/composables/useConfirm";
import type { VoiceChannelTheme } from "@/types/voice-extended";

const emit = defineEmits<{
  (e: "create"): void;
  (e: "edit", theme: VoiceChannelTheme): void;
}>();

const { themes, loading, remove } = useVoiceThemes();
const { confirm } = useConfirm();

async function onRemove(theme: VoiceChannelTheme) {
  if (
    !(await confirm({
      title: "Supprimer le thème",
      message: `Supprimer le thème "${theme.name}" ?`,
    }))
  )
    return;
  await remove(theme.id);
}
</script>

<template>
  <section class="card">
    <div class="card-header">
      <h2>Thèmes existants</h2>
      <AppButton variant="primary" @click="emit('create')">+ Nouveau thème</AppButton>
    </div>

    <div v-if="loading" class="loading">Chargement…</div>
    <div v-else-if="themes.length === 0" class="empty">
      Aucun thème — créez-en un pour permettre la création automatique de salons.
    </div>
    <table v-else class="table">
      <thead>
        <tr>
          <th></th>
          <th>Nom</th>
          <th>Visibilité</th>
          <th>Limite</th>
          <th>Bitrate</th>
          <th>Drapeaux</th>
          <th></th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="t in themes" :key="t.id">
          <td class="emoji">{{ t.emoji ?? "🎙️" }}</td>
          <td>
            <strong>{{ t.name }}</strong>
            <small class="muted">{{ t.channel_name_template }}</small>
          </td>
          <td>{{ t.visibility }}</td>
          <td>{{ t.member_limit ?? "—" }}</td>
          <td>{{ t.bitrate ? `${Math.round(t.bitrate / 1000)} kbps` : "—" }}</td>
          <td>
            <span v-if="t.is_default" class="flag default">défaut</span>
            <span v-if="t.locked" class="flag locked">verrouillé</span>
            <span v-if="t.queue_enabled" class="flag queue">queue</span>
            <span v-if="t.stage_enabled" class="flag stage">stage</span>
          </td>
          <td class="row-actions">
            <AppButton variant="secondary" @click="emit('edit', t)">Modifier</AppButton>
            <AppButton variant="danger" @click="onRemove(t)">🗑️</AppButton>
          </td>
        </tr>
      </tbody>
    </table>
  </section>
</template>

<style scoped>
@import "../pages/_admin-page-shared.css";
.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 12px;
}
.card-header h2 { margin: 0; }
.emoji { font-size: 1.4rem; text-align: center; width: 40px; }
.flag {
  display: inline-block;
  margin-right: 4px;
  padding: 1px 6px;
  border-radius: var(--radius-md);
  font-size: 0.7rem;
  font-weight: 600;
  color: white;
}
.flag.default { background: var(--accent); }
.flag.locked { background: var(--accent-warm); }
.flag.queue { background: #9B59B6; }
.flag.stage { background: var(--success); }
.row-actions { display: flex; gap: 4px; }
</style>
