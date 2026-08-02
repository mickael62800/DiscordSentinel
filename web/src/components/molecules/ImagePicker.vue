<script setup lang="ts">
// Choix d'une image parmi celles livrées avec le site, sans manipuler d'URL.
//
// Le champ était une saisie libre : il fallait connaître l'adresse exacte du
// fichier, la taper sans faute, et espérer que Discord la trouve. Le
// sélecteur supprime les trois problèmes d'un coup — on choisit un nom, la
// vignette confirme le choix, et l'URL absolue est calculée.
//
// La saisie libre reste accessible : une image hébergée ailleurs doit rester
// possible, c'est le cas d'usage qui a fait choisir un champ texte au départ.

import { computed, ref, watch } from "vue";
import {
  BANNIERES,
  banniereDepuisUrl,
  categories,
  cheminBanniere,
  urlBanniere,
} from "@/data/bannieres";
import AppInput from "../atoms/AppInput.vue";

const props = defineProps<{
  modelValue: string;
  /// Sans image : `true` masque l'option « Aucune ».
  requis?: boolean;
  /**
   * Forme de l'URL enregistrée. Les deux destinations ont des exigences
   * OPPOSÉES, d'où le choix explicite :
   *
   * - `absolue` (défaut) — l'image part dans un embed Discord, et c'est
   *   Discord qui va la chercher sur Internet. Un chemin relatif ne lui dit
   *   rien : l'embed s'afficherait sans image et sans erreur.
   * - `relative` — l'image est affichée par le site. L'API REFUSE une URL
   *   absolue : elle figerait le domaine en base et ouvrirait la porte à un
   *   `javascript:` dans un attribut `src`.
   */
  mode?: "absolue" | "relative";
}>();

const emit = defineEmits<{ "update:modelValue": [value: string] }>();

/// Une URL qui ne correspond à aucune bannière connue est forcément externe :
/// on ouvre directement la saisie libre plutôt que d'afficher un sélecteur
/// vide qui donnerait l'impression d'avoir perdu la valeur.
const externe = ref(Boolean(props.modelValue) && !banniereDepuisUrl(props.modelValue));

watch(
  () => props.modelValue,
  (v) => {
    if (v && !banniereDepuisUrl(v)) externe.value = true;
  },
);

const choisie = computed(() => banniereDepuisUrl(props.modelValue));
const fichierChoisi = computed(() => choisie.value?.fichier ?? "");

const groupes = computed(() =>
  categories().map((c) => ({
    nom: c,
    images: BANNIERES.filter((b) => b.categorie === c),
  })),
);

function selectionner(e: Event) {
  const fichier = (e.target as HTMLSelectElement).value;
  if (!fichier) {
    emit("update:modelValue", "");
    return;
  }
  const url = props.mode === "relative" ? cheminBanniere(fichier) : urlBanniere(fichier);
  emit("update:modelValue", url);
}

function basculer() {
  externe.value = !externe.value;
  // On repart de zéro : garder l'ancienne valeur ferait croire qu'elle
  // s'applique encore au mode qu'on vient de quitter.
  emit("update:modelValue", "");
}
</script>

<template>
  <div class="ip">
    <template v-if="!externe">
      <select class="ip-select" :value="fichierChoisi" @change="selectionner">
        <option v-if="!requis" value="">Aucune image</option>
        <optgroup v-for="g in groupes" :key="g.nom" :label="g.nom">
          <option v-for="b in g.images" :key="b.fichier" :value="b.fichier">
            {{ b.libelle }}
          </option>
        </optgroup>
      </select>

      <figure v-if="choisie" class="ip-apercu">
        <img :src="cheminBanniere(choisie.fichier)" :alt="choisie.libelle" loading="lazy" />
      </figure>
    </template>

    <AppInput
      v-else
      :model-value="modelValue"
      :placeholder="mode === 'relative' ? '/imgs/mon-image.jpg' : 'https://...'"
      @update:model-value="emit('update:modelValue', String($event))"
    />

    <button type="button" class="ip-bascule" @click="basculer">
      {{
        externe
          ? "Choisir une image du site"
          : mode === "relative"
            ? "Saisir un chemin manuellement"
            : "Utiliser une URL externe"
      }}
    </button>
  </div>
</template>

<style scoped>
.ip {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.ip-select {
  width: 100%;
  padding: 8px 10px;
  background: var(--bg-primary);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  color: var(--text-primary);
  font-size: 14px;
}

/* La vignette confirme le choix : le libellé seul laisse un doute sur
   l'image qu'on vient de sélectionner. */
.ip-apercu {
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  overflow: hidden;
  line-height: 0;
}

.ip-apercu img {
  width: 100%;
  max-height: 120px;
  object-fit: cover;
}

.ip-bascule {
  align-self: flex-start;
  background: none;
  border: none;
  padding: 0;
  font-size: 12px;
  color: var(--text-secondary);
  cursor: pointer;
  text-decoration: underline;
}

.ip-bascule:hover {
  color: var(--accent);
}
</style>
