<script setup lang="ts">
// Section de présentation : un texte, des points clés, une illustration.
//
// L'accueil en enchaîne six. L'illustration alterne de côté à chaque section
// pour que la page respire au défilement — une colonne d'images toutes du même
// côté se lit comme un tableau.
//
// Une image absente ne laisse pas de trou : le bloc se retire et le texte
// reprend toute la largeur. C'est le comportement voulu tant que les visuels
// ne sont pas tous livrés.

defineProps<{
  surtitre: string;
  titre: string;
  texte: string;
  points?: string[];
  image?: string;
  alt?: string;
  /// Vrai = illustration à gauche. Piloté par l'index de la boucle appelante.
  inverse?: boolean;
}>();

function onImageError(event: Event): void {
  const el = event.target as HTMLImageElement | null;
  el?.closest(".sec-media")?.setAttribute("hidden", "true");
}
</script>

<template>
  <section class="sec" :class="{ reverse: inverse }">
    <div class="sec-text">
      <span class="sec-sur">{{ surtitre }}</span>
      <h2>{{ titre }}</h2>
      <p>{{ texte }}</p>

      <ul v-if="points?.length">
        <li v-for="pt in points" :key="pt">{{ pt }}</li>
      </ul>
    </div>

    <figure v-if="image" class="sec-media">
      <img :src="image" :alt="alt ?? ''" loading="lazy" @error="onImageError" />
    </figure>
  </section>
</template>

<style scoped>
.sec {
  display: grid;
  grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
  align-items: center;
  gap: clamp(var(--space-xl), 5vw, var(--space-3xl));
}

/* L'ordre visuel s'inverse sans toucher à l'ordre du DOM : le texte reste lu
   en premier par un lecteur d'écran, quelle que soit la mise en page. */
.sec.reverse .sec-text {
  order: 2;
}

.sec-sur {
  display: block;
  margin-bottom: var(--space-sm);
  color: var(--accent);
  font-size: 0.8rem;
  font-weight: 700;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}

.sec-text h2 {
  margin: 0 0 var(--space-md);
  font-size: clamp(1.4rem, 3vw, 1.9rem);
  line-height: 1.25;
  text-wrap: balance;
}

.sec-text p {
  margin: 0;
  color: var(--text-secondary);
  line-height: 1.65;
}

.sec-text ul {
  margin: var(--space-lg) 0 0;
  padding: 0;
  list-style: none;
  display: flex;
  flex-direction: column;
  gap: var(--space-sm);
}

.sec-text li {
  position: relative;
  padding-left: 1.5rem;
  color: var(--site-ink-3);
  font-size: 0.94rem;
  line-height: 1.5;
}

/* Puce dessinée plutôt qu'un caractère : elle s'aligne sur la première ligne
   quel que soit le corps de texte, ce qu'un `•` ne fait pas. */
.sec-text li::before {
  content: "";
  position: absolute;
  left: 0;
  top: 0.55em;
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: var(--accent);
}

.sec-media {
  margin: 0;
  border-radius: var(--radius-xl);
  overflow: hidden;
  border: 1px solid var(--border);
}

.sec-media img {
  display: block;
  width: 100%;
  height: auto;
  aspect-ratio: 4 / 3;
  object-fit: cover;
}

@media (max-width: 768px) {
  .sec {
    grid-template-columns: 1fr;
  }

  /* Sur une colonne, l'alternance n'a plus de sens : l'image passerait
     au-dessus du titre une fois sur deux. */
  .sec.reverse .sec-text {
    order: 0;
  }
}
</style>
