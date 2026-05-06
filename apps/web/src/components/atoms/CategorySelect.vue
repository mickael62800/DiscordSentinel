<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
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

const allChannels = ref<DiscordChannelInfo[]>([]);
const loading = ref(false);
const errorMsg = ref("");

const categories = computed(() =>
  allChannels.value.filter((c) => c.kind === "category"),
);

async function load() {
  if (!props.guildId) {
    allChannels.value = [];
    return;
  }
  loading.value = true;
  errorMsg.value = "";
  try {
    allChannels.value = await guildChannelsService.listAllChannels(props.guildId);
  } catch (e) {
    errorMsg.value = e instanceof Error ? e.message : String(e);
  } finally {
    loading.value = false;
  }
}

watch(() => props.guildId, load);
onMounted(load);

function onChange(e: Event) {
  emit("update:modelValue", (e.target as HTMLSelectElement).value);
}
</script>

<template>
  <div class="cat-select-wrap">
    <select
      :id="id"
      :value="modelValue"
      class="cat-select"
      :disabled="loading || !guildId"
      @change="onChange"
    >
      <option value="">
        {{ loading ? "Chargement..." : "— Aucune categorie —" }}
      </option>
      <option v-for="cat in categories" :key="cat.id" :value="cat.id">
        📁 {{ cat.name }}
      </option>
    </select>
    <span v-if="errorMsg" class="cat-err">{{ errorMsg }}</span>
  </div>
</template>

<style scoped>
.cat-select-wrap { display: flex; flex-direction: column; gap: 4px; }
.cat-select {
  background: var(--bg-input);
  color: var(--text-primary);
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 8px 10px;
  font-size: 13px;
}
.cat-select:disabled { opacity: 0.5; cursor: not-allowed; }
.cat-err { color: var(--danger); font-size: 11px; }
</style>
