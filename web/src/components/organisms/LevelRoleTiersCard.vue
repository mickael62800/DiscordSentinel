<script setup lang="ts">
// Paliers de rôles : à quel niveau un membre change de rôle.
//
// Le réglage est stocké comme une chaîne `niveau:role_id` séparée par des
// virgules — même format que les autres réglages du module, donc lisible
// aussi bien depuis la page du composant que d'ici. L'écran, lui, ne montre
// jamais cette chaîne : on édite des lignes, avec un vrai sélecteur de rôle.
//
// Saisir « 10:445566778899 » à la main demande d'aller chercher un identifiant
// Discord dans les paramètres du serveur. C'est précisément ce qu'un
// back-office doit éviter.

import { computed, onMounted, ref, watch } from "vue";
import { useGuildSelector } from "../../composables/useGuildSelector";
import { useToast } from "../../composables/useToast";
import { botConfigService } from "@/services/botConfigService";
import { errMsg } from "@/utils/errMsg";
import RoleSelect from "../atoms/RoleSelect.vue";
import AppToggle from "../atoms/AppToggle.vue";
import IconButton from "../atoms/IconButton.vue";

const BOT = "progression-bot";
const CLE_PALIERS = "level_role_rewards";
const CLE_MODE = "level_role_mode";

interface Palier {
  niveau: number;
  roleId: string;
}

const { selectedGuildId } = useGuildSelector();
const { success, error: showError } = useToast();

const paliers = ref<Palier[]>([]);
const remplacement = ref(false);
const chargement = ref(false);
const enregistrement = ref(false);
const messageErreur = ref("");

/// État enregistré, pour savoir s'il reste des modifications en attente.
const referenceP = ref("");
const referenceM = ref(false);

/// Sérialise dans le format attendu par le bot. Trié par niveau : la chaîne
/// relue depuis la page du composant reste lisible.
const serialise = computed(() =>
  [...paliers.value]
    .filter((p) => p.niveau > 0 && p.roleId)
    .sort((a, b) => a.niveau - b.niveau)
    .map((p) => `${p.niveau}:${p.roleId}`)
    .join(","),
);

const modifie = computed(
  () => serialise.value !== referenceP.value || remplacement.value !== referenceM.value,
);

/// Deux paliers au même niveau : le bot n'en garderait qu'un, autant le dire
/// avant d'enregistrer plutôt que de laisser disparaître une ligne.
const niveauxEnDouble = computed(() => {
  const vus = new Set<number>();
  const doubles = new Set<number>();
  for (const p of paliers.value) {
    if (p.niveau > 0 && vus.has(p.niveau)) doubles.add(p.niveau);
    vus.add(p.niveau);
  }
  return doubles;
});

/// Une ligne sans rôle n'est PAS une erreur : tous les niveaux ne donnent pas
/// un nouveau rôle. On génère l'échelle complète, on remplit les seuils qui
/// comptent, et le reste sert de repère visuel sans rien changer.
///
/// Elle n'est simplement pas enregistrée. Ce compteur le dit sans alarmer.
const sansRole = computed(() => paliers.value.filter((p) => !p.roleId).length);

/// Ce qui est vraiment faux : un rôle choisi sur un niveau invalide. Là, la
/// ligne serait perdue sans que rien ne l'explique.
const niveauxInvalides = computed(() =>
  paliers.value.some((p) => p.roleId && (!Number.isFinite(p.niveau) || p.niveau <= 0)),
);

function analyser(brut: string): Palier[] {
  return brut
    .split(/[,\n]/)
    .map((e) => e.trim())
    .filter(Boolean)
    .map((e) => {
      const [n, r] = e.split(":");
      return { niveau: Number.parseInt(n ?? "", 10), roleId: (r ?? "").trim() };
    })
    .filter((p) => Number.isFinite(p.niveau) && p.niveau > 0 && p.roleId)
    .sort((a, b) => a.niveau - b.niveau);
}

async function charger() {
  if (!selectedGuildId.value) return;
  chargement.value = true;
  messageErreur.value = "";
  try {
    const config = await botConfigService.getGuildConfig(selectedGuildId.value);
    const propre = config.filter((c) => c.bot_name === BOT);
    const brut = propre.find((c) => c.config_key === CLE_PALIERS)?.config_value ?? "";
    const mode = propre.find((c) => c.config_key === CLE_MODE)?.config_value ?? "cumulatif";

    paliers.value = analyser(brut);
    remplacement.value = mode === "remplacement";
    referenceP.value = serialise.value;
    referenceM.value = remplacement.value;
  } catch (e) {
    messageErreur.value = errMsg(e);
  } finally {
    chargement.value = false;
  }
}

function ajouter() {
  // Le niveau proposé suit le dernier palier : on ajoute presque toujours un
  // seuil plus haut que les précédents.
  const dernier = paliers.value.reduce((max, p) => Math.max(max, p.niveau), 0);
  paliers.value.push({ niveau: dernier + 5, roleId: "" });
}

function retirer(index: number) {
  paliers.value.splice(index, 1);
}

/// Bornes de l'échelle. 100 couvre largement une communauté active ; au-delà,
/// on ajoute les paliers un par un.
const echelleJusqua = ref(100);
const echellePas = ref(5);

/// Construit l'échelle : niveau 1, puis un palier tous les N niveaux.
///
/// Le niveau 1 est toujours présent — c'est le premier level-up, celui où le
/// membre doit déjà porter un rôle. Il n'est pas couvert par « Rôles attribués
/// par défaut », qui se donne à l'ARRIVÉE et ne dépend d'aucun niveau.
///
/// Les rôles déjà choisis sont conservés quand le niveau retombe sur un palier
/// existant : régénérer l'échelle après en avoir rempli la moitié ne doit pas
/// effacer le travail fait.
function genererEchelle() {
  const pas = Math.max(1, echellePas.value);
  const max = Math.min(1000, Math.max(1, echelleJusqua.value));

  const dejaChoisi = new Map(paliers.value.map((p) => [p.niveau, p.roleId]));
  const niveaux = [1];
  for (let n = pas; n <= max; n += pas) {
    if (n !== 1) niveaux.push(n);
  }

  paliers.value = niveaux.map((niveau) => ({
    niveau,
    roleId: dejaChoisi.get(niveau) ?? "",
  }));
}

async function enregistrer() {
  if (!selectedGuildId.value) return;
  enregistrement.value = true;
  try {
    await botConfigService.set(selectedGuildId.value, BOT, CLE_PALIERS, serialise.value);
    await botConfigService.set(
      selectedGuildId.value,
      BOT,
      CLE_MODE,
      remplacement.value ? "remplacement" : "cumulatif",
    );
    referenceP.value = serialise.value;
    referenceM.value = remplacement.value;
    success("Paliers enregistrés.");
  } catch (e) {
    showError(errMsg(e));
  } finally {
    enregistrement.value = false;
  }
}

watch(selectedGuildId, charger);
onMounted(charger);
</script>

<template>
  <section class="lrt">
    <header class="lrt-head">
      <h2>Paliers de rôles</h2>
      <p>À partir de quel niveau un membre reçoit quel rôle.</p>
    </header>

    <p v-if="messageErreur" class="lrt-erreur">{{ messageErreur }}</p>
    <p v-else-if="chargement" class="lrt-info">Chargement…</p>

    <template v-else>
      <p v-if="!paliers.length" class="lrt-vide">
        Aucun palier. Le rôle donné à l'arrivée se règle séparément, avec
        « Rôles attribués par défaut ».
      </p>

      <ul v-else class="lrt-liste">
        <li v-for="(p, i) in paliers" :key="i" class="lrt-ligne" :class="{ 'lrt-inactive': !p.roleId }">
          <label class="lrt-niveau">
            <span>Niveau</span>
            <input
              v-model.number="p.niveau"
              type="number"
              min="1"
              max="1000"
              :class="{ 'lrt-double': niveauxEnDouble.has(p.niveau) }"
            />
          </label>

          <span class="lrt-fleche" aria-hidden="true">→</span>

          <div class="lrt-role">
            <RoleSelect
              :model-value="p.roleId"
              :guild-id="selectedGuildId"
              @update:model-value="p.roleId = $event"
            />
          </div>

          <IconButton label="Retirer ce palier" @click="retirer(i)">✕</IconButton>
        </li>
      </ul>

      <p v-if="niveauxEnDouble.size" class="lrt-avert">
        Deux paliers portent le même niveau : un seul sera conservé.
      </p>
      <p v-if="niveauxInvalides" class="lrt-avert">
        Un rôle est posé sur un niveau invalide — cette ligne serait perdue.
      </p>
      <p v-else-if="sansRole" class="lrt-neutre">
        {{ sansRole }} niveau(x) sans rôle : normal, ils ne changent rien.
      </p>

      <div class="lrt-mode">
        <div class="lrt-mode-ligne">
          <AppToggle v-model="remplacement" />
          <span>Ne garder que le rôle du palier atteint</span>
        </div>
        <p class="lrt-mode-aide">
          <template v-if="remplacement">
            Le membre ne porte que son rang actuel. Le bot
            <strong>retire</strong> les rôles des autres paliers — n'y mets que
            des rôles dédiés à la progression, ils seront enlevés aux membres
            qui ne sont plus au bon niveau.
          </template>
          <template v-else>
            Le membre garde tous les rôles obtenus au fil des paliers.
          </template>
        </p>
      </div>

      <div class="lrt-echelle">
        <span>Créer l'échelle : niveau 1, puis tous les</span>
        <input v-model.number="echellePas" type="number" min="1" max="100" />
        <span>niveaux jusqu'au niveau</span>
        <input v-model.number="echelleJusqua" type="number" min="1" max="1000" />
        <button type="button" class="lrt-ajout" @click="genererEchelle">Générer</button>
      </div>

      <div class="lrt-actions">
        <button type="button" class="lrt-ajout" @click="ajouter">+ Ajouter un palier</button>
        <button
          type="button"
          class="lrt-save"
          :disabled="enregistrement || !modifie"
          @click="enregistrer"
        >
          {{ enregistrement ? "Enregistrement…" : modifie ? "Enregistrer" : "Aucune modification" }}
        </button>
      </div>
    </template>
  </section>
</template>

<style scoped>
.lrt {
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  padding: 20px 24px;
}

.lrt-head h2 { font-size: 16px; color: var(--text-primary); }
.lrt-head p { font-size: 13px; color: var(--text-secondary); margin-top: 2px; }

.lrt-info, .lrt-vide { font-size: 13px; color: var(--text-secondary); padding: 16px 0; }
.lrt-erreur { font-size: 13px; color: var(--danger); padding: 16px 0; }

.lrt-liste {
  list-style: none;
  display: flex;
  flex-direction: column;
  gap: 10px;
  margin: 16px 0;
}

.lrt-ligne {
  display: flex;
  align-items: center;
  gap: 12px;
  flex-wrap: wrap;
}

.lrt-inactive { opacity: 0.55; }
.lrt-inactive:focus-within { opacity: 1; }

.lrt-niveau {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 13px;
  color: var(--text-secondary);
}

.lrt-niveau input {
  width: 80px;
  padding: 6px 8px;
  background: var(--bg-primary);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  color: var(--text-primary);
  font-variant-numeric: tabular-nums;
}

.lrt-double { border-color: var(--warning, #e67e22); }

.lrt-fleche { color: var(--text-secondary); }
.lrt-role { flex: 1 1 240px; min-width: 200px; }

.lrt-avert {
  font-size: 12px;
  color: var(--warning, #e67e22);
  margin-bottom: 8px;
}

.lrt-neutre {
  font-size: 12px;
  color: var(--text-secondary);
  margin-bottom: 8px;
}

.lrt-mode {
  border-top: 1px solid var(--border);
  padding-top: 16px;
  margin-top: 8px;
}

.lrt-mode-ligne {
  display: flex;
  align-items: center;
  gap: 10px;
  font-size: 13px;
  color: var(--text-primary);
}

.lrt-mode-aide {
  font-size: 12px;
  color: var(--text-secondary);
  line-height: 1.5;
  margin-top: 6px;
}

.lrt-echelle {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 8px;
  font-size: 13px;
  color: var(--text-secondary);
  border-top: 1px solid var(--border);
  padding-top: 16px;
  margin-top: 16px;
}

.lrt-echelle input {
  width: 70px;
  padding: 6px 8px;
  background: var(--bg-primary);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  color: var(--text-primary);
  font-variant-numeric: tabular-nums;
}

.lrt-actions {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-top: 20px;
}

.lrt-ajout, .lrt-save {
  padding: 8px 16px;
  border-radius: var(--radius-sm);
  font-size: 13px;
  cursor: pointer;
  border: 1px solid var(--border);
  background: var(--bg-primary);
  color: var(--text-primary);
}

.lrt-save {
  background: var(--accent);
  border-color: var(--accent);
  color: #fff;
  margin-left: auto;
}

.lrt-save:disabled {
  opacity: 0.5;
  cursor: default;
}
</style>
