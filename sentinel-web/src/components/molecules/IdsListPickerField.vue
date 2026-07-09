<script setup lang="ts">
import { ref, computed, watch, onMounted } from "vue";
import { errMsg } from "@/utils/errMsg";
import { guildChannelsService } from "@/services/guildChannelsService";
import { discordRolesService } from "@/services/discordRolesService";
import type { DiscordChannelInfo, DiscordRole } from "@/types";

type Kind = "channel" | "channel-voice" | "role";

const props = defineProps<{
  modelValue: string;
  guildId: string | null;
  kind: Kind;
}>();

const emit = defineEmits<{
  (e: "update:modelValue", value: string): void;
}>();

interface Option {
  id: string;
  label: string;
  color?: string;
}

const channels = ref<DiscordChannelInfo[]>([]);
const roles = ref<DiscordRole[]>([]);
const loading = ref(false);
const errorMsg = ref("");
const pickedId = ref("");

async function load() {
  if (!props.guildId) {
    channels.value = [];
    roles.value = [];
    return;
  }
  loading.value = true;
  errorMsg.value = "";
  try {
    if (props.kind === "channel") {
      channels.value = await guildChannelsService.listTextChannels(props.guildId);
    } else if (props.kind === "channel-voice") {
      const all = await guildChannelsService.listAllChannels(props.guildId);
      channels.value = all.filter((c) => c.kind === "voice" || c.kind === "stage");
    } else {
      roles.value = await discordRolesService.getAll(props.guildId);
    }
  } catch (e) {
    errorMsg.value = errMsg(e);
  } finally {
    loading.value = false;
  }
}

watch(() => props.guildId, load);
watch(() => props.kind, load);
onMounted(load);

const options = computed<Option[]>(() => {
  if (props.kind === "channel" || props.kind === "channel-voice") {
    const prefix = props.kind === "channel-voice" ? "🔊" : "#";
    return channels.value.map((c) => ({ id: c.id, label: `${prefix} ${c.name}` }));
  }
  const sorted = [...roles.value].sort((a, b) => (b.position ?? 0) - (a.position ?? 0));
  return sorted.map((r) => ({
    id: r.id,
    label: `@${r.name}`,
    color: r.color ? "#" + r.color.toString(16).padStart(6, "0") : undefined,
  }));
});

const selectedIds = computed<string[]>(() =>
  props.modelValue
    .split(/[\n,;]+/)
    .map((s) => s.trim())
    .filter(Boolean),
);

const usedSet = computed(() => new Set(selectedIds.value));

const availableOptions = computed(() =>
  options.value.filter((o) => !usedSet.value.has(o.id)),
);

function labelFor(id: string): string {
  return options.value.find((o) => o.id === id)?.label ?? `ID ${id}`;
}

function colorFor(id: string): string | undefined {
  return options.value.find((o) => o.id === id)?.color;
}

function serialize(ids: string[]): string {
  return ids.join(",");
}

function add() {
  if (!pickedId.value) return;
  if (usedSet.value.has(pickedId.value)) return;
  emit("update:modelValue", serialize([...selectedIds.value, pickedId.value]));
  pickedId.value = "";
}

function remove(id: string) {
  emit("update:modelValue", serialize(selectedIds.value.filter((x) => x !== id)));
}

const placeholderTxt = computed(() =>
  props.kind === "role" ? "— Choisir un rôle —" : "— Choisir un salon —",
);
</script>

<template>
  <div class="ids-field">
    <div class="picker-row">
      <select
        v-model="pickedId"
        class="picker-select"
        :disabled="loading || !guildId || availableOptions.length === 0"
        @change="add"
      >
        <option value="">
          {{
            loading
              ? "Chargement..."
              : availableOptions.length === 0
                ? "— Tout est déjà ajouté —"
                : placeholderTxt
          }}
        </option>
        <option
          v-for="o in availableOptions"
          :key="o.id"
          :value="o.id"
          :style="o.color ? { color: o.color } : undefined"
        >
          {{ o.label }}
        </option>
      </select>
    </div>

    <span v-if="errorMsg" class="err">{{ errorMsg }}</span>

    <div v-if="selectedIds.length > 0" class="chips">
      <span
        v-for="id in selectedIds"
        :key="id"
        class="chip"
        :style="colorFor(id) ? { borderColor: colorFor(id), color: colorFor(id) } : undefined"
      >
        <span class="chip-label" :title="labelFor(id)">{{ labelFor(id) }}</span>
        <button
          type="button"
          class="chip-remove"
          :title="`Retirer ${labelFor(id)}`"
          @click="remove(id)"
        >
          <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden="true">
            <path
              d="M2 2l6 6M8 2l-6 6"
              stroke="currentColor"
              stroke-width="1.8"
              stroke-linecap="round"
            />
          </svg>
        </button>
      </span>
    </div>

    <p v-else class="empty">
      Aucun {{ kind === "role" ? "rôle" : "salon" }} sélectionné.
    </p>
  </div>
</template>

<style scoped>
.ids-field {
  display: flex;
  flex-direction: column;
  gap: 8px;
  width: 100%;
}

.picker-row {
  display: flex;
}

.picker-select {
  width: 100%;
  padding: 8px 28px 8px 12px;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm, 6px);
  background: var(--bg-card);
  color: var(--text-primary);
  font-size: 13px;
  cursor: pointer;
  appearance: none;
  background-image: url("data:image/svg+xml,%3Csvg width='10' height='6' viewBox='0 0 10 6' xmlns='http://www.w3.org/2000/svg'%3E%3Cpath d='M1 1l4 4 4-4' stroke='%239ca3af' stroke-width='1.5' fill='none' stroke-linecap='round'/%3E%3C/svg%3E");
  background-repeat: no-repeat;
  background-position: right 10px center;
}

.picker-select:focus {
  outline: none;
  border-color: var(--accent);
}

.picker-select:disabled {
  opacity: 0.55;
  cursor: not-allowed;
}

.err {
  font-size: 11px;
  color: var(--danger, #ef4444);
}

.chips {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.chip {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 4px 4px 4px 10px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 999px;
  font-size: 12px;
  color: var(--text-primary);
  max-width: 100%;
}

.chip-label {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  max-width: 220px;
}

.chip-remove {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 18px;
  height: 18px;
  border: none;
  border-radius: 50%;
  background: transparent;
  color: inherit;
  opacity: 0.7;
  cursor: pointer;
  transition: background 0.12s, opacity 0.12s;
}

.chip-remove:hover {
  background: var(--danger-bg, rgba(237, 66, 69, 0.2));
  color: var(--danger, #ed4245);
  opacity: 1;
}

.empty {
  margin: 0;
  font-size: 12px;
  color: var(--text-secondary);
  font-style: italic;
}
</style>
