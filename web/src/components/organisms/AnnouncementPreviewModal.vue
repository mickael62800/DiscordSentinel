<script setup lang="ts">
import type { RenderedAnnouncement } from "@/services/announcementsService";
import AppModal from "../atoms/AppModal.vue";
import { safeHttpsImageUrl } from "@/utils/safeUrl";

defineProps<{
  preview: RenderedAnnouncement | null;
}>();

const emit = defineEmits<{ close: [] }>();
</script>

<template>
  <AppModal
    :visible="!!preview"
    title="👁 Aperçu"
    size="md"
    @close="emit('close')"
  >
    <div v-if="preview" class="preview-body">
      <p v-if="preview.mentions_prefix" class="prev-mentions">{{ preview.mentions_prefix }}</p>
      <div
        v-if="preview.embed"
        class="prev-embed"
        :style="{ borderLeftColor: preview.embed.color != null ? '#' + preview.embed.color.toString(16).padStart(6, '0') : '#5865f2' }"
      >
        <h4 v-if="preview.embed.title">{{ preview.embed.title }}</h4>
        <p class="prev-desc">{{ preview.embed.description }}</p>
        <img v-if="safeHttpsImageUrl(preview.embed.thumbnail_url)" :src="safeHttpsImageUrl(preview.embed.thumbnail_url)!" class="prev-thumb" />
        <img v-if="safeHttpsImageUrl(preview.embed.image_url)" :src="safeHttpsImageUrl(preview.embed.image_url)!" class="prev-img" />
      </div>
      <p v-else class="prev-text">{{ preview.content_text }}</p>
      <p class="muted small">
        Sera publié sur {{ preview.channel_ids.length }} salon{{ preview.channel_ids.length > 1 ? "s" : "" }}.
      </p>
    </div>
  </AppModal>
</template>

<style scoped>
.preview-body { display: flex; flex-direction: column; gap: 10px; }
.prev-mentions { font-weight: 600; color: var(--accent); margin: 0; }
.prev-embed {
  background: var(--bg-secondary);
  border-left: 4px solid var(--accent);
  border-radius: var(--radius-sm);
  padding: 12px;
}
.prev-embed h4 { margin: 0 0 6px 0; font-size: 14px; }
.prev-desc { white-space: pre-wrap; margin: 0; font-size: 13px; }
.prev-text { white-space: pre-wrap; margin: 0; font-size: 13px; }
.prev-img { max-width: 100%; border-radius: var(--radius-sm); margin-top: 8px; }
.prev-thumb {
  max-width: 80px; max-height: 80px;
  border-radius: var(--radius-sm);
  float: right;
  margin-left: 10px;
}
.muted { color: var(--text-secondary); }
.small { font-size: 12px; }
</style>
