<script setup lang="ts">
// Bouton carré ne contenant qu'une icône : fermer, rafraîchir, supprimer.
//
// `AppButton` ne convient pas ici. Un bouton d'icône est un carré à
// proportions fixes, pas un rectangle dimensionné par son texte — le forcer
// dans l'atome générique aurait demandé une variante qui contredit tout le
// reste de ses règles.
//
// `label` est OBLIGATOIRE, et c'est délibéré : une icône seule ne dit rien à
// un lecteur d'écran, et « ✕ » n'est pas un intitulé. Les six variantes
// locales que ce composant remplace n'en avaient aucune.

defineProps<{
  /// Intitulé accessible, également affiché en infobulle.
  label: string;
  variant?: "neutral" | "danger";
  size?: "sm" | "md";
}>();
</script>

<template>
  <button
    type="button"
    class="icon-btn"
    :class="[variant ?? 'neutral', size ?? 'md']"
    :title="label"
    :aria-label="label"
  >
    <slot />
  </button>
</template>

<style scoped>
.icon-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex: none;
  background: transparent;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  color: var(--text-secondary);
  cursor: pointer;
  transition:
    border-color var(--transition-fast),
    color var(--transition-fast);
}

.icon-btn.md {
  width: 32px;
  height: 32px;
  font-size: 15px;
}

.icon-btn.sm {
  width: 26px;
  height: 26px;
  font-size: 13px;
}

.icon-btn.neutral:hover:not(:disabled) {
  border-color: var(--accent);
  color: var(--text-primary);
}

/* L'action destructrice ne se colore qu'au survol : rouge en permanence, une
   rangée de corbeilles dans un tableau crierait au danger en continu. */
.icon-btn.danger:hover:not(:disabled) {
  border-color: var(--danger);
  color: var(--danger);
}

.icon-btn:focus-visible {
  outline: none;
  box-shadow: var(--focus-ring);
}

.icon-btn:disabled {
  opacity: 0.5;
  cursor: default;
}
</style>
