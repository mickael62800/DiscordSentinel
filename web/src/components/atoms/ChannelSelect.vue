<script setup lang="ts">
import { ref, watch, onMounted } from "vue";
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
    channels.value = await guildChannelsService.listTextChannels(props.guildId);
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
  <div class="ch-select-wrap">
    <select
      :id="id"
      :value="modelValue"
      class="ch-select"
      :disabled="loading || !guildId"
      @change="onChange"
    >
      <option value="">
        {{ loading ? "Chargement..." : "— Aucun salon —" }}
      </option>
      <option v-for="ch in channels" :key="ch.id" :value="ch.id">
        # {{ ch.name }}
      </option>
    </select>
    <span v-if="errorMsg" class="ch-err">{{ errorMsg }}</span>
  </div>
</template>

<style scoped>
.ch-select-wrap {
  display: flex;
  flex-direction: column;
  gap: 4px;
  width: 100%;
}

.ch-select {
  width: 100%;
  padding: 8px 12px;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
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

.ch-select:focus {
  outline: none;
  border-color: var(--accent);
}

.ch-select:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.ch-err {
  font-size: 11px;
  color: var(--danger, var(--danger));
}
</style>
