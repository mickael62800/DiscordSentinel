<script setup lang="ts">
// Case à cocher avec son libellé.
//
// Distincte d'`AppToggle`, qui est un interrupteur à glissière. Les deux ne
// disent pas la même chose : un interrupteur active un comportement (« le
// module tourne »), une case exprime un choix dans un formulaire (« inclure
// les brouillons »). Les confondre trompe sur la portée de l'action.
//
// Le libellé est DANS le `<label>` : cliquer le texte coche la case. Les
// variantes locales qu'elle remplace (`.cb`, `.checkbox`, `.toggle-row`)
// avaient chacune leur espacement, et certaines laissaient le texte hors du
// `<label>` — donc inerte au clic.

defineProps<{
  modelValue: boolean;
  disabled?: boolean;
}>();

defineEmits<{
  "update:modelValue": [value: boolean];
}>();
</script>

<template>
  <label class="cb" :class="{ disabled }">
    <input
      type="checkbox"
      :checked="modelValue"
      :disabled="disabled"
      @change="$emit('update:modelValue', ($event.target as HTMLInputElement).checked)"
    />
    <span class="cb-label"><slot /></span>
  </label>
</template>

<style scoped>
.cb {
  display: inline-flex;
  align-items: center;
  gap: var(--space-sm);
  font-size: 0.8rem;
  color: var(--text-secondary);
  cursor: pointer;
  /* Le libellé et sa case restent solidaires en fin de ligne : les séparer
     ferait porter le clic sur une case orpheline. */
  white-space: nowrap;
}

.cb.disabled {
  opacity: 0.55;
  cursor: default;
}

.cb input {
  flex: none;
  width: 15px;
  height: 15px;
  accent-color: var(--accent);
  cursor: inherit;
}

.cb-label {
  white-space: normal;
}

.cb input:focus-visible {
  outline: none;
  box-shadow: var(--focus-ring);
}
</style>
