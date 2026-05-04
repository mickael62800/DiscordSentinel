<script setup lang="ts">
import { ref, watch, onMounted, computed } from "vue";
import { guildChannelsService } from "@/services/guildChannelsService";
import type { DiscordChannelInfo } from "@/types";

const props = defineProps<{
  modelValue: string;
  id?: string;
  guildId: string | null;
  includeStage?: boolean;
}>();

const emit = defineEmits<{
  (e: "update:modelValue", value: string): void;
}>();

const channels = ref<DiscordChannelInfo[]>([]);
const loading = ref(false);
const errorMsg = ref("");

async function load() {
  if (!props.guildId) {
    channels.value = [];
    return;
  }
  loading.value = true;
  errorMsg.value = "";
  try {
    const all = await guildChannelsService.listAllChannels(props.guildId);
    channels.value = all.filter((c) =>
      c.kind === "voice" || (props.includeStage !== false && c.kind === "stage"),
    );
  } catch (e) {
    errorMsg.value = e instanceof Error ? e.message : String(e);
  } finally {
    loading.value = false;
  }
}

const sortedChannels = computed(() =>
  [...channels.value].sort((a, b) => a.name.localeCompare(b.name)),
);

watch(() => props.guildId, load);
onMounted(load);

function onChange(e: Event) {
  emit("update:modelValue", (e.target as HTMLSelectElement).value);
}

function icon(kind?: string): string {
  return kind === "stage" ? "📢" : "🔊";
}
</script>

<template>
  <div class="vc-select-wrap">
    <select
      :id="id"
      :value="modelValue"
      class="vc-select"
      :disabled="loading || !guildId"
      @change="onChange"
    >
      <option value="">
        {{ loading ? "Chargement..." : "— Aucun salon vocal —" }}
      </option>
      <option v-for="ch in sortedChannels" :key="ch.id" :value="ch.id">
        {{ icon(ch.kind) }} {{ ch.name }}
      </option>
    </select>
    <span v-if="errorMsg" class="vc-err">{{ errorMsg }}</span>
  </div>
</template>

<style scoped>
.vc-select-wrap {
  display: flex;
  flex-direction: column;
  gap: 4px;
  width: 100%;
}

.vc-select {
  width: 100%;
  padding: 8px 12px;
  border: 1px solid var(--border);
  border-radius: 6px;
  background: var(--bg-card);
  color: var(--text-primary);
  font-size: 13px;
  font-family: inherit;
  cursor: pointer;
  appearance: none;
  background-image: url("data:image/svg+xml,%3Csvg width='10' height='6' viewBox='0 0 10 6' xmlns='http://www.w3.org/2000/svg'%3E%3Cpath d='M1 1l4 4 4-4' stroke='%239ca3af' stroke-width='1.5' fill='none' stroke-linecap='round'/%3E%3C/svg%3E");
  background-repeat: no-repeat;
  background-position: right 12px center;
  padding-right: 32px;
}

.vc-select:focus {
  outline: none;
  border-color: var(--accent);
}

.vc-select:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.vc-err {
  font-size: 11px;
  color: var(--danger, #ef4444);
}
</style>
