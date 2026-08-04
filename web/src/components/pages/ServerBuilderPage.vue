<script setup lang="ts">
/**
 * Constructeur de salons.
 *
 * L'idée : on compose d'abord la structure voulue — catégories, salons, ce qui
 * est privé — on la voit en entier, puis on valide UNE fois. Discord impose
 * l'inverse : créer salon par salon, régler les permissions après coup, et
 * découvrir le résultat au fur et à mesure.
 *
 * Rien n'est envoyé à Discord tant que « Créer » n'est pas cliqué : le plan
 * n'existe que dans cette page. C'est ce qui autorise à essayer un modèle, le
 * retoucher, changer d'avis.
 */
import { onBeforeUnmount, onMounted, ref, watch, computed } from "vue";
import { onBeforeRouteLeave } from "vue-router";
import { guildStructureService } from "@/services/guildStructureService";
import type {
  ExistingChannel,
  LiveRole,
  PlanItemResult,
  PlannedKind,
} from "@/services/guildStructureService";
import {
  useServerBuilder,
  TEMPLATES,
  KIND_META,
  CHILD_KINDS,
  previewName,
  type BuilderTemplate,
} from "@/composables/useServerBuilder";
import { useGuildSelector } from "@/composables/useGuildSelector";
import { useMyRole } from "@/composables/useMyRole";
import { useConfirm } from "@/composables/useConfirm";
import { useToast } from "@/composables/useToast";
import AdminPageShell from "../layouts/AdminPageShell.vue";
import AppButton from "../atoms/AppButton.vue";
import ChannelAccessEditor from "../organisms/ChannelAccessEditor.vue";
import LoadingState from "../atoms/LoadingState.vue";
import ErrorState from "../atoms/ErrorState.vue";

const { selectedGuildId } = useGuildSelector();
const { role, isSuper } = useMyRole();
const { confirm } = useConfirm();
const { success, error: toastError, info } = useToast();

const builder = useServerBuilder(() => selectedGuildId.value);

/// Créer des salons = admin (aligné sur la gate API). Supprimer un salon
/// existant détruit son historique de messages : owner seulement.
const canBuild = computed(() => isSuper.value || role.value === "owner" || role.value === "admin");
const canDelete = computed(() => isSuper.value || role.value === "owner");

// ── Structure actuelle du serveur ──
const existing = ref<ExistingChannel[]>([]);
const loading = ref(false);
const loadError = ref<string | null>(null);

const existingCategories = computed(() => existing.value.filter((c) => c.kind === "category"));
/// Discord ne renvoie pas le parent dans cette vue : on n'affiche donc pas
/// l'imbrication réelle mais deux listes (catégories / salons), ce qui suffit
/// à répondre à la seule question posée ici — « qu'est-ce qui existe déjà ? ».
const existingChannels = computed(() => existing.value.filter((c) => c.kind !== "category"));

/// Rôles du serveur, lus en direct auprès de Discord (et non dans la table
/// synchronisée) : on compose des permissions, une liste en retard ferait poser
/// des droits sur un rôle qui n'existe plus.
const roles = ref<LiveRole[]>([]);
const rolesError = ref<string | null>(null);

async function loadExisting() {
  if (!selectedGuildId.value) return;
  loading.value = true;
  loadError.value = null;
  try {
    existing.value = await guildStructureService.getStructure(selectedGuildId.value);
  } catch (e) {
    loadError.value = String(e);
  } finally {
    loading.value = false;
  }
}

async function loadRoles() {
  if (!selectedGuildId.value) return;
  rolesError.value = null;
  try {
    roles.value = await guildStructureService.getRoles(selectedGuildId.value);
  } catch (e) {
    // Non bloquant : on peut composer une structure sans toucher aux accès.
    rolesError.value = String(e);
  }
}

async function refreshAll() {
  await Promise.all([loadExisting(), loadRoles()]);
}

async function deleteExisting(chan: ExistingChannel) {
  if (!selectedGuildId.value) return;
  const ok = await confirm({
    title: `Supprimer « ${chan.name} » ?`,
    message:
      chan.kind === "category"
        ? "La catégorie sera supprimée. Ses salons ne sont pas supprimés : Discord les remonte à la racine du serveur."
        : "Le salon et TOUS ses messages seront définitivement supprimés. Cette action est irréversible.",
  });
  if (!ok) return;
  try {
    await guildStructureService.removeChannel(selectedGuildId.value, chan.id);
    success(`« ${chan.name} » supprimé.`);
    await loadExisting();
  } catch (e) {
    toastError(`Suppression : ${String(e)}`);
  }
}

// ── Application du plan ──
const applying = ref(false);
const results = ref<PlanItemResult[] | null>(null);

/// Édition du plan verrouillée : soit le rôle ne le permet pas, soit une
/// création est en cours. Laisser modifier le plan pendant l'application ferait
/// diverger ce qui est affiché de ce qui a réellement été envoyé.
const locked = computed(() => !canBuild.value || applying.value);

/// Vider le plan est irréversible et peut effacer un long travail : on
/// confirme, comme partout ailleurs dans le panel.
async function clearPlan() {
  const s = builder.summary.value;
  const ok = await confirm({
    title: "Vider le plan ?",
    message: `${s.categories} catégorie(s) et ${s.channels} salon(s) composés seront effacés. Rien n'a encore été créé sur Discord.`,
  });
  if (ok) {
    builder.reset();
    results.value = null;
  }
}

function addTemplate(tpl: BuilderTemplate) {
  builder.applyTemplate(tpl);
  info(`Modèle « ${tpl.label} » ajouté au plan — retouchez-le avant de valider.`);
}

async function apply() {
  if (!selectedGuildId.value || !builder.canApply.value) return;
  const s = builder.summary.value;
  const ok = await confirm({
    title: "Créer cette structure ?",
    message: `${s.categories} catégorie(s) et ${s.channels} salon(s) vont être créés sur le serveur Discord${
      s.private > 0 ? `, dont ${s.private} privé(s)` : ""
    }.`,
  });
  if (!ok) return;

  applying.value = true;
  results.value = null;
  try {
    const report = await guildStructureService.apply(selectedGuildId.value, builder.payload());
    results.value = report.results;
    if (report.failed === 0 && report.skipped === 0) {
      success(`${report.created} salon(s) créé(s).`);
      builder.reset();
    } else {
      toastError(
        `${report.created} créé(s), ${report.failed} en échec, ${report.skipped} ignoré(s). Détail ci-dessous.`,
      );
      // On retire ce qui est passé et on rattache les salons restants à la
      // catégorie RÉELLEMENT créée : le plan reste relançable tel quel, sans
      // recréer de doublon ni ressaisir quoi que ce soit.
      builder.dropCreated(
        report.results
          .filter((r) => r.status === "created")
          .map((r) => ({ key: r.key, channel_id: r.channel_id })),
      );
    }
    await loadExisting();
  } catch (e) {
    toastError(`Création : ${String(e)}`);
  } finally {
    applying.value = false;
  }
}

/// Paliers de mode lent proposés. Une liste courte de valeurs parlantes vaut
/// mieux qu'un champ libre en secondes, que personne ne remplit correctement.
const SLOWMODE_STEPS = [
  { value: 0, label: "Aucun" },
  { value: 5, label: "5 s" },
  { value: 30, label: "30 s" },
  { value: 60, label: "1 min" },
  { value: 300, label: "5 min" },
  { value: 3600, label: "1 h" },
  { value: 21600, label: "6 h" },
];

function kindsFor(): PlannedKind[] {
  return CHILD_KINDS;
}

/// Destinations possibles d'un salon : la racine, une catégorie du plan, ou une
/// catégorie DÉJÀ sur le serveur — ce dernier cas était impossible jusqu'ici,
/// alors qu'ajouter un salon dans une catégorie existante est le geste le plus
/// courant sur un serveur déjà en place.
const parentOptions = computed(() => [
  { value: "", label: "— Hors catégorie —" },
  ...builder.categories.value.map((c) => ({
    value: `plan:${c.key}`,
    label: `📁 ${c.name || "(sans nom)"} · à créer`,
  })),
  ...existingCategories.value.map((c) => ({
    value: `guild:${c.id}`,
    label: `📁 ${c.name} · existante`,
  })),
]);

/// Chaque serveur garde SON plan. Basculer de serveur ne détruit donc rien :
/// on range le plan courant et on ressort celui de l'autre serveur — ce qui
/// évite au passage d'appliquer des règles d'accès portant des rôles étrangers.
watch(selectedGuildId, () => {
  results.value = null;
  roles.value = [];
  builder.restore();
  void refreshAll();
});

/// Le plan est sauvegardé pour la session, mais quitter la page sans l'avoir
/// appliqué reste presque toujours une erreur : on le signale une fois.
onBeforeRouteLeave(async () => {
  if (builder.isEmpty.value || applying.value) return true;
  return await confirm({
    title: "Quitter le constructeur ?",
    message:
      "Le plan en cours n'a pas encore été créé sur Discord. Il sera conservé et vous le retrouverez en revenant.",
  });
});

/// Fermer l'onglet pendant la création laisserait une structure à moitié
/// posée : là, l'avertissement du navigateur est justifié.
function warnBeforeUnload(e: BeforeUnloadEvent) {
  if (!applying.value) return;
  e.preventDefault();
  e.returnValue = "";
}

onMounted(() => {
  window.addEventListener("beforeunload", warnBeforeUnload);
  builder.restore();
  void refreshAll();
});
onBeforeUnmount(() => window.removeEventListener("beforeunload", warnBeforeUnload));
</script>

<template>
  <AdminPageShell title="Constructeur de salons" icon="🧱" width="wide">
    <template #lede>
      Composez l'arborescence voulue — catégories, salons, accès — puis créez tout
      d'un coup sur Discord.
    </template>
    <template #actions>
      <AppButton variant="secondary" :disabled="loading" @click="refreshAll">↻ Rafraîchir</AppButton>
      <AppButton
        v-if="!builder.isEmpty.value"
        variant="secondary"
        :disabled="applying"
        @click="clearPlan"
      >
        Vider le plan
      </AppButton>
    </template>

    <div v-if="!selectedGuildId" class="empty-state">
      <p>Sélectionnez un serveur dans la barre latérale.</p>
    </div>

    <template v-else>
      <p v-if="!canBuild" class="ro-banner">
        👁 Lecture seule — la création de salons demande le rôle Administrateur.
      </p>
      <p v-if="rolesError" class="ro-banner">
        ⚠️ Rôles du serveur indisponibles ({{ rolesError }}) — la structure reste
        créable, mais les accès par rôle ne peuvent pas être réglés.
      </p>

      <div class="layout">
        <!-- ── Colonne gauche : ce qui existe déjà ── -->
        <aside class="panel existing">
          <h2 class="panel-title">Déjà sur le serveur</h2>
          <LoadingState v-if="loading" />
          <ErrorState v-else-if="loadError" :message="loadError" @retry="loadExisting" />
          <template v-else>
            <p v-if="!existing.length" class="muted small">Ce serveur n'a aucun salon.</p>
            <template v-else>
              <h3 class="sub">Catégories ({{ existingCategories.length }})</h3>
              <ul class="tree">
                <li v-for="c in existingCategories" :key="c.id" class="tree-row">
                  <span class="ico">📁</span>
                  <span class="tree-name">{{ c.name }}</span>
                  <button
                    v-if="canDelete"
                    class="del"
                    :aria-label="`Supprimer la catégorie ${c.name} du serveur`"
                    title="Supprimer cette catégorie"
                    @click="deleteExisting(c)"
                  >🗑</button>
                </li>
              </ul>
              <h3 class="sub">Salons ({{ existingChannels.length }})</h3>
              <ul class="tree">
                <li v-for="c in existingChannels" :key="c.id" class="tree-row">
                  <span class="ico">{{ c.kind === "voice" || c.kind === "stage" ? "🔊" : "#" }}</span>
                  <span class="tree-name">{{ c.name }}</span>
                  <button
                    v-if="canDelete"
                    class="del"
                    :aria-label="`Supprimer le salon ${c.name} du serveur, avec ses messages`"
                    title="Supprimer ce salon (et ses messages)"
                    @click="deleteExisting(c)"
                  >🗑</button>
                </li>
              </ul>
            </template>
          </template>
        </aside>

        <!-- ── Colonne droite : le plan en composition ── -->
        <section class="panel plan">
          <h2 class="panel-title">Votre plan</h2>

          <!-- Modèles : le point de départ -->
          <div v-if="canBuild" class="templates">
            <button
              v-for="tpl in TEMPLATES"
              :key="tpl.id"
              class="tpl"
              :title="tpl.description"
              :disabled="applying"
              @click="addTemplate(tpl)"
            >
              <span class="tpl-icon">{{ tpl.icon }}</span>
              <span class="tpl-text">
                <strong>{{ tpl.label }}</strong>
                <em>{{ tpl.description }}</em>
              </span>
            </button>
          </div>

          <div v-if="canBuild" class="add-row">
            <AppButton variant="secondary" size="sm" :disabled="applying" @click="builder.addCategory()">
              📁 Ajouter une catégorie
            </AppButton>
            <AppButton variant="secondary" size="sm" :disabled="applying" @click="builder.addChannel('text')">
              # Ajouter un salon libre
            </AppButton>
          </div>

          <p v-if="builder.isEmpty.value" class="muted plan-empty">
            Le plan est vide. Partez d'un modèle ci-dessus, ou ajoutez vos propres
            catégories et salons. Rien n'est envoyé à Discord avant de valider.
          </p>

          <!-- Catégories du plan et leurs salons -->
          <div v-for="cat in builder.categories.value" :key="cat.key" class="cat-card">
            <header class="cat-head">
              <span class="ico" aria-hidden="true">📁</span>
              <input
                v-model="cat.name"
                class="input name-input"
                :aria-label="`Nom de la catégorie ${cat.name}`"
                :disabled="locked"
              />
              <label class="opt" title="Refuse l'accès à @everyone">
                <input v-model="cat.private" type="checkbox" :disabled="locked" /> privée
              </label>
              <button
                class="del"
                :aria-label="`Retirer la catégorie ${cat.name} du plan, avec ses salons`"
                :disabled="locked"
                @click="builder.remove(cat.key)"
              >✕</button>
            </header>

            <!-- Régler l'accès ici plutôt que salon par salon : les salons
                 d'une catégorie en héritent, c'est le geste rentable. -->
            <ChannelAccessEditor
              v-if="selectedGuildId"
              :item="cat"
              :roles="roles"
              :guild-id="selectedGuildId"
              :disabled="locked"
            />

            <div v-for="child in builder.childrenOf(cat.key)" :key="child.key" class="chan-row">
              <select
                v-model="child.kind"
                class="input kind-select"
                :aria-label="`Type du salon ${child.name}`"
                :disabled="locked"
              >
                <option v-for="k in kindsFor()" :key="k" :value="k">
                  {{ KIND_META[k].icon }} {{ KIND_META[k].label }}
                </option>
              </select>
              <input
                v-model="child.name"
                class="input name-input"
                :aria-label="`Nom du salon ${child.name}`"
                :disabled="locked"
              />
              <span class="preview" title="Nom final sur Discord">→ {{ previewName(child.name, child.kind) || "…" }}</span>
              <select
                class="input parent-select"
                :value="builder.parentValue(child)"
                :aria-label="`Catégorie du salon ${child.name}`"
                :disabled="locked"
                @change="builder.setParent(child, ($event.target as HTMLSelectElement).value)"
              >
                <option v-for="o in parentOptions" :key="o.value" :value="o.value">{{ o.label }}</option>
              </select>

              <label class="opt" title="Refuse l'accès à @everyone">
                <input v-model="child.private" type="checkbox" :disabled="locked" /> privé
              </label>
              <select
                v-if="child.kind === 'text' || child.kind === 'forum'"
                v-model.number="child.slowmode"
                class="input slow-select"
                :aria-label="`Mode lent du salon ${child.name}`"
                :disabled="locked"
              >
                <option v-for="s in SLOWMODE_STEPS" :key="s.value" :value="s.value">⏱ {{ s.label }}</option>
              </select>
              <input
                v-if="child.kind === 'voice' || child.kind === 'stage'"
                v-model.number="child.user_limit"
                class="input limit-input"
                type="number"
                min="0"
                max="99"
                placeholder="∞"
                :aria-label="`Nombre maximum de participants du salon ${child.name} (vide = illimité)`"
                :disabled="locked"
              />

              <button
                class="del"
                :aria-label="`Retirer le salon ${child.name} du plan`"
                :disabled="locked"
                @click="builder.remove(child.key)"
              >✕</button>

              <ChannelAccessEditor
                v-if="selectedGuildId"
                :item="child"
                :roles="roles"
                :guild-id="selectedGuildId"
                :disabled="locked"
              />
            </div>

            <div v-if="canBuild" class="cat-add">
              <button
                v-for="k in kindsFor()"
                :key="k"
                class="mini-add"
                :title="KIND_META[k].hint"
                :aria-label="`Ajouter un ${KIND_META[k].label.toLowerCase()} dans la catégorie ${cat.name}`"
                :disabled="locked"
                @click="builder.addChannel(k, cat.key)"
              >
                + {{ KIND_META[k].icon }} {{ KIND_META[k].label }}
              </button>
            </div>
          </div>

          <!-- Salons sans catégorie -->
          <div v-if="builder.rootChannels.value.length" class="cat-card root-card">
            <header class="cat-head">
              <span class="ico" aria-hidden="true">🗂</span>
              <strong>Hors catégorie du plan</strong>
              <span class="muted small">
                — à la racine du serveur, ou dans une catégorie déjà existante
              </span>
            </header>
            <div v-for="child in builder.rootChannels.value" :key="child.key" class="chan-row">
              <select
                v-model="child.kind"
                class="input kind-select"
                :aria-label="`Type du salon ${child.name}`"
                :disabled="locked"
              >
                <option v-for="k in kindsFor()" :key="k" :value="k">
                  {{ KIND_META[k].icon }} {{ KIND_META[k].label }}
                </option>
              </select>
              <input
                v-model="child.name"
                class="input name-input"
                :aria-label="`Nom du salon ${child.name}`"
                :disabled="locked"
              />
              <span class="preview">→ {{ previewName(child.name, child.kind) || "…" }}</span>
              <select
                class="input parent-select"
                :value="builder.parentValue(child)"
                :aria-label="`Catégorie du salon ${child.name}`"
                :disabled="locked"
                @change="builder.setParent(child, ($event.target as HTMLSelectElement).value)"
              >
                <option v-for="o in parentOptions" :key="o.value" :value="o.value">{{ o.label }}</option>
              </select>
              <label class="opt"><input v-model="child.private" type="checkbox" :disabled="locked" /> privé</label>
              <button
                class="del"
                :aria-label="`Retirer le salon ${child.name} du plan`"
                :disabled="locked"
                @click="builder.remove(child.key)"
              >✕</button>

              <ChannelAccessEditor
                v-if="selectedGuildId"
                :item="child"
                :roles="roles"
                :guild-id="selectedGuildId"
                :disabled="locked"
              />
            </div>
          </div>

          <!-- Erreurs bloquantes -->
          <ul v-if="builder.errors.value.length" class="errors" role="alert">
            <li v-for="(e, i) in builder.errors.value" :key="i">⚠️ {{ e }}</li>
          </ul>

          <!-- Barre de validation -->
          <div v-if="!builder.isEmpty.value" class="apply-bar">
            <span class="summary">
              {{ builder.summary.value.categories }} catégorie(s) ·
              {{ builder.summary.value.channels }} salon(s)
              <template v-if="builder.summary.value.private">
                · {{ builder.summary.value.private }} privé(s)
              </template>
            </span>
            <AppButton
              variant="primary"
              :disabled="!canBuild || !builder.canApply.value || applying"
              @click="apply"
            >
              {{ applying ? "Création en cours…" : "✓ Créer sur Discord" }}
            </AppButton>
          </div>

          <!-- Progression : Discord limite le rythme, l'opération peut durer
               une bonne minute sur un gros plan. Sans ce retour, l'écran
               paraît figé et l'utilisateur relance ou s'en va. -->
          <div v-if="applying" class="progress" role="status" aria-live="polite">
            <span class="spinner" aria-hidden="true"></span>
            Création de {{ builder.summary.value.categories + builder.summary.value.channels }}
            élément(s) en cours — Discord limite le rythme, cela peut prendre
            jusqu'à une minute. Ne fermez pas la page.
          </div>

          <!-- Compte rendu de la dernière exécution -->
          <div v-if="results" class="results" role="status" aria-live="polite">
            <h3 class="sub">Résultat</h3>
            <ul>
              <li v-for="r in results" :key="r.key" :class="r.status">
                <span aria-hidden="true">{{ r.status === "created" ? "✅" : r.status === "skipped" ? "⏭" : "❌" }}</span>
                <strong>{{ r.name }}</strong>
                <em v-if="r.error">{{ r.error }}</em>
              </li>
            </ul>
            <p v-if="!builder.isEmpty.value" class="muted small retry-hint">
              Les éléments créés ont été retirés du plan et les salons restants
              rattachés à leur catégorie réelle : corrigez ce qui a échoué et
              relancez, aucun doublon ne sera créé.
            </p>
          </div>
        </section>
      </div>
    </template>
  </AdminPageShell>
</template>

<style scoped>
.empty-state { text-align: center; padding: 60px 20px; color: var(--text-secondary); }
.muted { color: var(--text-secondary); }
.small { font-size: 11px; }

.ro-banner {
  background: color-mix(in srgb, var(--warning, var(--accent-warm)) 10%, var(--bg-secondary));
  border-left: 3px solid var(--warning, var(--accent-warm));
  padding: 10px 14px; border-radius: var(--radius-sm);
  font-size: 13px; margin-bottom: 14px;
}

.layout { display: grid; grid-template-columns: minmax(200px, 280px) 1fr; gap: 18px; align-items: start; }
@media (max-width: 900px) { .layout { grid-template-columns: 1fr; } }

.panel {
  background: var(--bg-secondary); border: 1px solid var(--border);
  border-radius: var(--radius-lg); padding: 16px 18px;
}
.panel-title { margin: 0 0 12px; font-size: 15px; }
.sub {
  margin: 14px 0 6px; font-size: 11px; text-transform: uppercase;
  letter-spacing: 0.4px; color: var(--text-secondary);
}

/* Colonne existante */
.tree { list-style: none; margin: 0; padding: 0; max-height: 260px; overflow-y: auto; }
.tree-row { display: flex; align-items: center; gap: 8px; padding: 3px 0; font-size: 13px; }
.tree-name { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.ico { opacity: 0.7; width: 1.2em; text-align: center; flex-shrink: 0; }
/* Cible tactile décente et focus clavier visible : ce bouton supprime un salon
   Discord avec son historique, il ne doit pas être une croix fantôme. */
.del {
  background: none; border: none; cursor: pointer; opacity: 0.6;
  font-size: 0.9rem; padding: 8px; line-height: 1; color: inherit;
  border-radius: var(--radius-sm);
}
.del:hover:not(:disabled) { opacity: 1; color: var(--danger); }
.del:focus-visible { outline: 2px solid var(--danger); outline-offset: 1px; opacity: 1; }
.del:disabled { opacity: 0.25; cursor: default; }

/* Modèles */
.templates { display: grid; grid-template-columns: repeat(auto-fill, minmax(210px, 1fr)); gap: 8px; margin-bottom: 14px; }
.tpl {
  display: flex; align-items: center; gap: 10px; text-align: left;
  background: color-mix(in srgb, var(--accent) 5%, transparent);
  border: 1px solid var(--border); border-radius: var(--radius-md);
  padding: 10px 12px; cursor: pointer; color: inherit; font: inherit;
}
.tpl:hover:not(:disabled) { border-color: var(--accent); }
.tpl:disabled { opacity: 0.5; cursor: default; }
.tpl-icon { font-size: 20px; }
.tpl-text { display: flex; flex-direction: column; gap: 2px; }
.tpl-text em { font-style: normal; font-size: 11px; color: var(--text-secondary); }

.add-row { display: flex; gap: 8px; flex-wrap: wrap; margin-bottom: 14px; }
.plan-empty { font-size: 13px; line-height: 1.5; margin: 18px 0; }

/* Cartes de catégorie */
.cat-card {
  border: 1px solid var(--border); border-radius: var(--radius-md);
  padding: 10px 12px; margin-bottom: 10px;
  background: color-mix(in srgb, var(--accent) 3%, transparent);
}
.root-card { background: transparent; border-style: dashed; }
.cat-head { display: flex; align-items: center; gap: 8px; margin-bottom: 8px; }

.chan-row {
  display: flex; align-items: center; gap: 6px; flex-wrap: wrap;
  padding: 4px 0 4px 18px; font-size: 13px;
}
/* Le nom normalisé peut faire 90 caractères sans espace : sans coupure, il
   pousse la page en débordement horizontal sur mobile (motif déjà corrigé
   ailleurs, cf. RESPONSIVE_ISSUES.md). */
.preview {
  font-family: "JetBrains Mono", monospace; font-size: 11px;
  color: var(--text-secondary); min-width: 0;
  overflow-wrap: anywhere; word-break: break-all;
}
.opt { display: flex; align-items: center; gap: 4px; font-size: 12px; color: var(--text-secondary); }

/* Standard d'inputs du repo (global.css .app-input-base) : padding 8/12,
   radius 6, 13px. Redéfini ici seulement parce que ces champs sont en ligne
   et ne doivent pas prendre toute la largeur. */
.input {
  background: var(--bg-card); color: var(--text-primary);
  border: 1px solid var(--border); border-radius: 6px;
  padding: 8px 12px; font-size: 13px; font-family: inherit; outline: none;
}
.input:focus { border-color: var(--accent); box-shadow: var(--focus-ring); }
.name-input { flex: 1; min-width: 120px; }
.kind-select { width: 130px; }
.parent-select { max-width: 190px; }
.slow-select { width: 92px; }
.limit-input { width: 62px; }

.cat-add { display: flex; gap: 6px; flex-wrap: wrap; padding-left: 18px; margin-top: 6px; }
.mini-add {
  background: none; border: 1px dashed var(--border); border-radius: var(--radius-pill);
  padding: 6px 12px; font-size: 11px; cursor: pointer; color: var(--text-secondary);
}
.mini-add:hover:not(:disabled) { border-color: var(--accent); color: var(--accent); }
.mini-add:disabled { opacity: 0.4; cursor: default; }

.errors {
  list-style: none; margin: 14px 0 0; padding: 10px 14px;
  border: 1px solid var(--danger); border-radius: var(--radius-md);
  background: color-mix(in srgb, var(--danger) 6%, transparent);
  font-size: 12.5px; line-height: 1.6; color: var(--danger);
}

.apply-bar {
  display: flex; align-items: center; justify-content: space-between; gap: 14px;
  margin-top: 16px; padding-top: 14px; border-top: 1px solid var(--border);
  flex-wrap: wrap;
}
.summary { font-size: 13px; color: var(--text-secondary); }

.progress {
  display: flex; align-items: center; gap: 10px; flex-wrap: wrap;
  margin-top: 14px; padding: 10px 14px; font-size: 13px; line-height: 1.5;
  border-radius: var(--radius-md);
  background: color-mix(in srgb, var(--accent) 8%, transparent);
  border-left: 3px solid var(--accent);
}
.spinner {
  width: 14px; height: 14px; flex-shrink: 0; border-radius: 50%;
  border: 2px solid color-mix(in srgb, var(--accent) 30%, transparent);
  border-top-color: var(--accent); animation: spin 0.8s linear infinite;
}
@keyframes spin { to { transform: rotate(360deg); } }
@media (prefers-reduced-motion: reduce) { .spinner { animation: none; } }

.results { margin-top: 16px; }
.retry-hint { margin: 10px 0 0; line-height: 1.5; }
.results ul { list-style: none; margin: 0; padding: 0; font-size: 13px; }
.results li { display: flex; align-items: baseline; gap: 8px; padding: 3px 0; }
.results li em { font-style: normal; font-size: 11px; color: var(--text-secondary); }
.results li.failed em { color: var(--danger); }
</style>
