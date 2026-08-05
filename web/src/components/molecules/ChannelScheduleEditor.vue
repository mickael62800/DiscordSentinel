<script setup lang="ts">
// Editeur « salon + frequence » : liste de couples { salon, heures }.
// La valeur est serialisee en JSON : [{"channel_id":"123","hours":24}, ...].
import { computed } from "vue";
import ChannelSelect from "../atoms/ChannelSelect.vue";

interface Row {
  channel_id: string;
  hours: number;
}

const props = defineProps<{
  modelValue: string;
  guildId: string | null;
}>();
const emit = defineEmits<{ "update:modelValue": [value: string] }>();

const rows = computed<Row[]>(() => {
  if (!props.modelValue?.trim()) return [];
  try {
    const parsed = JSON.parse(props.modelValue);
    if (!Array.isArray(parsed)) return [];
    return parsed
      .filter((r) => r && typeof r === "object")
      .map((r) => ({
        channel_id: String(r.channel_id ?? ""),
        hours: Number(r.hours) > 0 ? Number(r.hours) : 24,
      }));
  } catch {
    return [];
  }
});

function commit(next: Row[]) {
  // On ne garde que les lignes avec un salon choisi.
  const clean = next.filter((r) => r.channel_id);
  emit("update:modelValue", JSON.stringify(clean));
}

function addRow() {
  commit([...rows.value, { channel_id: "", hours: 24 }]);
}
function removeRow(i: number) {
  const next = [...rows.value];
  next.splice(i, 1);
  commit(next);
}
function setChannel(i: number, channelId: string) {
  const next = rows.value.map((r, idx) => (idx === i ? { ...r, channel_id: channelId } : r));
  commit(next);
}
function setHours(i: number, hours: number) {
  const h = Number.isFinite(hours) && hours > 0 ? Math.min(720, Math.round(hours)) : 1;
  const next = rows.value.map((r, idx) => (idx === i ? { ...r, hours: h } : r));
  commit(next);
}
</script>

<template>
  <div class="cse">
    <div v-if="rows.length === 0" class="cse-empty">
      Aucun salon programmé. Ajoute un salon et sa fréquence de nettoyage.
    </div>
    <div v-for="(row, i) in rows" :key="i" class="cse-row">
      <ChannelSelect
        :model-value="row.channel_id"
        :guild-id="guildId"
        @update:model-value="(v: string) => setChannel(i, v)"
      />
      <div class="cse-hours">
        <input
          type="number"
          min="1"
          max="720"
          :value="row.hours"
          @input="setHours(i, Number(($event.target as HTMLInputElement).value))"
        />
        <span class="cse-unit">h</span>
      </div>
      <button type="button" class="cse-remove" title="Retirer" @click="removeRow(i)">✕</button>
    </div>
    <button type="button" class="cse-add" @click="addRow">+ Ajouter un salon</button>
  </div>
</template>

<style scoped>
.cse { display: flex; flex-direction: column; gap: 8px; }
.cse-empty { color: var(--text-secondary); font-size: 13px; }
.cse-row { display: flex; align-items: center; gap: 8px; }
.cse-row > :first-child { flex: 1; min-width: 0; }
.cse-hours { display: flex; align-items: center; gap: 4px; }
.cse-hours input {
  width: 72px; padding: 8px 10px; border-radius: var(--radius-sm);
  border: 1px solid var(--border); background: var(--bg-primary); color: var(--text-primary);
}
.cse-unit { color: var(--text-secondary); font-size: 13px; }
.cse-remove {
  background: none; border: 1px solid var(--border); border-radius: var(--radius-sm);
  color: var(--danger); cursor: pointer; padding: 6px 9px;
}
.cse-remove:hover { border-color: var(--danger); }
.cse-add {
  align-self: flex-start; background: none; border: 1px dashed var(--border);
  border-radius: var(--radius-sm); color: var(--text-secondary); cursor: pointer; padding: 6px 12px; font-size: 13px;
}
.cse-add:hover { border-color: var(--accent); color: var(--accent); }
</style>
