<script setup lang="ts">
import { ref, watch, onMounted, computed } from "vue";
import { guildChannelsService } from "@/services/guildChannelsService";
import type { DiscordChannelInfo } from "@/types";

const props = defineProps<{
  modelValue: string;
  id?: string;
  guildId: string | null;
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
    channels.value = await guildChannelsService.listAllChannels(props.guildId);
  } catch (e) {
    errorMsg.value = e instanceof Error ? e.message : String(e);
  } finally {
    loading.value = false;
  }
}

// Tri : categories, puis vocaux, puis textuels, par nom.
const sorted = computed(() => {
  const rank = (k?: string) => (k === "category" ? 0 : k === "voice" || k === "stage" ? 1 : 2);
  return [...channels.value].sort(
    (a, b) => rank(a.kind) - rank(b.kind) || a.name.localeCompare(b.name),
  );
});

watch(() => props.guildId, load);
onMounted(load);

function icon(kind?: string): string {
  switch (kind) {
    case "voice": return "🔊";
    case "stage": return "📢";
    case "category": return "📂";
    default: return "#";
  }
}

function onChange(e: Event) {
  emit("update:modelValue", (e.target as HTMLSelectElement).value);
}
</script>

<template>
  <div class="ac-select-wrap">
    <select
      :id="id"
      :value="modelValue"
      class="ac-select"
      :disabled="loading || !guildId"
      @change="onChange"
    >
      <option value="">
        {{ loading ? "Chargement..." : "— Aucun salon —" }}
      </option>
      <option v-for="ch in sorted" :key="ch.id" :value="ch.id">
        {{ icon(ch.kind) }} {{ ch.name }}
      </option>
    </select>
    <span v-if="errorMsg" class="ac-err">{{ errorMsg }}</span>
  </div>
</template>

<style scoped>
.ac-select-wrap {
  display: flex;
  flex-direction: column;
  gap: 4px;
  width: 100%;
}

.ac-select {
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

.ac-select:focus {
  outline: none;
  border-color: var(--accent);
}

.ac-select:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.ac-err {
  font-size: 11px;
  color: var(--danger, #ef4444);
}
</style>
