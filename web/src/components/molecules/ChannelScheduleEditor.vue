<script setup lang="ts">
// Editeur « salon + frequence » : liste de couples { salon, heures }.
// La valeur est serialisee en JSON : [{"channel_id":"123","hours":24}, ...].
//
// Etat LOCAL (pas derive uniquement de modelValue) : sinon une ligne vide
// ajoutee (channel_id="") serait immediatement filtree a la serialisation et
// « Ajouter » n'afficherait jamais rien. On garde les lignes vides en local et
// on ne serialise que les lignes completes.
import { ref, watch } from "vue";
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

function parse(v: string): Row[] {
  if (!v?.trim()) return [];
  try {
    const arr = JSON.parse(v);
    if (!Array.isArray(arr)) return [];
    return arr
      .filter((r) => r && typeof r === "object")
      .map((r) => ({
        channel_id: String(r.channel_id ?? ""),
        hours: Number(r.hours) > 0 ? Number(r.hours) : 24,
      }));
  } catch {
    return [];
  }
}

const rows = ref<Row[]>(parse(props.modelValue));

// Re-sync si la valeur change de l'exterieur (chargement / annulation), mais
// pas quand c'est nous qui venons d'emettre (comparaison sur le serialise).
watch(
  () => props.modelValue,
  (v) => {
    if (v !== serialize(rows.value)) rows.value = parse(v);
  },
);

function serialize(list: Row[]): string {
  return JSON.stringify(list.filter((r) => r.channel_id));
}
function commit() {
  emit("update:modelValue", serialize(rows.value));
}

function addRow() {
  rows.value.push({ channel_id: "", hours: 24 });
  // Pas de commit : la ligne vide n'est pas encore serialisee (pas de salon).
}
function removeRow(i: number) {
  rows.value.splice(i, 1);
  commit();
}
function setChannel(i: number, channelId: string) {
  rows.value[i].channel_id = channelId;
  commit();
}
function setHours(i: number, hours: number) {
  rows.value[i].hours =
    Number.isFinite(hours) && hours > 0 ? Math.min(720, Math.round(hours)) : 1;
  commit();
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
