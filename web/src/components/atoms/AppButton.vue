<script setup lang="ts">
// Bouton de l'application. SEULE source de vérité pour l'apparence d'un
// bouton : toute classe `.btn` locale est un doublon à supprimer, pas une
// variante à ajouter.
//
// Les couleurs viennent des tokens, donc un changement de thème — y compris
// `.theme-communaute` sur le site public — s'y applique sans rien toucher ici.
//
// Pour un LIEN qui ressemble à un bouton, utiliser `ActionButton` : déguiser
// un lien en `<button>` casse le clic-milieu et l'ouverture en onglet.
defineProps<{
  variant?: "primary" | "secondary" | "ghost" | "success" | "warning" | "danger";
  /// `xs` pour les actions en ligne dans un tableau, `md` par défaut.
  /// `sm` et `small` sont synonymes : les deux existaient dans le code.
  size?: "xs" | "sm" | "md" | "small";
}>();
</script>

<template>
  <button :class="[variant ?? 'primary', size ?? 'md']">
    <slot />
  </button>
</template>

<style scoped>
button {
  cursor: pointer;
  border: none;
  border-radius: var(--radius-sm);
  font-weight: 500;
  transition: opacity var(--transition-fast);
}

button.md {
  padding: 8px 16px;
  font-size: 0.875rem;
}

button.sm,
button.small {
  padding: 4px 10px;
  font-size: 0.8rem;
}

button.xs {
  padding: 2px 8px;
  font-size: 0.75rem;
}

button.primary {
  background-color: var(--accent);
  color: white;
}

/* `--surface` et `--text` n'ont jamais existe dans les tokens : la variante
   etait donc transparente avec du texte herite, dans toute l'application. */
button.secondary {
  background-color: var(--bg-card);
  color: var(--text-primary);
  border: 1px solid var(--border);
}

/* Variante la plus repandue du back-office : transparente, cernee. Elle
   existait sous le nom `.btn` dans une douzaine de pages, chacune avec ses
   propres valeurs. */
button.ghost {
  background-color: transparent;
  color: var(--text-primary);
  border: 1px solid var(--border);
}

button.ghost:hover:not(:disabled) {
  border-color: var(--accent);
}

button.success {
  background-color: var(--success);
  color: white;
}

button.warning {
  background-color: var(--warning);
  color: white;
}

button.danger {
  background-color: var(--danger);
  color: white;
}

button:hover:not(:disabled) {
  opacity: 0.9;
}

button:focus-visible {
  outline: none;
  box-shadow: var(--focus-ring);
}

button:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
</style>
