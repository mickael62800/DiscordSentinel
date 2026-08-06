<script setup lang="ts">
// Éditeur des cases de la Roue du Destin.
//
// Les dix cases étaient des constantes du code : changer la rareté de la
// licorne ou ajouter une case demandait de recompiler, ce qui revient à ne
// jamais le faire.
//
// Deux choses guident l'écran :
//
//   - le poids ne se lit pas seul. « 3 » ne veut rien dire ; « 2,8 % de
//     chances » se compare à une intuition. La part est donc calculée en
//     continu, à côté de chaque case.
//   - l'espérance de gain d'un tirage est la vraie question d'équilibrage.
//     Elle est affichée en bas : c'est ce qui dit si la roue enrichit ou
//     appauvrit le serveur sur la durée.

import { computed, ref, watch } from "vue";

import AdminPageShell from "../layouts/AdminPageShell.vue";
import ActionButton from "../atoms/ActionButton.vue";
import { useGuildSelector } from "../../composables/useGuildSelector";
import {
  chancePercent,
  nexusWheelService,
  type WheelCase,
} from "@/services/nexusWheelService";

const { selectedGuildId, selectedGuild } = useGuildSelector();

const cases = ref<WheelCase[]>([]);
const customized = ref(false);
const loading = ref(false);
const saving = ref(false);
const errorMessage = ref("");
const successMessage = ref("");

const MAX_CASES = 25;

/// Espérance de gain d'un tirage : somme des gains pondérés par leur chance.
/// Positive, la roue distribue plus qu'elle ne reprend.
const esperance = computed(() => {
  const total = cases.value.reduce((s, c) => s + Math.max(0, c.weight), 0);
  if (total <= 0) return 0;
  return cases.value.reduce((s, c) => s + (c.payout * Math.max(0, c.weight)) / total, 0);
});

/// Les erreurs de saisie, listées avant l'envoi. Le serveur revalide — mais
/// se faire refuser sans savoir laquelle des dix lignes est fautive serait
/// pénible.
const problemes = computed(() => {
  const out: string[] = [];
  if (!cases.value.length) out.push("Il faut au moins une case.");
  if (cases.value.length > MAX_CASES) out.push(`${MAX_CASES} cases au maximum.`);
  const vues = new Set<string>();
  for (const c of cases.value) {
    const key = c.key.trim();
    if (!key) out.push("Une case n'a pas d'identifiant.");
    else if (vues.has(key)) out.push(`Identifiant en double : ${key}`);
    else vues.add(key);
    if (!c.label.trim()) out.push(`La case ${key || "?"} n'a pas de libellé.`);
    if (c.weight < 1) out.push(`La case ${key || "?"} ne sortirait jamais (poids nul).`);
  }
  return [...new Set(out)];
});

async function charger() {
  if (!selectedGuildId.value) {
    cases.value = [];
    return;
  }
  loading.value = true;
  errorMessage.value = "";
  successMessage.value = "";
  try {
    const roue = await nexusWheelService.list(selectedGuildId.value);
    cases.value = roue.cases;
    customized.value = roue.customized;
  } catch (e) {
    errorMessage.value = e instanceof Error ? e.message : "Chargement impossible";
  } finally {
    loading.value = false;
  }
}

function ajouter() {
  cases.value.push({ key: "", label: "", payout: 0, weight: 1 });
}

function retirer(index: number) {
  cases.value.splice(index, 1);
}

function deplacer(index: number, pas: number) {
  const cible = index + pas;
  if (cible < 0 || cible >= cases.value.length) return;
  const [item] = cases.value.splice(index, 1);
  cases.value.splice(cible, 0, item);
}

async function enregistrer() {
  if (!selectedGuildId.value || problemes.value.length) return;
  saving.value = true;
  errorMessage.value = "";
  successMessage.value = "";
  try {
    const roue = await nexusWheelService.replace(selectedGuildId.value, cases.value);
    cases.value = roue.cases;
    customized.value = roue.customized;
    successMessage.value = "Roue enregistrée.";
  } catch (e) {
    errorMessage.value = e instanceof Error ? e.message : "Enregistrement impossible";
  } finally {
    saving.value = false;
  }
}

/// Revenir à la roue d'origine : on efface la personnalisation, l'API renvoie
/// les dix cases historiques.
async function restaurer() {
  if (!selectedGuildId.value) return;
  if (!window.confirm("Revenir à la roue d'origine ? Tes cases seront perdues.")) return;
  saving.value = true;
  errorMessage.value = "";
  try {
    const roue = await nexusWheelService.replace(selectedGuildId.value, []);
    cases.value = roue.cases;
    customized.value = roue.customized;
    successMessage.value = "Roue d'origine restaurée.";
  } catch (e) {
    errorMessage.value = e instanceof Error ? e.message : "Restauration impossible";
  } finally {
    saving.value = false;
  }
}

function part(c: WheelCase): string {
  return `${chancePercent(cases.value, c.weight).toFixed(1)} %`;
}

watch(selectedGuildId, charger, { immediate: true });
</script>

<template>
  <AdminPageShell
    title="Roue du Destin"
    :subtitle="selectedGuild?.name ?? 'Aucun serveur sélectionné'"
  >
    <p v-if="!selectedGuildId" class="rw-hint">
      Sélectionne un serveur Discord pour régler sa roue.
    </p>
    <p v-else-if="loading" class="rw-hint">Chargement…</p>

    <template v-else>
      <p class="rw-etat">
        <template v-if="customized">
          Ce serveur a sa propre roue.
          <button class="rw-lien" type="button" @click="restaurer">
            Revenir à la roue d'origine
          </button>
        </template>
        <template v-else>
          Ce serveur joue la roue d'origine. La modifier créera sa propre roue.
        </template>
      </p>

      <table class="rw-table">
        <thead>
          <tr>
            <th>Ordre</th>
            <th>Identifiant</th>
            <th>Libellé affiché</th>
            <th>Gain</th>
            <th>Poids</th>
            <th>Chance</th>
            <th></th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="(c, i) in cases" :key="i">
            <td class="rw-ordre">
              <button type="button" :disabled="i === 0" @click="deplacer(i, -1)">↑</button>
              <button
                type="button"
                :disabled="i === cases.length - 1"
                @click="deplacer(i, 1)"
              >
                ↓
              </button>
            </td>
            <td><input v-model="c.key" class="rw-input rw-cle" placeholder="licorne" /></td>
            <td>
              <input v-model="c.label" class="rw-input" placeholder="🦄 LICORNE — +10000c" />
            </td>
            <td>
              <input v-model.number="c.payout" type="number" class="rw-input rw-nombre" />
            </td>
            <td>
              <input
                v-model.number="c.weight"
                type="number"
                min="1"
                class="rw-input rw-nombre"
              />
            </td>
            <td class="rw-chance">{{ part(c) }}</td>
            <td>
              <button class="rw-suppr" type="button" @click="retirer(i)">✕</button>
            </td>
          </tr>
        </tbody>
      </table>

      <div class="rw-barre">
        <ActionButton variant="secondary" :disabled="cases.length >= MAX_CASES" @click="ajouter">
          Ajouter une case
        </ActionButton>
        <span class="rw-esperance">
          Gain moyen par tirage : <b>{{ Math.round(esperance) }}</b> coins
        </span>
        <ActionButton :disabled="saving || problemes.length > 0" @click="enregistrer">
          {{ saving ? "Enregistrement…" : "Enregistrer" }}
        </ActionButton>
      </div>

      <ul v-if="problemes.length" class="rw-problemes">
        <li v-for="p in problemes" :key="p">{{ p }}</li>
      </ul>
      <p v-if="errorMessage" class="rw-error">{{ errorMessage }}</p>
      <p v-if="successMessage" class="rw-ok">{{ successMessage }}</p>

      <p class="rw-note">
        Le gain moyen tient compte du poids de chaque case. Positif, la roue distribue
        plus de coins qu'elle n'en reprend — c'est ce qui fait gonfler les soldes sur la
        durée. Le multiplicateur global de la page Configuration s'applique par-dessus.
      </p>
    </template>
  </AdminPageShell>
</template>

<style scoped>
.rw-hint,
.rw-note,
.rw-etat {
  color: var(--text-secondary);
}

.rw-note {
  margin-top: var(--space-md);
  font-size: 0.86rem;
}

.rw-lien {
  background: none;
  border: none;
  padding: 0;
  color: var(--accent);
  cursor: pointer;
  text-decoration: underline;
  font: inherit;
}

.rw-table {
  width: 100%;
  border-collapse: collapse;
  margin-top: var(--space-md);
}

.rw-table th,
.rw-table td {
  text-align: left;
  padding: var(--space-xs) var(--space-sm);
  border-bottom: 1px solid var(--bg-hover);
}

.rw-table th {
  color: var(--text-secondary);
  font-weight: 600;
  font-size: 0.86rem;
}

.rw-input {
  width: 100%;
  padding: 0.35rem 0.5rem;
  background: var(--bg-card);
  border: 1px solid var(--bg-hover);
  border-radius: var(--radius-sm);
  color: var(--text-primary);
  font: inherit;
}

.rw-input:focus {
  outline: none;
  border-color: var(--accent);
}

.rw-cle {
  max-width: 9rem;
}

.rw-nombre {
  max-width: 6.5rem;
  font-variant-numeric: tabular-nums;
}

.rw-chance {
  color: var(--text-secondary);
  font-variant-numeric: tabular-nums;
  white-space: nowrap;
}

.rw-ordre button,
.rw-suppr {
  background: none;
  border: none;
  color: var(--text-secondary);
  cursor: pointer;
  font-size: 1rem;
}

.rw-ordre button:disabled {
  opacity: 0.3;
  cursor: default;
}

.rw-suppr:hover {
  color: var(--danger);
}

.rw-barre {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-md);
  flex-wrap: wrap;
  margin-top: var(--space-md);
}

.rw-esperance {
  color: var(--text-secondary);
}

.rw-problemes {
  margin-top: var(--space-sm);
  padding-left: 1.2rem;
  color: var(--danger);
}

.rw-error {
  color: var(--danger);
}

.rw-ok {
  color: var(--success);
}
</style>
