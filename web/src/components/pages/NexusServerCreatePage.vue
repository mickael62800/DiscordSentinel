<script setup lang="ts">
// Création d'un serveur de jeu — choix du jeu puis réglages.
//
// Le formulaire est ENTIÈREMENT piloté par le `config_schema` du template
// choisi. Ajouter une option à Minecraft ou Palworld se fait donc en base,
// sans toucher à ce fichier — c'est ce qui permettra d'ajouter de nouveaux
// jeux sans redéployer le front.
//
// Deux étapes délibérées : choisir le jeu, puis le configurer. Tout afficher
// d'un coup noierait l'utilisateur sous des dizaines de champs dont la moitié
// dépendent du jeu retenu.

import { computed, ref, watch } from "vue";
import { useRouter } from "vue-router";
import { useGuildSelector } from "../../composables/useGuildSelector";
import { useAuth } from "../../composables/useAuth";
import { useToast } from "../../composables/useToast";
import {
  nexusGamesService,
  type GameTemplate,
  type TemplateField,
} from "@/services/nexusGamesService";
import AdminPageShell from "../layouts/AdminPageShell.vue";

const router = useRouter();
const { selectedGuildId, selectedGuild } = useGuildSelector();
const { user } = useAuth();
const { success, error: showError } = useToast();

const templates = ref<GameTemplate[]>([]);
const loading = ref(false);
const errorMessage = ref("");

const chosen = ref<GameTemplate | null>(null);
const name = ref("");
const memoryMb = ref<number>(0);
const cpuLimit = ref<number>(2);
const ipRevealDays = ref<number | null>(null);
/// Valeurs des champs du template, indexées par clé.
const values = ref<Record<string, string>>({});
const submitting = ref(false);

async function loadTemplates() {
  if (!selectedGuildId.value) return;
  loading.value = true;
  errorMessage.value = "";
  try {
    templates.value = await nexusGamesService.listTemplates(selectedGuildId.value);
  } catch (e) {
    errorMessage.value = e instanceof Error ? e.message : "Chargement impossible";
    templates.value = [];
  } finally {
    loading.value = false;
  }
}

/// Sélectionner un jeu pré-remplit tous ses champs avec les valeurs par
/// défaut : l'utilisateur n'a plus qu'à ajuster ce qui l'intéresse.
function choose(t: GameTemplate) {
  chosen.value = t;
  memoryMb.value = t.default_memory_mb;
  const initial: Record<string, string> = {};
  for (const f of t.config_schema ?? []) {
    initial[f.key] = f.default === undefined ? "" : String(f.default);
  }
  values.value = initial;
  if (!name.value) name.value = t.slug;
}

/// Le nom devient celui du conteneur : on impose ce que la base accepte
/// (lettres, chiffres, espaces, tirets, underscores).
const nameError = computed(() => {
  const v = name.value.trim();
  if (!v) return "Donne un nom au serveur.";
  if (v.length > 64) return "64 caractères maximum.";
  if (!/^[a-zA-Z0-9 _-]+$/.test(v)) {
    return "Lettres, chiffres, espaces, tirets et underscores uniquement.";
  }
  return "";
});

const memoryError = computed(() => {
  const t = chosen.value;
  if (!t) return "";
  if (memoryMb.value < t.min_memory_mb) return `Minimum ${t.min_memory_mb} Mo pour ce jeu.`;
  if (memoryMb.value > t.max_memory_mb) return `Maximum ${t.max_memory_mb} Mo pour ce jeu.`;
  return "";
});

const canSubmit = computed(
  () => !!chosen.value && !nameError.value && !memoryError.value && !submitting.value,
);

async function submit() {
  if (!canSubmit.value || !selectedGuildId.value || !chosen.value) return;
  submitting.value = true;
  try {
    const created = await nexusGamesService.create(selectedGuildId.value, {
      template_slug: chosen.value.slug,
      name: name.value.trim(),
      memory_mb: memoryMb.value,
      cpu_limit: cpuLimit.value,
      owner_user_id: user.value?.id ?? "",
      config: values.value,
      ip_reveal_days: ipRevealDays.value ?? undefined,
    });
    success(`Serveur « ${created.name} » créé.`);
    router.push(`/nexus/servers/${created.id}`);
  } catch (e) {
    showError(e instanceof Error ? e.message : "Création impossible");
  } finally {
    submitting.value = false;
  }
}

/// Champs regroupés par section, dans l'ordre d'apparition du schéma.
/// Un jeu peut avoir cinquante réglages : sans sections, le formulaire
/// devient illisible.
const groupes = computed(() => {
  const out: { nom: string; champs: TemplateField[] }[] = [];
  for (const f of chosen.value?.config_schema ?? []) {
    const nom = f.group || "Réglages";
    let g = out.find((x) => x.nom === nom);
    if (!g) {
      g = { nom, champs: [] };
      out.push(g);
    }
    g.champs.push(f);
  }
  return out;
});

/// Un champ booléen du schéma vaut "true"/"false" en base : on convertit pour
/// la case à cocher.
function boolValue(f: TemplateField): boolean {
  return values.value[f.key] === "true";
}
function setBool(f: TemplateField, checked: boolean) {
  values.value[f.key] = checked ? "true" : "false";
}

watch(selectedGuildId, loadTemplates, { immediate: true });
</script>

<template>
  <AdminPageShell
    title="Nouveau serveur de jeu"
    :subtitle="selectedGuild?.name ?? 'Aucun serveur sélectionné'"
  >
    <p v-if="!selectedGuildId" class="nc-hint">
      Sélectionne un serveur Discord pour créer un serveur de jeu.
    </p>

    <p v-else-if="errorMessage" class="nc-error">{{ errorMessage }}</p>

    <p v-else-if="loading" class="nc-hint">Chargement du catalogue…</p>

    <template v-else>
      <!-- Étape 1 : le jeu -->
      <h2 class="nc-step">1. Choisis le jeu</h2>
      <div class="nc-games">
        <button
          v-for="t in templates"
          :key="t.id"
          type="button"
          class="nc-game"
          :class="{ active: chosen?.id === t.id }"
          :style="t.accent_color ? { '--accent-game': `#${t.accent_color}` } : undefined"
          @click="choose(t)"
        >
          <span class="nc-game-icon">{{ t.icon || "🎮" }}</span>
          <span class="nc-game-name">{{ t.name }}</span>
          <span v-if="t.category" class="nc-game-cat">{{ t.category }}</span>
          <span class="nc-game-ram">{{ t.default_memory_mb }} Mo conseillés</span>
        </button>
      </div>

      <p v-if="!templates.length" class="nc-hint">
        Aucun jeu autorisé pour ce serveur. Vérifie la liste
        <code>allowed_templates</code> dans la configuration Nexus.
      </p>

      <!-- Étape 2 : les réglages -->
      <template v-if="chosen">
        <p v-if="chosen.description" class="nc-desc">{{ chosen.description }}</p>

        <h2 class="nc-step">2. Règle le serveur</h2>

        <div class="nc-form">
          <label class="nc-field">
            <span>Nom du serveur</span>
            <input v-model="name" type="text" maxlength="64" />
            <small v-if="nameError" class="nc-err">{{ nameError }}</small>
          </label>

          <label class="nc-field">
            <span>Mémoire allouée (Mo)</span>
            <input
              v-model.number="memoryMb"
              type="number"
              :min="chosen.min_memory_mb"
              :max="chosen.max_memory_mb"
              step="512"
            />
            <small v-if="memoryError" class="nc-err">{{ memoryError }}</small>
            <small v-else class="nc-note">
              Entre {{ chosen.min_memory_mb }} et {{ chosen.max_memory_mb }} Mo.
            </small>
          </label>

          <label class="nc-field">
            <span>Cœurs processeur</span>
            <input v-model.number="cpuLimit" type="number" min="0.5" max="32" step="0.5" />
            <small class="nc-note">
              Plafond, pas une réservation. Minecraft n'exploite quasiment qu'un
              cœur : 2 suffisent. Palworld est multithreadé : 4 sont utiles.
            </small>
          </label>

          <label class="nc-field">
            <span>Révélation de l'IP (jours)</span>
            <input v-model.number="ipRevealDays" type="number" min="0" placeholder="défaut" />
            <small class="nc-note">
              Vide = réglage du serveur Discord. 0 = adresse visible tout de suite.
            </small>
          </label>

        </div>

        <!-- Champs propres au jeu, générés depuis le schéma et regroupés. -->
        <details v-for="g in groupes" :key="g.nom" class="nc-group" open>
          <summary>{{ g.nom }}</summary>
          <div class="nc-form">
          <label v-for="f in g.champs" :key="f.key" class="nc-field">
            <span>{{ f.label || f.key }}</span>

            <select v-if="f.type === 'enum'" v-model="values[f.key]">
              <option v-for="o in f.options ?? []" :key="o" :value="o">{{ o }}</option>
            </select>

            <input
              v-else-if="f.type === 'boolean'"
              type="checkbox"
              class="nc-check"
              :checked="boolValue(f)"
              @change="setBool(f, ($event.target as HTMLInputElement).checked)"
            />

            <input
              v-else-if="f.type === 'number'"
              v-model="values[f.key]"
              type="number"
              :min="f.min"
              :max="f.max"
            />

            <input
              v-else
              v-model="values[f.key]"
              type="text"
              :maxlength="f.max_length"
            />

            <small v-if="f.description" class="nc-note">{{ f.description }}</small>
          </label>
          </div>
        </details>

        <div class="nc-actions">
          <button type="button" class="nc-submit" :disabled="!canSubmit" @click="submit">
            {{ submitting ? "Création…" : "Créer le serveur" }}
          </button>
          <RouterLink to="/nexus/servers" class="nc-cancel">Annuler</RouterLink>
        </div>

        <p class="nc-warn">
          Le conteneur est créé à l'arrêt. Il faudra le démarrer depuis la liste
          des serveurs — la première image peut mettre plusieurs minutes à se
          télécharger.
        </p>
      </template>
    </template>
  </AdminPageShell>
</template>

<style scoped>
.nc-hint,
.nc-desc,
.nc-note {
  color: var(--text-secondary);
}

.nc-error,
.nc-err {
  color: var(--danger);
}

.nc-step {
  font-size: 1.05rem;
  margin: var(--space-lg) 0 var(--space-sm);
}

.nc-step:first-of-type {
  margin-top: 0;
}

.nc-games {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(11rem, 1fr));
  gap: var(--space-sm);
}

.nc-game {
  --accent-game: var(--accent);
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 2px;
  padding: var(--space-md);
  background: var(--bg-card);
  border: 1px solid var(--bg-hover);
  border-radius: var(--radius-md);
  color: var(--text-primary);
  cursor: pointer;
  text-align: left;
  transition: var(--transition-fast);
}

.nc-game:hover {
  border-color: var(--accent-game);
}

.nc-game.active {
  border-color: var(--accent-game);
  box-shadow: 0 0 0 1px var(--accent-game) inset;
}

.nc-game-icon {
  font-size: 1.5rem;
}

.nc-game-name {
  font-weight: 600;
}

.nc-game-cat,
.nc-game-ram {
  font-size: 0.78rem;
  color: var(--text-secondary);
}

.nc-desc {
  margin: var(--space-sm) 0 0;
  font-size: 0.9rem;
}

.nc-form {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(16rem, 1fr));
  gap: var(--space-md);
}

.nc-field {
  display: flex;
  flex-direction: column;
  gap: 4px;
  font-size: 0.9rem;
}

.nc-field > span {
  color: var(--text-secondary);
}

.nc-field input,
.nc-field select {
  background: var(--bg-card);
  border: 1px solid var(--bg-hover);
  border-radius: var(--radius-sm);
  color: var(--text-primary);
  padding: 6px 10px;
}

.nc-field input:focus,
.nc-field select:focus {
  outline: none;
  border-color: var(--accent);
}

.nc-check {
  width: 1.1rem;
  height: 1.1rem;
  align-self: flex-start;
}

.nc-field small {
  font-size: 0.76rem;
}

.nc-group {
  margin-top: var(--space-md);
  border: 1px solid var(--bg-hover);
  border-radius: var(--radius-md);
  padding: var(--space-sm) var(--space-md);
}

.nc-group > summary {
  cursor: pointer;
  font-weight: 600;
  color: var(--text-primary);
}

.nc-group > .nc-form {
  margin-top: var(--space-md);
}

.nc-actions {
  display: flex;
  align-items: center;
  gap: var(--space-md);
  margin-top: var(--space-lg);
}

.nc-submit {
  background: var(--accent);
  border: none;
  color: #fff;
  border-radius: var(--radius-md);
  padding: 8px 20px;
  font-weight: 600;
  cursor: pointer;
}

.nc-submit:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.nc-cancel {
  color: var(--text-secondary);
}

.nc-warn {
  margin-top: var(--space-md);
  font-size: 0.84rem;
  color: var(--text-secondary);
}
</style>
