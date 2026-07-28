<script setup lang="ts">
import { computed } from "vue";
import type { UserActivity } from "../../../types";
import { topChannels, topVoiceCompanions } from "../../../utils/memberActivity";

const props = defineProps<{ activity: UserActivity[] }>();

const channels = computed(() => topChannels(props.activity));
const companions = computed(() => topVoiceCompanions(props.activity));
</script>

<template>
  <div v-if="channels.length > 0 || companions.length > 0" class="section watch-tops">
    <div v-if="channels.length > 0" class="watch-tops-col">
      <h3>🏆 Top salons</h3>
      <ul class="watch-rank">
        <li v-for="(c, i) in channels" :key="c.id">
          <span class="rank-pos">#{{ i + 1 }}</span>
          <span class="rank-name">#{{ c.name }}</span>
          <span class="rank-count">{{ c.count }} msg</span>
        </li>
      </ul>
    </div>
    <div v-if="companions.length > 0" class="watch-tops-col">
      <h3>👥 Compagnons vocaux</h3>
      <ul class="watch-rank">
        <li v-for="(c, i) in companions" :key="c.user_id">
          <span class="rank-pos">#{{ i + 1 }}</span>
          <span class="rank-name">{{ c.username }}</span>
          <span class="rank-count">{{ c.count }}×</span>
        </li>
      </ul>
    </div>
  </div>
</template>

<style scoped>
.section { margin-bottom: 20px; }

.watch-tops { display: grid; grid-template-columns: 1fr 1fr; gap: 16px; }
@media (max-width: 700px) {
  .watch-tops { grid-template-columns: 1fr; }
}
.watch-tops-col {
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 10px 14px;
}
.watch-tops-col h3 { margin: 0 0 8px; font-size: 13px; }
.watch-rank { list-style: none; margin: 0; padding: 0; }
.watch-rank li {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 4px 0;
  font-size: 13px;
  border-bottom: 1px solid color-mix(in srgb, var(--border) 50%, transparent);
}
.watch-rank li:last-child { border-bottom: none; }
.rank-pos { font-weight: 700; color: var(--accent); min-width: 28px; }
.rank-name { flex: 1; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.rank-count { color: var(--text-secondary); font-size: 12px; }
</style>
