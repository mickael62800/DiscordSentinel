<script setup lang="ts">
import type { GameTemplate } from "@/services/gamePortalService";
import { useComponentVisibility } from "@/composables/useComponentVisibility";

const { visible } = useComponentVisibility();

defineProps<{
  templates: GameTemplate[];
  busy: string | null;
}>();

const emit = defineEmits<{
  launch: [template: GameTemplate];
}>();
</script>

<template>
  <section class="panel catalog">
    <div class="panel-head">
      <h2>Catalogue de jeux</h2>
      <span class="hint">{{ templates.length }} template(s)</span>
    </div>
    <div class="game-grid">
      <article
        v-for="t in templates"
        :key="t.id"
        class="game-card"
        :style="{ '--accent': '#' + (t.accent_color ?? '5865f2') }"
      >
        <div class="game-cover">
          <img
            v-if="t.cover_image_url"
            :src="t.cover_image_url"
            :alt="t.name"
            class="game-cover-img"
            loading="lazy"
            @error="($event.target as HTMLImageElement).style.display = 'none'"
          />
          <div v-else class="game-cover-fallback">{{ t.icon ?? '🎮' }}</div>
        </div>
        <div class="game-body">
          <div class="game-title">
            {{ t.name }}
            <span v-if="t.category" class="cat">{{ t.category }}</span>
          </div>
          <p class="game-desc">{{ t.description ?? '' }}</p>
          <code class="img">{{ t.slug }}</code>
        </div>
        <button
          v-if="visible('game.server.create')"
          class="btn-launch"
          :disabled="busy === t.id"
          @click="emit('launch', t)"
        >
          {{ busy === t.id ? "…" : "Lancer" }}
        </button>
      </article>
    </div>
  </section>
</template>

<style scoped>
.panel {
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  padding: var(--space-lg);
  display: flex; flex-direction: column;
  min-height: 0;
}
.panel-head {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: var(--space-md);
  margin-bottom: var(--space-md);
}
.panel-head h2 { margin: 0; font-size: 14px; font-weight: 600; }
.hint { color: var(--text-secondary); font-size: 12px; font-weight: 400; }

.game-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
  gap: var(--space-md);
  overflow: auto;
  flex: 1;
  min-height: 0;
  padding-right: 4px;
  align-content: start;
}

.game-card {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  padding: var(--space-md);
  display: flex; flex-direction: column; gap: var(--space-sm);
  position: relative; overflow: hidden;
  border-top: 3px solid var(--accent);
  transition: var(--transition-fast);
}
.game-card:hover {
  transform: translateY(-2px);
  box-shadow: var(--shadow-md);
  border-color: var(--accent);
}
.game-card::before {
  content: '';
  position: absolute; inset: 0;
  background: radial-gradient(300px 100px at 50% -50%, var(--accent), transparent 70%);
  opacity: 0.12;
  pointer-events: none;
}

.game-cover {
  width: 100%;
  aspect-ratio: 460 / 215;
  border-radius: var(--radius-sm);
  overflow: hidden;
  background: linear-gradient(135deg, var(--accent), var(--bg-card));
  display: flex;
  align-items: center;
  justify-content: center;
  position: relative;
  z-index: 1;
}
.game-cover-img {
  width: 100%;
  height: 100%;
  object-fit: contain;
  display: block;
}
.game-cover-fallback {
  font-size: 48px;
  filter: drop-shadow(0 2px 4px rgba(0, 0, 0, 0.3));
}

.game-title {
  font-weight: 700; font-size: 14px;
  display: flex; gap: var(--space-sm); align-items: center;
  color: var(--text-primary);
}
.cat {
  font-size: 9px; font-weight: 600;
  background: var(--bg-primary);
  padding: 2px 6px;
  border-radius: var(--radius-sm);
  color: var(--text-secondary);
  text-transform: uppercase; letter-spacing: 0.5px;
}
.game-desc {
  margin: 0;
  font-size: 12px;
  color: var(--text-secondary);
  line-height: 1.4;
  flex: 1;
}
.img {
  font-size: 11px; color: var(--text-secondary);
  background: var(--bg-primary);
  padding: 4px 6px;
  border-radius: var(--radius-sm);
  display: block; word-break: break-all;
  font-family: 'JetBrains Mono', 'Cascadia Code', monospace;
}

.btn-launch {
  background: var(--accent);
  color: #fff;
  border: none;
  border-radius: var(--radius-sm);
  padding: 8px;
  font-weight: 700;
  cursor: pointer;
  text-transform: uppercase; letter-spacing: 0.5px; font-size: 11px;
  transition: var(--transition-fast);
}
.btn-launch:hover:not(:disabled) { background: var(--accent-hover); }
.btn-launch:disabled { opacity: 0.4; cursor: not-allowed; }
</style>
