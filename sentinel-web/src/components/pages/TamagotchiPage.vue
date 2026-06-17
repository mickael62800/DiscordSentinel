<script setup lang="ts">
import { ref, computed, onMounted, watch } from "vue";
import { tamagotchiService, type Pet } from "@/services/tamagotchiService";
import { useGuildSelector } from "../../composables/useGuildSelector";
import { useConfirm } from "../../composables/useConfirm";
import { useToast } from "../../composables/useToast";
import AppButton from "../atoms/AppButton.vue";
import LoadingState from "../atoms/LoadingState.vue";
import ErrorState from "../atoms/ErrorState.vue";
import EmptyState from "../atoms/EmptyState.vue";
import AdminPageShell from "../layouts/AdminPageShell.vue";

const { selectedGuildId } = useGuildSelector();
const { confirm } = useConfirm();
const { success, error: toastError } = useToast();

const pets = ref<Pet[]>([]);
const loading = ref(false);
const error = ref<string | null>(null);
const search = ref("");

const SPECIES_EMOJI: Record<string, string> = {
  loup: "🐺",
  sanglier: "🐗",
  renard: "🦊",
  tortue: "🐢",
  lapin: "🐰",
  ours: "🐻",
};

/** Doit refleter stage_from_level cote bot (bebe<=4, jeune 5-14, adulte 15-29, vieux 30+). */
function stageLabel(level: number): string {
  if (level <= 4) return "Bébé";
  if (level <= 14) return "Jeune";
  if (level <= 29) return "Adulte";
  return "Vieux";
}

function statusLabel(s: string): string {
  if (s === "healthy") return "En forme";
  if (s === "sick") return "Malade";
  if (s === "dead") return "Mort";
  return s;
}

function speciesEmoji(s: string): string {
  return SPECIES_EMOJI[s] ?? "🐾";
}

const filtered = computed(() => {
  if (!search.value) return pets.value;
  const q = search.value.toLowerCase();
  return pets.value.filter(
    (p) =>
      p.name.toLowerCase().includes(q) ||
      p.owner_id.includes(q) ||
      p.species.toLowerCase().includes(q),
  );
});

const aliveCount = computed(() => pets.value.filter((p) => p.status !== "dead").length);
const deadCount = computed(() => pets.value.filter((p) => p.status === "dead").length);
const topLevel = computed(() =>
  pets.value.length > 0 ? Math.max(...pets.value.map((p) => p.level)) : 0,
);

async function fetchPets() {
  if (!selectedGuildId.value) return;
  loading.value = true;
  error.value = null;
  try {
    pets.value = await tamagotchiService.list(selectedGuildId.value);
  } catch (e) {
    error.value = String(e);
  } finally {
    loading.value = false;
  }
}

async function removePet(pet: Pet) {
  if (!selectedGuildId.value) return;
  const ok = await confirm({
    title: "Supprimer le compagnon",
    message: `Supprimer définitivement « ${pet.name} » (dresseur ${pet.owner_id}) ? Le dresseur pourra en adopter un nouveau. Action irréversible.`,
  });
  if (!ok) return;
  try {
    await tamagotchiService.delete(selectedGuildId.value, pet.id);
    success(`Compagnon « ${pet.name} » supprimé`);
    await fetchPets();
  } catch (e) {
    toastError(String(e));
  }
}

watch(selectedGuildId, fetchPets);
onMounted(fetchPets);
</script>

<template>
  <AdminPageShell title="Jeux Tamagotchi" icon="🐾">
    <template #lede>
      Évolution des compagnons de chaque dresseur — niveau, stade, état et combats.
    </template>
    <template #actions>
      <AppButton variant="secondary" @click="fetchPets" :disabled="loading">
        ↻ Rafraîchir
      </AppButton>
    </template>

    <div class="kpi-grid">
      <div class="kpi-card">
        <div class="kpi-icon">🐾</div>
        <div class="kpi-content">
          <span class="kpi-label">Compagnons</span>
          <strong class="kpi-value">{{ pets.length }}</strong>
        </div>
      </div>
      <div class="kpi-card">
        <div class="kpi-icon">💚</div>
        <div class="kpi-content">
          <span class="kpi-label">Vivants</span>
          <strong class="kpi-value">{{ aliveCount }}</strong>
        </div>
      </div>
      <div class="kpi-card">
        <div class="kpi-icon">🪦</div>
        <div class="kpi-content">
          <span class="kpi-label">Morts</span>
          <strong class="kpi-value">{{ deadCount }}</strong>
        </div>
      </div>
      <div class="kpi-card">
        <div class="kpi-icon">👑</div>
        <div class="kpi-content">
          <span class="kpi-label">Niveau max</span>
          <strong class="kpi-value">{{ topLevel }}</strong>
        </div>
      </div>
    </div>

    <div class="search-bar">
      <div class="search-input">
        <span class="search-icon">🔍</span>
        <input
          type="text"
          v-model="search"
          placeholder="Rechercher par nom, espèce ou dresseur (ID)..."
          class="input"
        />
      </div>
      <span class="count">{{ filtered.length }} compagnon(s)</span>
    </div>

    <LoadingState v-if="loading" />
    <ErrorState v-else-if="error" :message="error" @retry="fetchPets" />
    <EmptyState v-else-if="filtered.length === 0" message="Aucun compagnon trouvé" />

    <div v-else class="table-wrap">
      <table class="pets-table">
        <thead>
          <tr>
            <th>Compagnon</th>
            <th>Dresseur</th>
            <th>Niveau</th>
            <th>Stade</th>
            <th>État</th>
            <th>Jauges</th>
            <th>Combats</th>
            <th></th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="pet in filtered" :key="pet.id" :class="{ dead: pet.status === 'dead' }">
            <td>
              <div class="pet-name">
                <span class="emoji">{{ speciesEmoji(pet.species) }}</span>
                <div>
                  <strong>{{ pet.name }}</strong>
                  <span class="sub">{{ pet.species }}</span>
                </div>
              </div>
            </td>
            <td><code>{{ pet.owner_id }}</code></td>
            <td>
              <strong>{{ pet.level }}</strong>
              <span class="sub">{{ pet.xp_in_level }}/{{ pet.xp_for_level }} XP</span>
            </td>
            <td><span class="badge">{{ stageLabel(pet.level) }}</span></td>
            <td>
              <span class="status" :class="pet.status">{{ statusLabel(pet.status) }}</span>
            </td>
            <td>
              <div class="gauges">
                <span title="Faim">🍖 {{ pet.hunger }}</span>
                <span title="Bonheur">😊 {{ pet.happiness }}</span>
                <span title="Énergie">⚡ {{ pet.energy }}</span>
              </div>
            </td>
            <td>
              <span class="sub">ELO {{ pet.elo }} · {{ pet.wins }}V / {{ pet.losses }}D</span>
            </td>
            <td class="actions-cell">
              <AppButton variant="danger" size="sm" @click="removePet(pet)">
                🗑 Supprimer
              </AppButton>
            </td>
          </tr>
        </tbody>
      </table>
    </div>
  </AdminPageShell>
</template>

<style scoped>
.kpi-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
  gap: 16px;
}
.kpi-card {
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 12px;
  padding: 18px 20px;
  display: flex; align-items: center; gap: 16px;
}
.kpi-icon {
  font-size: 2rem; width: 48px; height: 48px;
  display: flex; align-items: center; justify-content: center;
  border-radius: 10px;
  background: color-mix(in srgb, var(--accent) 15%, transparent);
}
.kpi-content { display: flex; flex-direction: column; gap: 2px; }
.kpi-label {
  font-size: 0.78rem; color: var(--text-secondary);
  text-transform: uppercase; letter-spacing: 0.5px; font-weight: 500;
}
.kpi-value { font-size: 1.75rem; font-weight: 700; line-height: 1; color: var(--text); }

.search-bar { display: flex; align-items: center; gap: 16px; padding: 0 4px; }
.search-input { position: relative; flex: 1; max-width: 500px; }
.search-icon {
  position: absolute; left: 14px; top: 50%; transform: translateY(-50%);
  font-size: 0.9rem; opacity: 0.6;
}
.search-input .input { padding-left: 40px; width: 100%; }
.count { color: var(--text-secondary); font-size: 0.85rem; font-weight: 500; }
.input {
  background: var(--bg); color: var(--text);
  border: 1px solid var(--border); border-radius: 8px;
  padding: 10px 14px; font-size: 0.9rem; font-family: inherit;
  outline: none; transition: border-color var(--transition-fast);
}
.input:focus { border-color: var(--accent); }

.table-wrap {
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 12px;
  overflow-x: auto;
}
.pets-table { width: 100%; border-collapse: collapse; font-size: 0.9rem; }
.pets-table th {
  text-align: left; padding: 12px 16px;
  font-size: 0.72rem; text-transform: uppercase; letter-spacing: 0.5px;
  color: var(--text-secondary); border-bottom: 1px solid var(--border);
}
.pets-table td { padding: 12px 16px; border-bottom: 1px solid var(--border); vertical-align: middle; }
.pets-table tr:last-child td { border-bottom: none; }
.pets-table tr.dead { opacity: 0.55; }

.pet-name { display: flex; align-items: center; gap: 10px; }
.pet-name .emoji { font-size: 1.6rem; }
.pet-name strong { display: block; }
.sub { display: block; font-size: 0.75rem; color: var(--text-secondary); }
code { font-size: 0.8rem; color: var(--text-secondary); }

.badge {
  display: inline-block; padding: 2px 10px; border-radius: 999px;
  background: color-mix(in srgb, var(--accent) 18%, transparent);
  color: var(--text); font-size: 0.78rem; font-weight: 600;
}
.status { font-weight: 600; }
.status.healthy { color: #2ecc71; }
.status.sick { color: #e67e22; }
.status.dead { color: var(--text-secondary); }

.gauges { display: flex; gap: 12px; font-size: 0.82rem; }
.actions-cell { text-align: right; white-space: nowrap; }
</style>
