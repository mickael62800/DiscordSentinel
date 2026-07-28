<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { errMsg } from "@/utils/errMsg";
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
    errorMsg.value = errMsg(e);
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
.cat-select-wrap {
  display: flex;
  flex-direction: column;
  gap: 4px;
  width: 100%;
}

.cat-select {
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

.cat-select:focus {
  outline: none;
  border-color: var(--accent);
}

.cat-select:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.cat-err {
  font-size: 11px;
  color: var(--danger, #ef4444);
}
</style>
