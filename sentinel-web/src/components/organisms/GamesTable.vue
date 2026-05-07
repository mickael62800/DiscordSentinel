<script setup lang="ts">
import { gamesService, type Game } from "@/services/gamesService";
import { useGames } from "@/composables/useGames";
import { useGuildSelector } from "@/composables/useGuildSelector";
import { useToast } from "@/composables/useToast";
import { useConfirm } from "@/composables/useConfirm";
import AppButton from "@/components/atoms/AppButton.vue";
import AppBadge from "@/components/atoms/AppBadge.vue";

defineProps<{
  games: Game[];
}>();

const emit = defineEmits<{ edit: [game: Game] }>();

const { selectedGuildId } = useGuildSelector();
const { fetchAll } = useGames();
const { success: showSuccess, error: showError } = useToast();
const { confirm } = useConfirm();

// Extrait l'ID d'un emoji custom Discord `<:name:id>` ou `<a:name:id>`.
const customEmojiRe = /<(a?):([A-Za-z0-9_]+):(\d+)>/;
function emojiCdn(emoji: string | null): string | null {
  if (!emoji) return null;
  const m = emoji.match(customEmojiRe);
  if (!m) return null;
  const animated = m[1] === "a";
  const id = m[3];
  return `https://cdn.discordapp.com/emojis/${id}.${animated ? "gif" : "png"}?size=32`;
}
function emojiText(emoji: string | null): string {
  if (!emoji) return "";
  return customEmojiRe.test(emoji) ? "" : emoji;
}

async function onDelete(game: Game) {
  const gid = selectedGuildId.value;
  if (!gid) return;
  const ok = await confirm({
    title: "Supprimer le jeu",
    message: `Supprimer "${game.game_name}" ? Le role Discord associe sera egalement supprime.`,
  });
  if (!ok) return;
  try {
    await gamesService.delete(gid, game.id);
    showSuccess(`Jeu "${game.game_name}" supprime.`);
    await fetchAll();
  } catch (e) {
    showError(e instanceof Error ? e.message : String(e));
  }
}
</script>

<template>
  <div class="card games-table">
    <div class="row header-row">
      <div class="col emoji">Emoji</div>
      <div class="col name">Nom</div>
      <div class="col category">Categorie</div>
      <div class="col subs">Role</div>
      <div class="col actions">Actions</div>
    </div>
    <div v-for="g in games" :key="g.id" class="row">
      <div class="col emoji">
        <img
          v-if="emojiCdn(g.emoji)"
          :src="emojiCdn(g.emoji)!"
          :alt="g.game_name"
          class="emoji-img"
        />
        <span v-else class="emoji-text">{{ emojiText(g.emoji) || "—" }}</span>
      </div>
      <div class="col name">{{ g.game_name }}</div>
      <div class="col category">
        <AppBadge v-if="g.category" :label="g.category" variant="info" />
        <span v-else class="muted">—</span>
      </div>
      <div class="col subs">
        <AppBadge v-if="g.role_id" :label="`@${g.game_name}`" variant="success" />
        <span v-else class="muted">—</span>
      </div>
      <div class="col actions">
        <AppButton variant="secondary" size="sm" @click="emit('edit', g)">Editer</AppButton>
        <AppButton variant="danger" size="sm" @click="onDelete(g)">Suppr.</AppButton>
      </div>
    </div>
  </div>
</template>

<style scoped>
.games-table {
  padding: 0;
  overflow: hidden;
}

.row {
  display: grid;
  grid-template-columns: 80px 1fr 200px 100px 180px;
  gap: 12px;
  padding: 12px 16px;
  border-bottom: 1px solid var(--border);
  align-items: center;
}
.row:last-child { border-bottom: none; }

.header-row {
  background: var(--bg-secondary);
  font-size: 11px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  color: var(--text-secondary);
}

.col.emoji { display: flex; align-items: center; justify-content: center; }
.emoji-img { width: 28px; height: 28px; object-fit: contain; }
.emoji-text { font-size: 20px; }

.col.name { font-weight: 600; color: var(--text-primary); }
.col.subs { font-variant-numeric: tabular-nums; }
.col.actions { display: flex; gap: 6px; justify-content: flex-end; }

.muted { color: var(--text-secondary); }

@media (max-width: 900px) {
  .row { grid-template-columns: 60px 1fr 1fr; }
  .row .col.subs, .row .col.actions { grid-column: 1 / -1; }
  .row .col.actions { justify-content: flex-start; }
}
</style>
