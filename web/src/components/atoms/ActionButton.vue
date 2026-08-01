<script setup lang="ts">
// Action principale du site public : bouton, lien interne ou lien externe.
//
// `AppButton` ne rend qu'un `<button>`. Or la moitié des actions du site
// public sont des LIENS — rejoindre Discord, aller à l'espace membre — et les
// déguiser en boutons casse la navigation : plus de clic-milieu, plus
// d'ouverture dans un onglet, plus rien pour un lecteur d'écran.
//
// Cet atome choisit donc l'élément selon ce qu'on lui donne, et ne partage
// que l'apparence. C'est la seule différence légitime avec `AppButton` ; tout
// le reste (couleurs, rayons) vient des mêmes tokens, donc le thème
// `.theme-communaute` s'y applique sans rien de particulier.

import { computed } from "vue";

const props = defineProps<{
  /// Destination interne (vue-router). Rend un `<RouterLink>`.
  to?: string;
  /// Destination externe. Rend un `<a target="_blank">`.
  href?: string;
  variant?: "primary" | "secondary" | "ghost";
  size?: "md" | "lg";
  disabled?: boolean;
}>();

/// Sans `to` ni `href`, c'est un vrai bouton : l'appelant écoutera `@click`.
const balise = computed(() => {
  if (props.to) return "RouterLink";
  if (props.href) return "a";
  return "button";
});

/// Un lien externe s'ouvre dans un onglet neuf, avec les garde-fous d'usage :
/// `noopener` empêche la page cible d'accéder à la nôtre via `window.opener`.
const attributs = computed(() => {
  if (props.to) return { to: props.to };
  if (props.href) {
    return { href: props.href, target: "_blank", rel: "noopener noreferrer" };
  }
  return { type: "button", disabled: props.disabled };
});
</script>

<template>
  <component
    :is="balise"
    v-bind="attributs"
    class="action"
    :class="[variant ?? 'primary', size ?? 'md']"
  >
    <slot />
  </component>
</template>

<style scoped>
.action {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: var(--space-sm);
  border: 1px solid transparent;
  border-radius: var(--radius-pill);
  font: inherit;
  font-weight: 600;
  text-decoration: none;
  cursor: pointer;
  transition:
    border-color var(--transition-fast),
    color var(--transition-fast),
    transform var(--transition-fast);
}

.action.md {
  padding: 0.5rem 1.3rem;
  font-size: 0.9rem;
}

.action.lg {
  padding: 0.8rem 2rem;
  font-size: 1rem;
}

.action.primary {
  background: linear-gradient(135deg, var(--accent), var(--accent-hover));
  color: #fff;
}

.action.secondary {
  background: var(--bg-card);
  border-color: var(--border);
  color: var(--text-primary);
}

.action.ghost {
  background: none;
  border-color: var(--border);
  color: var(--text-secondary);
}

.action:hover:not(:disabled) {
  border-color: var(--accent);
  color: #fff;
  transform: translateY(-1px);
}

/* Le dégradé porte déjà l'emphase : un survol qui change sa bordure le rendrait
   flottant. Seul le léger soulèvement demeure. */
.action.primary:hover:not(:disabled) {
  border-color: transparent;
}

.action:focus-visible {
  outline: none;
  box-shadow: var(--focus-ring);
}

.action:disabled {
  opacity: 0.5;
  cursor: default;
  transform: none;
}

@media (prefers-reduced-motion: reduce) {
  .action {
    transition: none;
  }

  .action:hover:not(:disabled) {
    transform: none;
  }
}
</style>
