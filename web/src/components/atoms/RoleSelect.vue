<script setup lang="ts">
import { ref, watch, onMounted, computed } from "vue";
import { errMsg } from "@/utils/errMsg";
import { discordRolesService } from "@/services/discordRolesService";
import type { DiscordRole } from "@/types";

const props = defineProps<{
  modelValue: string;
  id?: string;
  guildId: string | null;
}>();

const emit = defineEmits<{
  (e: "update:modelValue", value: string): void;
}>();

const roles = ref<DiscordRole[]>([]);
const loading = ref(false);
const errorMsg = ref("");

async function load() {
  if (!props.guildId) {
    roles.value = [];
    return;
  }
  loading.value = true;
  errorMsg.value = "";
  try {
    roles.value = await discordRolesService.getAll(props.guildId);
  } catch (e) {
    errorMsg.value = errMsg(e);
  } finally {
    loading.value = false;
  }
}

// Tri par position decroissante (les roles les plus eleves en premier).
const sortedRoles = computed(() =>
  [...roles.value].sort((a, b) => (b.position ?? 0) - (a.position ?? 0)),
);

watch(() => props.guildId, load);
onMounted(load);

function onChange(e: Event) {
  emit("update:modelValue", (e.target as HTMLSelectElement).value);
}

function fmtColor(c: number): string | undefined {
  if (!c) return undefined;
  return "#" + c.toString(16).padStart(6, "0");
}
</script>

<template>
  <div class="role-select-wrap">
    <select
      :id="id"
      :value="modelValue"
      class="role-select"
      :disabled="loading || !guildId"
      @change="onChange"
    >
      <option value="">
        {{ loading ? "Chargement..." : "— Aucun role —" }}
      </option>
      <option
        v-for="r in sortedRoles"
        :key="r.id"
        :value="r.id"
        :style="r.color ? { color: fmtColor(r.color) } : undefined"
      >
        @{{ r.name }}
      </option>
    </select>
    <span v-if="errorMsg" class="role-err">{{ errorMsg }}</span>
  </div>
</template>

<style scoped>
.role-select-wrap {
  display: flex;
  flex-direction: column;
  gap: 4px;
  width: 100%;
}

.role-select {
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

.role-select:focus {
  outline: none;
  border-color: var(--accent);
}

.role-select:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.role-err {
  font-size: 11px;
  color: var(--danger, var(--danger));
}
</style>
