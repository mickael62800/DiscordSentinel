<script setup lang="ts">
import SectionCard from "../molecules/SectionCard.vue";

// Sections affichées sur la page d accueil. La clé `sectionKey` est
// stable et destinée a etre utilisee par le RBAC pour autoriser ou non
// l acces a chaque tuile selon l utilisateur.
type Section = {
  key: string;
  path: string;
  label: string;
  icon: string;
};

const sections: Section[] = [
  { key: "general.stats", path: "/stats", label: "Statistiques serveur", icon: "bar-chart-2" },
  { key: "general.modstats", path: "/modstats", label: "Statistiques admin", icon: "bar-chart-2" },

  { key: "moderation.hub", path: "/moderation", label: "Moderation", icon: "gavel" },
  { key: "moderation.members", path: "/members", label: "Membres", icon: "users" },
  { key: "moderation.rules", path: "/rules", label: "Regles", icon: "shield" },
  { key: "moderation.strikes", path: "/strikes", label: "Strikes", icon: "alert-triangle" },
  { key: "moderation.notes", path: "/notes", label: "Notes", icon: "edit-3" },
  { key: "moderation.reminders", path: "/reminders", label: "Reminders", icon: "clock" },
  { key: "moderation.evidence", path: "/evidence", label: "Preuves", icon: "paperclip" },
  { key: "moderation.review", path: "/review", label: "Reviews", icon: "check-circle" },
  { key: "moderation.name-history", path: "/name-history", label: "Historique pseudos", icon: "user-x" },

  { key: "community.welcome", path: "/welcome", label: "Bienvenue", icon: "user-plus" },
  { key: "community.tickets", path: "/tickets", label: "Tickets", icon: "ticket" },
  { key: "community.voice-channels", path: "/voice-channels", label: "Vocaux", icon: "mic" },
  { key: "community.voice-themes", path: "/voice-themes", label: "Themes vocaux", icon: "layers" },
  { key: "community.role-panels", path: "/role-panels", label: "Roles", icon: "users" },
  { key: "community.levels", path: "/levels", label: "Niveaux", icon: "trending-up" },
  { key: "community.levels-config", path: "/levels-config", label: "Niveaux config", icon: "sliders" },
  { key: "community.sponsorships", path: "/sponsorships", label: "Parrainages", icon: "user-check" },
  { key: "community.temp-roles", path: "/temp-roles", label: "Roles temp.", icon: "clock" },

  { key: "security.hub", path: "/security", label: "Securite", icon: "zap" },
  { key: "security.automod", path: "/automod", label: "Automod", icon: "shield" },
  { key: "security.audit", path: "/audit", label: "Audit", icon: "clipboard" },

  { key: "logs.journal", path: "/logs", label: "Journaux", icon: "list" },

  { key: "games.hub", path: "/games", label: "Jeux", icon: "layers" },
  { key: "games.coude", path: "/coude", label: "Coup de Coude", icon: "zap" },
  { key: "games.coude-social", path: "/coude/social", label: "Coude social", icon: "users" },
  { key: "games.blackjack", path: "/blackjack", label: "Blackjack", icon: "layers" },
  { key: "games.slot", path: "/slot", label: "Slot machine", icon: "dollar-sign" },
  { key: "games.wheel", path: "/wheel", label: "Roue du Destin", icon: "refresh-cw" },
  { key: "games.wallet", path: "/wallet", label: "Wallet", icon: "dollar-sign" },
  { key: "games.tournaments", path: "/tournaments", label: "Tournoi hebdo", icon: "zap" },
  { key: "games.taunts", path: "/taunts", label: "Railleries", icon: "zap" },

  { key: "config.components", path: "/component-config", label: "Composants", icon: "cpu" },
  { key: "config.rbac", path: "/rbac", label: "Acces RBAC", icon: "shield" },
  { key: "config.system-ops", path: "/system/operations", label: "System ops", icon: "activity" },
  { key: "config.settings", path: "/settings", label: "Parametres", icon: "settings" },
];
</script>

<template>
  <div class="home">
    <header class="dash-hero">
      <img src="/logo.png" alt="DiscordSentinel" class="hero-logo" />
      <div class="hero-text">
        <h1>DiscordSentinel</h1>
        <p>Panneau d'administration unifié — modération, communauté, jeux.</p>
      </div>
    </header>
    <div class="section-grid">
      <SectionCard
        v-for="s in sections"
        :key="s.key"
        :path="s.path"
        :label="s.label"
        :icon="s.icon"
        :section-key="s.key"
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
  display: flex;
  align-items: center;
  gap: 20px;
  padding: 24px 28px;
  margin-bottom: 24px;
  background: linear-gradient(135deg,
    color-mix(in srgb, var(--accent) 12%, transparent),
    color-mix(in srgb, var(--accent-alt, var(--accent)) 6%, transparent));
  border: 1px solid var(--border);
  border-radius: 16px;
}
.hero-logo {
  width: 84px;
  height: 84px;
  border-radius: 18px;
  object-fit: contain;
  filter: drop-shadow(0 6px 16px rgba(0, 0, 0, 0.4));
  flex-shrink: 0;
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
