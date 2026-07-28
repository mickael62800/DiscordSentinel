<script setup lang="ts">
import SectionIcon from "../atoms/SectionIcon.vue";

const props = defineProps<{
  path: string;
  label: string;
  icon: string;
  sectionKey: string;
  requiredBot?: string;
  /** Si la feature concerne plusieurs bots (ex: railleries = blackjack),
   *  on liste tout. Le badge affiche tous les noms joints par " + ". */
  requiredAnyBot?: string[];
}>();

// Le theme est derive du prefixe de la cle (ex: "moderation.strikes" -> "moderation").
const theme = props.sectionKey.split(".")[0] || "default";

/** Convertit "moderation-bot" → "moderation" / "blackjack-bot" → "bj" pour le badge. */
const SHORT_OVERRIDES: Record<string, string> = {
  "blackjack-bot": "bj",
};
function shortBotName(name: string): string {
  return SHORT_OVERRIDES[name] ?? name.replace(/-bot$/, "");
}

/** Texte du badge : single bot OU liste joinde de bots (railleries-like). */
const badgeText = (() => {
  if (props.requiredAnyBot && props.requiredAnyBot.length > 0) {
    return props.requiredAnyBot.map(shortBotName).join("+");
  }
  if (props.requiredBot) return shortBotName(props.requiredBot);
  return "";
})();
</script>

<template>
  <router-link :to="path" :class="['section-card', `theme-${theme}`]" :data-section-key="sectionKey">
    <div class="gloss" aria-hidden="true"></div>
    <span
      v-if="badgeText"
      class="component-tag"
      :title="requiredAnyBot && requiredAnyBot.length > 0
        ? `Composants : ${requiredAnyBot.join(', ')}`
        : `Composant requis : ${requiredBot}`"
    >{{ badgeText }}</span>
    <div class="icon-wrap">
      <SectionIcon :name="icon" />
    </div>
    <span class="label">{{ label }}</span>
  </router-link>
</template>

<style scoped>
.section-card {
  --theme-color: var(--accent);
  --theme-bg: color-mix(in srgb, var(--theme-color) 10%, var(--bg-card));
  --theme-border: color-mix(in srgb, var(--theme-color) 35%, var(--border));

  position: relative;
  overflow: hidden;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 8px;
  padding: 10px 8px;
  height: 100%;
  min-height: 70px;
  border-radius: 10px;
  background-color: var(--theme-bg);
  border: 1px solid var(--theme-border);
  color: var(--text-secondary);
  text-decoration: none;
  text-align: center;
  cursor: pointer;
  transition: transform 0.35s cubic-bezier(0.34, 1.56, 0.64, 1),
    border-color 0.25s ease,
    color 0.25s ease,
    background-color 0.25s ease,
    box-shadow 0.35s ease;
  /* Entree en cascade : chaque carte apparait avec un delai */
  opacity: 0;
  animation: card-enter 0.5s ease-out forwards;
}

/* Stagger : les premieres ~24 cartes ont des delais croissants.
   Au-dela, toutes apparaissent en meme temps (cap pour eviter les pages
   plus longues a charger). */
.section-card:nth-child(1)  { animation-delay: 0.02s; }
.section-card:nth-child(2)  { animation-delay: 0.05s; }
.section-card:nth-child(3)  { animation-delay: 0.08s; }
.section-card:nth-child(4)  { animation-delay: 0.11s; }
.section-card:nth-child(5)  { animation-delay: 0.14s; }
.section-card:nth-child(6)  { animation-delay: 0.17s; }
.section-card:nth-child(7)  { animation-delay: 0.20s; }
.section-card:nth-child(8)  { animation-delay: 0.23s; }
.section-card:nth-child(9)  { animation-delay: 0.26s; }
.section-card:nth-child(10) { animation-delay: 0.29s; }
.section-card:nth-child(11) { animation-delay: 0.32s; }
.section-card:nth-child(12) { animation-delay: 0.35s; }
.section-card:nth-child(13) { animation-delay: 0.38s; }
.section-card:nth-child(14) { animation-delay: 0.41s; }
.section-card:nth-child(15) { animation-delay: 0.44s; }
.section-card:nth-child(16) { animation-delay: 0.47s; }
.section-card:nth-child(17) { animation-delay: 0.50s; }
.section-card:nth-child(18) { animation-delay: 0.53s; }
.section-card:nth-child(19) { animation-delay: 0.56s; }
.section-card:nth-child(20) { animation-delay: 0.59s; }
.section-card:nth-child(n+21) { animation-delay: 0.62s; }

@keyframes card-enter {
  0% {
    opacity: 0;
    transform: translateY(12px) scale(0.95);
  }
  100% {
    opacity: 1;
    transform: translateY(0) scale(1);
  }
}

.section-card:hover {
  transform: translateY(-4px) scale(1.03);
  border-color: var(--theme-color);
  color: var(--text-primary);
  background-color: color-mix(in srgb, var(--theme-color) 22%, var(--bg-card));
  box-shadow:
    0 10px 24px color-mix(in srgb, var(--theme-color) 35%, transparent),
    0 0 0 1px color-mix(in srgb, var(--theme-color) 50%, transparent);
}

.section-card:active {
  transform: translateY(-1px) scale(1.0);
  transition-duration: 0.1s;
}

/* Effet gloss/shine : reflet diagonal qui balaie la carte au hover */
.gloss {
  position: absolute;
  top: -50%;
  left: -75%;
  width: 50%;
  height: 200%;
  background: linear-gradient(
    115deg,
    transparent 0%,
    color-mix(in srgb, var(--theme-color) 0%, transparent) 40%,
    color-mix(in srgb, white 25%, transparent) 50%,
    color-mix(in srgb, var(--theme-color) 0%, transparent) 60%,
    transparent 100%
  );
  transform: skewX(-20deg);
  pointer-events: none;
  opacity: 0;
  transition: opacity 0.2s ease;
}

.section-card:hover .gloss {
  opacity: 1;
  animation: gloss-sweep 0.85s ease-out;
}

@keyframes gloss-sweep {
  0%   { left: -75%; }
  100% { left: 125%; }
}

.icon-wrap {
  width: 36px;
  height: 36px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--theme-color);
  transition: transform 0.35s cubic-bezier(0.34, 1.56, 0.64, 1),
    filter 0.25s ease;
}

.section-card:hover .icon-wrap {
  transform: scale(1.15) rotate(-4deg);
  filter: drop-shadow(0 0 8px color-mix(in srgb, var(--theme-color) 60%, transparent));
}

.label {
  font-size: 13px;
  font-weight: 600;
  line-height: 1.2;
  position: relative;
  z-index: 1;
}

/* Badge composant en haut a droite : toute petite indication discrete
   du bot requis (sans le suffixe "-bot" pour la compacite). */
.component-tag {
  position: absolute;
  top: 4px;
  right: 6px;
  z-index: 2;
  font-size: 8.5px;
  font-weight: 700;
  letter-spacing: 0.4px;
  padding: 1px 5px;
  border-radius: 4px;
  background: color-mix(in srgb, var(--theme-color) 18%, transparent);
  color: var(--theme-color);
  text-transform: uppercase;
  pointer-events: none;
  opacity: 0.75;
  transition: opacity 0.2s ease;
}
.section-card:hover .component-tag {
  opacity: 1;
}

/* Respect du reduced-motion : on coupe les animations pour ceux qui en
   ont besoin (accessibilite). */
@media (prefers-reduced-motion: reduce) {
  .section-card,
  .icon-wrap,
  .gloss {
    animation: none !important;
    transition: none !important;
  }
  .section-card { opacity: 1; transform: none; }
}

/* ── Couleurs par theme ─────────────────────── */
.theme-general    { --theme-color: #38bdf8; } /* sky    */
.theme-moderation { --theme-color: #f43f5e; } /* rose   */
.theme-community  { --theme-color: #22c55e; } /* green  */
.theme-security   { --theme-color: #f59e0b; } /* amber  */
.theme-logs       { --theme-color: #a855f7; } /* purple */
.theme-games      { --theme-color: #ec4899; } /* pink   */
.theme-config     { --theme-color: #64748b; } /* slate  */
</style>
