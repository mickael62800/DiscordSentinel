import { computed } from "vue";
import { useBotEnabledStatus } from "@/composables/useBotEnabledStatus";
import { useComponentVisibility } from "@/composables/useComponentVisibility";

/// Tile affichee sur la page d accueil.
///
/// `requiredBot` : si defini, la tuile est cachee quand ce bot est
///   desactive pour la guild courante.
/// `requiredAnyBot` : si defini, la tuile est cachee uniquement quand
///   TOUS ces bots sont desactives (visible si au moins un actif).
///   Utilise pour Wallet (depend de plusieurs jeux).
export type DashboardSection = {
  key: string;
  path: string;
  label: string;
  icon: string;
  requiredBot?: string;
  requiredAnyBot?: string[];
};

const ALL_SECTIONS: DashboardSection[] = [
  { key: "general.stats", path: "/stats", label: "Statistiques serveur", icon: "bar-chart-2", requiredBot: "audit-bot" },
  { key: "general.modstats", path: "/modstats", label: "Statistiques admin", icon: "bar-chart-2", requiredBot: "moderation-bot" },

  { key: "moderation.hub", path: "/moderation", label: "Moderation", icon: "gavel", requiredBot: "moderation-bot" },
  { key: "moderation.members", path: "/members", label: "Membres", icon: "users" },
  { key: "moderation.rules", path: "/rules", label: "Regles", icon: "shield", requiredBot: "moderation-bot" },
  { key: "moderation.name-history", path: "/name-history", label: "Historique pseudos", icon: "user-x", requiredBot: "audit-bot" },

  { key: "community.welcome", path: "/welcome", label: "Bienvenue", icon: "user-plus", requiredBot: "welcome-bot" },
  { key: "community.announcements", path: "/announcements", label: "Annonces planifiées", icon: "clock" },
  { key: "community.confessions", path: "/confessions", label: "Confessions", icon: "edit-3" },
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
  // Railleries : feature partagee entre Coude et Blackjack (TauntEvent
  // emis par les deux moteurs de jeu, canal Discord unique). Visible
  // si AU MOINS un des deux bots est actif.
  {
    key: "games.taunts",
    path: "/taunts",
    label: "Railleries",
    icon: "zap",
    requiredAnyBot: ["coude-bot", "blackjack-bot"],
  },

  { key: "games.portal", path: "/game-portal", label: "Game Portal", icon: "server", requiredBot: "game-portal" },

  { key: "config.components", path: "/component-config", label: "Composants", icon: "cpu" },
  { key: "config.rbac", path: "/rbac", label: "Acces RBAC", icon: "shield" },
  { key: "config.system-ops", path: "/system/operations", label: "System ops", icon: "activity" },
  { key: "config.server-health", path: "/server-health", label: "État serveur", icon: "server" },
  { key: "config.server-security", path: "/server-security", label: "Sécurité serveur", icon: "shield" },
  // Logs systeme : place dans le groupe admin/config (bots/workers/API/WS),
  // pas dans le groupe Journaux (qui est metier Discord uniquement).
  { key: "config.system-logs", path: "/system-logs", label: "Logs système", icon: "list" },
  { key: "config.ai-dataset", path: "/ai-dataset", label: "Dataset IA", icon: "cpu" },
];

/// Filtre les tuiles dashboard selon :
/// - `requiredBot` : visible seulement si le bot est actif (single dep)
/// - `requiredAnyBot` : visible si AU MOINS UN bot de la liste est actif
/// - aucun des deux : toujours visible (autonome)
/// - RBAC visibility par role (overrides BDD + defauts registry).
export function useDashboardSections() {
  const { isBotEnabled } = useBotEnabledStatus();
  const { visible: rbacVisible } = useComponentVisibility();

  const sections = computed<DashboardSection[]>(() =>
    ALL_SECTIONS.filter((s) => {
      if (s.requiredBot && !isBotEnabled(s.requiredBot)) return false;
      if (s.requiredAnyBot && s.requiredAnyBot.length > 0) {
        const anyActive = s.requiredAnyBot.some((b) => isBotEnabled(b));
        if (!anyActive) return false;
      }
      if (!rbacVisible(s.key)) return false;
      return true;
    }),
  );

  return { sections };
}
