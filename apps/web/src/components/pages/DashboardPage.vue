<script setup lang="ts">
import { computed } from "vue";
import SectionCard from "../molecules/SectionCard.vue";
import { useBotEnabledStatus } from "@/composables/useBotEnabledStatus";
import { useComponentVisibility } from "@/composables/useComponentVisibility";

// Sections affichées sur la page d accueil. La clé `sectionKey` est
// stable et destinée a etre utilisee par le RBAC pour autoriser ou non
// l acces a chaque tuile selon l utilisateur.
//
// `requiredBot` : si défini (string), la tuile est cachée quand ce bot
//   est désactivé pour la guild courante.
// `requiredAnyBot` : si défini (array), la tuile est cachée seulement
//   quand TOUS ces bots sont désactivés (visible si au moins un actif).
//   Utilisé pour Wallet (dépend de plusieurs jeux).
type Section = {
  key: string;
  path: string;
  label: string;
  icon: string;
  requiredBot?: string;
  requiredAnyBot?: string[];
};

const { isBotEnabled, disabledBots, disabledCount } = useBotEnabledStatus();

const allSections: Section[] = [
  { key: "general.stats", path: "/stats", label: "Statistiques serveur", icon: "bar-chart-2", requiredBot: "audit-bot" },
  { key: "general.modstats", path: "/modstats", label: "Statistiques admin", icon: "bar-chart-2", requiredBot: "moderation-bot" },

  { key: "moderation.hub", path: "/moderation", label: "Moderation", icon: "gavel", requiredBot: "moderation-bot" },
  { key: "moderation.members", path: "/members", label: "Membres", icon: "users" },
  { key: "moderation.rules", path: "/rules", label: "Regles", icon: "shield", requiredBot: "moderation-bot" },
  { key: "moderation.strikes", path: "/strikes", label: "Strikes", icon: "alert-triangle", requiredBot: "moderation-bot" },
  { key: "moderation.notes", path: "/notes", label: "Notes", icon: "edit-3", requiredBot: "moderation-bot" },
  { key: "moderation.reminders", path: "/reminders", label: "Reminders", icon: "clock", requiredBot: "moderation-bot" },
  { key: "moderation.evidence", path: "/evidence", label: "Preuves", icon: "paperclip", requiredBot: "moderation-bot" },
  { key: "moderation.review", path: "/review", label: "Reviews", icon: "check-circle", requiredBot: "moderation-bot" },
  { key: "moderation.name-history", path: "/name-history", label: "Historique pseudos", icon: "user-x", requiredBot: "audit-bot" },

  { key: "community.welcome", path: "/welcome", label: "Bienvenue", icon: "user-plus", requiredBot: "welcome-bot" },
  { key: "community.tickets", path: "/tickets", label: "Tickets", icon: "ticket", requiredBot: "ticket-bot" },
  { key: "community.voice-channels", path: "/voice-channels", label: "Vocaux", icon: "mic", requiredBot: "voice-bot" },
  { key: "community.voice-themes", path: "/voice-themes", label: "Themes vocaux", icon: "layers", requiredBot: "voice-bot" },
  { key: "community.role-panels", path: "/role-panels", label: "Roles", icon: "users", requiredBot: "community-bot" },
  { key: "community.levels", path: "/levels", label: "Niveaux", icon: "trending-up", requiredBot: "progression-bot" },
  { key: "community.levels-config", path: "/levels-config", label: "Niveaux config", icon: "sliders", requiredBot: "progression-bot" },
  { key: "community.sponsorships", path: "/sponsorships", label: "Parrainages", icon: "user-check", requiredBot: "community-bot" },
  { key: "community.temp-roles", path: "/temp-roles", label: "Roles temp.", icon: "clock", requiredBot: "community-bot" },

  { key: "security.hub", path: "/security", label: "Securite", icon: "zap", requiredBot: "security-bot" },
  { key: "security.automod", path: "/automod", label: "Automod", icon: "shield", requiredBot: "automod-bot" },
  { key: "security.audit", path: "/audit", label: "Audit", icon: "clipboard", requiredBot: "audit-bot" },

  { key: "logs.journal", path: "/logs", label: "Journaux", icon: "list" },

  { key: "games.hub", path: "/games", label: "Jeux", icon: "layers", requiredBot: "game-bot" },
  { key: "games.coude", path: "/coude", label: "Coup de Coude", icon: "zap", requiredBot: "coude-bot" },
  { key: "games.coude-social", path: "/coude/social", label: "Coude social", icon: "users", requiredBot: "coude-bot" },
  { key: "games.blackjack", path: "/blackjack", label: "Blackjack", icon: "layers", requiredBot: "blackjack-bot" },
  { key: "games.slot", path: "/slot", label: "Slot machine", icon: "dollar-sign", requiredBot: "slot-bot" },
  { key: "games.wheel", path: "/wheel", label: "Roue du Destin", icon: "refresh-cw", requiredBot: "wheel-bot" },
  // Wallet : visible tant qu'au moins un jeu utilisant le wallet est actif.
  // Cache uniquement si coude + blackjack + slot + wheel sont TOUS off.
  {
    key: "games.wallet",
    path: "/wallet",
    label: "Wallet",
    icon: "dollar-sign",
    requiredAnyBot: ["coude-bot", "blackjack-bot", "slot-bot", "wheel-bot"],
  },
  { key: "games.tournaments", path: "/tournaments", label: "Tournoi hebdo", icon: "zap", requiredBot: "coude-bot" },
  { key: "games.taunts", path: "/taunts", label: "Railleries", icon: "zap", requiredBot: "coude-bot" },

  { key: "config.components", path: "/component-config", label: "Composants", icon: "cpu" },
  { key: "config.rbac", path: "/rbac", label: "Acces RBAC", icon: "shield" },
  { key: "config.system-ops", path: "/system/operations", label: "System ops", icon: "activity" },
  { key: "config.server-health", path: "/server-health", label: "État serveur", icon: "server" },
  { key: "config.server-security", path: "/server-security", label: "Sécurité serveur", icon: "shield" },
  { key: "config.ai-dataset", path: "/ai-dataset", label: "Dataset IA", icon: "cpu" },
  { key: "config.settings", path: "/settings", label: "Parametres", icon: "settings" },
];

// Sections visibles selon l'etat des bots :
// - `requiredBot` : visible seulement si le bot est actif (single dep)
// - `requiredAnyBot` : visible si AU MOINS UN bot de la liste est actif
// - aucun des deux : toujours visible (autonome)
// Visibilite RBAC par role (overrides BDD + defauts registry).
// useComponentVisibility() : si la cle s.key n'est pas declaree dans le
// registry, retourne true par defaut (zero regression sur boutons existants).
const { visible: rbacVisible } = useComponentVisibility();

const sections = computed<Section[]>(() =>
  allSections.filter((s) => {
    if (s.requiredBot && !isBotEnabled(s.requiredBot)) return false;
    if (s.requiredAnyBot && s.requiredAnyBot.length > 0) {
      const anyActive = s.requiredAnyBot.some((b) => isBotEnabled(b));
      if (!anyActive) return false;
    }
    if (!rbacVisible(s.key)) return false;
    return true;
  }),
);
</script>

<template>
  <div class="home">
    <header class="dash-hero">
      <div class="hero-pattern" aria-hidden="true"></div>
      <div class="hero-gloss" aria-hidden="true"></div>
      <div class="hero-logo-wrap">
        <img src="/logo.png" alt="DiscordSentinel" class="hero-logo" />
      </div>
      <div class="hero-text">
        <h1>DiscordSentinel</h1>
        <p>Panneau d'administration unifié — modération, communauté, jeux.</p>
      </div>
    </header>

    <!-- Indicateur de composants desactives entre la banner et les boutons.
         N'apparait que s'il y a au moins 1 composant off pour la guild
         courante — sinon on garde la dashboard epuree. -->
    <router-link
      v-if="disabledCount > 0"
      to="/component-config"
      class="disabled-banner"
      :title="`Voir / réactiver dans Composants : ${disabledBots.join(', ')}`"
    >
      <span class="disabled-icon">⚠️</span>
      <span class="disabled-text">
        <strong>{{ disabledCount }}</strong>
        composant{{ disabledCount > 1 ? 's' : '' }} désactivé{{ disabledCount > 1 ? 's' : '' }}
        — certains boutons sont masqués
      </span>
      <span class="disabled-arrow">→</span>
    </router-link>

    <div class="section-grid">
      <SectionCard
        v-for="s in sections"
        :key="s.key"
        :path="s.path"
        :label="s.label"
        :icon="s.icon"
        :section-key="s.key"
        :required-bot="s.requiredBot"
      />
    </div>
  </div>
</template>

<style scoped>
.home {
  height: 100%;
  min-height: 0;
  display: flex;
  flex-direction: column;
  overflow: auto;
  width: 100%;
  max-width: 1200px;
  margin: 0 auto;
}

.dash-hero {
  position: relative;
  overflow: hidden;
  display: flex;
  align-items: center;
  gap: 20px;
  padding: 26px 30px;
  margin-bottom: 24px;
  border-radius: 16px;
  /* Mesh gradient anime : 3 radial-gradients qui flottent en arriere-plan
     pour donner une sensation vivante sans etre distrayant. */
  background:
    radial-gradient(circle at var(--mesh-x1, 20%) var(--mesh-y1, 30%),
      color-mix(in srgb, var(--accent) 35%, transparent) 0%,
      transparent 50%),
    radial-gradient(circle at var(--mesh-x2, 80%) var(--mesh-y2, 70%),
      color-mix(in srgb, var(--accent-alt, #a855f7) 30%, transparent) 0%,
      transparent 50%),
    radial-gradient(circle at var(--mesh-x3, 50%) var(--mesh-y3, 50%),
      color-mix(in srgb, #ec4899 25%, transparent) 0%,
      transparent 60%),
    linear-gradient(135deg,
      color-mix(in srgb, var(--accent) 8%, var(--bg-card)),
      color-mix(in srgb, var(--accent-alt, var(--accent)) 4%, var(--bg-card)));
  /* On anime les positions via custom properties avec @property
     pour avoir une vraie interpolation (sinon CSS ne sait pas animer
     les valeurs dans radial-gradient). Fallback : background-position. */
  animation: mesh-drift 18s ease-in-out infinite alternate;
  /* Bordure conic rotative : 1px de gradient qui tourne autour. */
  border: 1px solid transparent;
  background-clip: padding-box;
  box-shadow:
    0 0 0 1px color-mix(in srgb, var(--accent) 25%, var(--border)),
    0 4px 16px color-mix(in srgb, var(--accent) 8%, transparent);
  transition: transform 0.4s cubic-bezier(0.34, 1.56, 0.64, 1),
    box-shadow 0.4s ease;
}

/* Pattern de points discret en overlay : ajoute de la texture sans
   distraire. Tres subtil (4% d'opacite). */
.hero-pattern {
  position: absolute;
  inset: 0;
  background-image:
    radial-gradient(circle, color-mix(in srgb, var(--text-primary) 100%, transparent) 1px, transparent 1.5px);
  background-size: 18px 18px;
  opacity: 0.04;
  pointer-events: none;
  mask-image: linear-gradient(to right, transparent 0%, black 30%, black 100%);
  -webkit-mask-image: linear-gradient(to right, transparent 0%, black 30%, black 100%);
}

/* Bordure conic-gradient rotative : utilise un masque pour ne montrer
   que le pourtour. Cree un effet "neon vivant" tres subtil. */
.dash-hero::after {
  content: "";
  position: absolute;
  inset: -1px;
  border-radius: 16px;
  padding: 1px;
  background: conic-gradient(
    from var(--border-angle, 0deg),
    color-mix(in srgb, var(--accent) 80%, transparent),
    color-mix(in srgb, var(--accent-alt, #a855f7) 80%, transparent),
    color-mix(in srgb, #ec4899 80%, transparent),
    color-mix(in srgb, var(--accent) 80%, transparent)
  );
  -webkit-mask:
    linear-gradient(#fff 0 0) content-box,
    linear-gradient(#fff 0 0);
  mask:
    linear-gradient(#fff 0 0) content-box,
    linear-gradient(#fff 0 0);
  -webkit-mask-composite: xor;
  mask-composite: exclude;
  pointer-events: none;
  opacity: 0.7;
  animation: border-rotate 8s linear infinite;
}

@property --border-angle {
  syntax: "<angle>";
  initial-value: 0deg;
  inherits: false;
}
@keyframes border-rotate {
  to { --border-angle: 360deg; }
}

@keyframes mesh-drift {
  0%   { background-position: 0% 0%, 100% 100%, 50% 50%, 0 0; }
  50%  { background-position: 30% 20%, 70% 80%, 60% 40%, 0 0; }
  100% { background-position: 10% 40%, 90% 60%, 40% 60%, 0 0; }
}

.dash-hero:hover {
  transform: translateY(-2px);
  box-shadow:
    0 0 0 1px color-mix(in srgb, var(--accent) 50%, var(--border)),
    0 14px 32px color-mix(in srgb, var(--accent) 25%, transparent);
}

/* Gloss : reflet diagonal qui balaie la banner au hover. */
.hero-gloss {
  position: absolute;
  top: -50%;
  left: -75%;
  width: 35%;
  height: 200%;
  background: linear-gradient(
    115deg,
    transparent 0%,
    color-mix(in srgb, white 0%, transparent) 40%,
    color-mix(in srgb, white 22%, transparent) 50%,
    color-mix(in srgb, white 0%, transparent) 60%,
    transparent 100%
  );
  transform: skewX(-20deg);
  pointer-events: none;
  opacity: 0;
  transition: opacity 0.2s ease;
}
.dash-hero:hover .hero-gloss {
  opacity: 1;
  animation: hero-gloss-sweep 1.1s ease-out;
}
@keyframes hero-gloss-sweep {
  0%   { left: -75%; }
  100% { left: 125%; }
}

/* Boucle automatique : un balayage gloss toutes les 10 secondes
   (1.4s de sweep visible + 8.6s "off" hors viewport). Premiere passe
   declenchee 0.4s apres le chargement. */
.dash-hero::before {
  content: "";
  position: absolute;
  top: -50%;
  left: -75%;
  width: 35%;
  height: 200%;
  background: linear-gradient(
    115deg,
    transparent 0%,
    color-mix(in srgb, white 0%, transparent) 40%,
    color-mix(in srgb, white 18%, transparent) 50%,
    color-mix(in srgb, white 0%, transparent) 60%,
    transparent 100%
  );
  transform: skewX(-20deg);
  pointer-events: none;
  animation: hero-gloss-loop 10s ease-out 0.4s infinite;
}

@keyframes hero-gloss-loop {
  /* 0%-14% : sweep visible (~1.4s sur 10s).
     14%-100% : reste off-screen pour creer la pause. */
  0%   { left: -75%; }
  14%  { left: 125%; }
  100% { left: 125%; }
}

@media (prefers-reduced-motion: reduce) {
  .dash-hero,
  .dash-hero:hover { transform: none; animation: none; }
  .hero-gloss { display: none; }
  .dash-hero::before,
  .dash-hero::after { animation: none; }
  .hero-logo-wrap::before { animation: none; opacity: 0.6; transform: none; }
  .hero-text h1 {
    animation: none;
    background: none;
    -webkit-text-fill-color: var(--text-primary);
    color: var(--text-primary);
  }
}

/* Halo pulsant derriere le logo : fait "exister" le logo dans la mesh. */
.hero-logo-wrap {
  position: relative;
  flex-shrink: 0;
  z-index: 1;
}
.hero-logo {
  width: 84px;
  height: 84px;
  border-radius: 18px;
  object-fit: contain;
  filter: drop-shadow(0 6px 16px rgba(0, 0, 0, 0.4));
  position: relative;
  z-index: 2;
  transition: transform 0.4s cubic-bezier(0.34, 1.56, 0.64, 1);
}
.hero-logo-wrap::before {
  content: "";
  position: absolute;
  inset: -12px;
  border-radius: 50%;
  background: radial-gradient(circle,
    color-mix(in srgb, var(--accent) 45%, transparent) 0%,
    transparent 65%);
  filter: blur(8px);
  z-index: 1;
  animation: halo-pulse 3.5s ease-in-out infinite;
}
@keyframes halo-pulse {
  0%, 100% { opacity: 0.55; transform: scale(0.92); }
  50%      { opacity: 0.95; transform: scale(1.1); }
}
.dash-hero:hover .hero-logo {
  transform: scale(1.06) rotate(-3deg);
}

.hero-text {
  position: relative;
  z-index: 1;
}
.hero-text h1 {
  /* Gradient text + shimmer qui balaie la couleur. */
  background: linear-gradient(
    90deg,
    var(--text-primary) 0%,
    color-mix(in srgb, var(--accent) 80%, var(--text-primary)) 25%,
    var(--text-primary) 50%,
    color-mix(in srgb, var(--accent-alt, #a855f7) 80%, var(--text-primary)) 75%,
    var(--text-primary) 100%
  );
  background-size: 200% auto;
  -webkit-background-clip: text;
  background-clip: text;
  -webkit-text-fill-color: transparent;
  color: transparent;
  animation: text-shimmer 6s linear infinite;
  letter-spacing: 0.5px;
}
@keyframes text-shimmer {
  0%   { background-position: 200% center; }
  100% { background-position: -200% center; }
}
.hero-text h1 {
  margin: 0 0 6px;
  font-size: 1.6rem;
  font-weight: 700;
}
.hero-text p {
  margin: 0;
  color: var(--text-muted, #888);
  font-size: 0.95rem;
}

/* Bandeau "X composants desactives" — discret, cliquable vers /component-config */
.disabled-banner {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 14px;
  margin: 0 0 14px;
  background: color-mix(in srgb, var(--warning, #e67e22) 12%, transparent);
  border: 1px solid color-mix(in srgb, var(--warning, #e67e22) 35%, var(--border));
  border-radius: 8px;
  color: var(--text-primary);
  text-decoration: none;
  font-size: 13px;
  transition: background-color 0.2s ease, transform 0.2s ease, border-color 0.2s ease;
}
.disabled-banner:hover {
  background: color-mix(in srgb, var(--warning, #e67e22) 20%, transparent);
  border-color: var(--warning, #e67e22);
  transform: translateY(-1px);
}
.disabled-icon { font-size: 14px; flex-shrink: 0; }
.disabled-text { flex: 1; }
.disabled-text strong {
  color: var(--warning, #e67e22);
  font-weight: 700;
}
.disabled-arrow {
  font-size: 16px;
  color: var(--warning, #e67e22);
  font-weight: 700;
  flex-shrink: 0;
}

.section-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
  grid-auto-rows: 120px;
  gap: 12px;
}

@media (max-width: 640px) {
  .dash-hero {
    flex-direction: column;
    text-align: center;
    padding: 20px;
  }
}
</style>
