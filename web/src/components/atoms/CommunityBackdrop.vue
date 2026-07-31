<script setup lang="ts">
// Fond commun aux pages publiques (accueil, espace membre).
//
// Remplace les deux halos circulaires flous, qui se lisaient comme deux taches
// posées sur la page. Ici trois couches qui se composent :
//   1. une aurore : dégradés coniques très étalés, sans contour identifiable ;
//   2. une trame de points, masquée en fondu vers les bords, qui donne de la
//      matière sans attirer l'œil ;
//   3. un grain léger, en SVG inline, qui casse les bandes de dégradé
//      visibles sur les grands aplats sombres.
//
// Tout est en CSS et en SVG inline : aucune image à télécharger, aucun coût
// réseau, et ça s'adapte à n'importe quelle taille d'écran.

withDefaults(
  defineProps<{
    /// Intensité globale, pour nuancer selon la page.
    intensity?: number;
  }>(),
  { intensity: 1 },
);
</script>

<template>
  <div class="backdrop" :style="{ '--intensity': intensity }" aria-hidden="true">
    <div class="backdrop-aurora"></div>
    <div class="backdrop-dots"></div>
    <svg class="backdrop-grain" xmlns="http://www.w3.org/2000/svg">
      <filter id="grain">
        <feTurbulence type="fractalNoise" baseFrequency="0.8" numOctaves="3" />
      </filter>
      <rect width="100%" height="100%" filter="url(#grain)" />
    </svg>
  </div>
</template>

<style scoped>
.backdrop {
  position: fixed;
  inset: 0;
  pointer-events: none;
  z-index: 0;
  overflow: hidden;
}

/* ── 1. Aurore ── */
.backdrop-aurora {
  position: absolute;
  /* Débord volontaire : les dégradés doivent mourir hors de l'écran, sinon on
     devine leur bord et l'effet retombe sur « une tache ». */
  inset: -30%;
  opacity: calc(0.55 * var(--intensity));
  background:
    radial-gradient(ellipse 70% 50% at 20% 15%, rgba(124, 58, 237, 0.55), transparent 60%),
    radial-gradient(ellipse 60% 45% at 82% 30%, rgba(192, 38, 211, 0.4), transparent 62%),
    radial-gradient(ellipse 80% 40% at 50% 92%, rgba(88, 28, 180, 0.45), transparent 65%);
  filter: blur(40px);
  animation: drift 34s ease-in-out infinite alternate;
}

@keyframes drift {
  from {
    transform: translate3d(-2%, -1%, 0) scale(1);
  }
  to {
    transform: translate3d(2%, 2%, 0) scale(1.08);
  }
}

/* ── 2. Trame de points ── */
.backdrop-dots {
  position: absolute;
  inset: 0;
  opacity: calc(0.5 * var(--intensity));
  background-image: radial-gradient(rgba(216, 199, 245, 0.35) 1px, transparent 1px);
  background-size: 28px 28px;
  /* Fondu vers les bords : la trame ne doit jamais toucher le cadre, sinon
     elle ressemble à un papier peint. */
  mask-image: radial-gradient(ellipse 70% 60% at 50% 40%, #000 20%, transparent 78%);
  -webkit-mask-image: radial-gradient(ellipse 70% 60% at 50% 40%, #000 20%, transparent 78%);
}

/* ── 3. Grain ── */
.backdrop-grain {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  /* Très discret : il sert à casser les bandes de dégradé sur les aplats
     sombres, pas à texturer la page. */
  opacity: calc(0.035 * var(--intensity));
  mix-blend-mode: overlay;
}

@media (prefers-reduced-motion: reduce) {
  .backdrop-aurora {
    animation: none;
  }
}
</style>
