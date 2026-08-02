import { computed, type Ref } from "vue";
import { useBotEnabledStatus } from "@/composables/useBotEnabledStatus";
import { useComponentVisibility } from "@/composables/useComponentVisibility";

/// Tile affichee sur la page d accueil.
///
/// `requiredBot` : si defini, la tuile est cachee quand ce bot est
///   desactive pour la guild courante.
/// `requiredAnyBot` : si defini, la tuile est cachee uniquement quand
///   TOUS ces bots sont desactives (visible si au moins un actif).
/// Univers applicatif d'une entree. Deux produits distincts partagent ce
/// dashboard : Sentinel (moderation/communaute) et Nexus (plateforme jeux).
/// La barre laterale n'affiche que l'univers courant.
export type Universe = "sentinel" | "nexus";

export type DashboardSection = {
  key: string;
  path: string;
  label: string;
  icon: string;
  requiredBot?: string;
  requiredAnyBot?: string[];
  /// Absent = "sentinel" (l'immense majorite des entrees existantes).
  universe?: Universe;
};

const ALL_SECTIONS: DashboardSection[] = [
  // ── Plateforme jeux Nexus ──
  // Backend distinct (nexus-api), accessible via la passerelle /nexus-api/.
  // L'acces est garde par le gate RBAC `nexus.access` cote serveur : masquer
  // ces entrees ne suffirait pas, nexus-api n'a aucun controle de role.
  // ── Journaux (un par ancien salon de logs Discord) ──
  { key: "logs.journal.members", path: "/journal/membres", label: "Journal — Membres", icon: "users" },
  { key: "logs.journal.profiles", path: "/journal/profils", label: "Journal — Profils et roles", icon: "user-check" },
  { key: "logs.journal.voice", path: "/journal/vocal", label: "Journal — Vocal", icon: "mic" },
  { key: "logs.journal.messages", path: "/journal/messages", label: "Journal — Messages", icon: "list" },
  { key: "logs.journal.server", path: "/journal/serveur", label: "Journal — Serveur", icon: "server" },
  { key: "logs.journal.admin", path: "/journal/commandes", label: "Journal — Commandes admin", icon: "shield" },
  { key: "logs.journal.anomalies", path: "/journal/anomalies", label: "Journal — Anomalies", icon: "zap" },
  { key: "logs.journal.moderation", path: "/journal/moderation", label: "Journal — Moderation", icon: "shield" },
  {
    key: "nexus.servers",
    path: "/nexus/servers",
    label: "Serveurs de jeu",
    icon: "server",
    universe: "nexus",
  },
  {
    key: "nexus.economy",
    path: "/nexus/economie",
    label: "Economie",
    icon: "trending-up",
    universe: "nexus",
  },
  {
    key: "nexus.coude",
    path: "/nexus/coude",
    label: "Coude",
    icon: "gavel",
    universe: "nexus",
  },
  {
    key: "nexus.config",
    path: "/nexus/config",
    label: "Configuration",
    icon: "sliders",
    universe: "nexus",
  },
  // Statistiques serveur + modération réunies (onglets). Visible si au moins un
  // des deux bots concernés est actif.
  { key: "general.stats", path: "/stats", label: "Statistiques", icon: "bar-chart-2", requiredAnyBot: ["audit-bot", "moderation-bot"] },

  { key: "moderation.hub", path: "/moderation", label: "Modération", icon: "gavel", requiredBot: "moderation-bot" },
  { key: "moderation.members", path: "/members", label: "Membres", icon: "users" },
  { key: "moderation.rules", path: "/rules", label: "Règles", icon: "shield", requiredBot: "moderation-bot" },
  { key: "moderation.name-history", path: "/name-history", label: "Historique pseudos", icon: "user-x", requiredBot: "audit-bot" },

  { key: "community.welcome", path: "/welcome", label: "Bienvenue", icon: "user-plus", requiredBot: "welcome-bot" },
  { key: "community.announcements", path: "/announcements", label: "Annonces planifiées", icon: "clock" },
  // Ce qui alimente l'espace membre du site : nouvelles, sondages, membre du
  // mois, modération des annonces de recherche de joueurs.
  { key: "community.life", path: "/vie-communaute", label: "Vie de la communauté", icon: "heart" },
  { key: "community.confessions", path: "/confessions", label: "Confessions", icon: "edit-3" },
  { key: "community.tickets", path: "/tickets", label: "Tickets", icon: "ticket", requiredBot: "ticket-bot" },
  { key: "community.voice-channels", path: "/voice-channels", label: "Vocaux", icon: "mic", requiredBot: "voice-bot" },
  { key: "community.role-panels", path: "/role-panels", label: "Panneaux de rôles", icon: "users", requiredBot: "community-bot" },
  { key: "community.levels", path: "/levels", label: "Niveaux", icon: "trending-up", requiredBot: "progression-bot" },
  { key: "community.sponsorships", path: "/sponsorships", label: "Parrainages", icon: "user-check", requiredBot: "community-bot" },
  { key: "community.temp-roles", path: "/temp-roles", label: "Rôles temporaires", icon: "clock", requiredBot: "community-bot" },

  { key: "security.hub", path: "/security", label: "Menaces & alertes", icon: "zap", requiredBot: "security-bot" },
  { key: "security.automod", path: "/automod", label: "Automod", icon: "shield", requiredBot: "automod-bot" },

  { key: "rotation.dashboard", path: "/rotation-dashboard", label: "Admin tournant", icon: "users", requiredBot: "rotation-bot" },

  // Observabilité : journaux métier + système + audit réunis (onglets).
  { key: "logs.system", path: "/system-logs", label: "Logs techniques", icon: "cpu" },


  { key: "config.components", path: "/component-config", label: "Composants", icon: "cpu" },
  { key: "config.rbac", path: "/rbac", label: "Accès RBAC", icon: "shield" },
  { key: "config.system-ops", path: "/system/operations", label: "Opérations système", icon: "activity" },
  { key: "config.server-health", path: "/server-health", label: "État serveur", icon: "server" },
  { key: "config.alert-rules", path: "/alert-rules", label: "Règles d'alerte", icon: "zap" },
  { key: "config.server-security", path: "/server-security", label: "Sécurité serveur", icon: "shield" },
  { key: "config.guild-backup", path: "/guild-backup", label: "Sauvegardes serveur", icon: "save" },
  { key: "config.ai-dataset", path: "/ai-dataset", label: "Dataset IA", icon: "cpu" },
  // Module sans page dediee : la tuile ouvre directement sa config (lien
  // profond ?bot= gere par ComponentConfigPage). Masquee si le bot est off.
  { key: "config.nasa-apod", path: "/component-config?bot=nasa-apod-bot", label: "Photo de l'espace", icon: "image", requiredBot: "nasa-apod-bot" },
];

/// Alias : sous-chemins de hubs (fusionnes) qui partagent la gouvernance RBAC
/// de leur hub — ils n'ont pas de tuile propre mais restent atteignables par URL.
const PATH_RBAC_ALIASES: Record<string, string> = {
  // Sous-pages des serveurs de jeu : meme droit que la liste.
  "/nexus/servers/nouveau": "nexus.servers",
  "/discord-roles": "community.role-panels",
  "/voice-themes": "community.voice-channels",
  "/levels-config": "community.levels",
  "/modstats": "general.stats",
};

/// Cle RBAC (composant) gouvernant l'acces a un chemin de route, ou `undefined`
/// si le chemin n'est pas soumis a une restriction de role connue. Utilise par
/// le guard de navigation pour bloquer l'ouverture directe d'une page par URL
/// quand le role ne la voit pas (defense alignee sur le masquage des tuiles).
export function rbacKeyForPath(path: string): string | undefined {
  const direct = ALL_SECTIONS.find((s) => s.path === path);
  if (direct) return direct.key;
  return PATH_RBAC_ALIASES[path];
}

/// Un groupe de tuiles regroupees par domaine (prefixe de `key`).
export type DashboardGroup = {
  prefix: string;
  label: string;
  sections: DashboardSection[];
};

/// Ordre d'affichage des groupes + libelles FR. Le prefixe correspond a
/// la partie de `key` avant le premier point (ex. "community.welcome").
/// Tout prefixe non liste ici est ignore du regroupement (ne devrait pas
/// arriver ; garde-fou en cas d'ajout futur non declare).
const GROUP_ORDER: { prefix: string; label: string }[] = [
  { prefix: "general", label: "Général" },
  { prefix: "moderation", label: "Modération" },
  { prefix: "community", label: "Communauté" },
  { prefix: "security", label: "Sécurité" },
  { prefix: "rotation", label: "Administration tournante" },
  { prefix: "config", label: "Configuration" },
  { prefix: "logs", label: "Journaux" },
  // ── Univers Nexus ──
  { prefix: "nexus", label: "Plateforme jeux" },
];

/// Filtre les tuiles dashboard selon :
/// - `requiredBot` : visible seulement si le bot est actif (single dep)
/// - `requiredAnyBot` : visible si AU MOINS UN bot de la liste est actif
/// - aucun des deux : toujours visible (autonome)
/// - RBAC visibility par role (overrides BDD + defauts registry).
export function useDashboardSections(universe?: Ref<Universe>) {
  const { isBotEnabled } = useBotEnabledStatus();
  const { visible: rbacVisible } = useComponentVisibility();

  const sections = computed<DashboardSection[]>(() =>
    ALL_SECTIONS.filter((s) => {
      // Univers : une entree sans `universe` appartient a Sentinel.
      const u = s.universe ?? "sentinel";
      if (universe && u !== universe.value) return false;
      if (s.requiredBot && !isBotEnabled(s.requiredBot)) return false;
      if (s.requiredAnyBot && s.requiredAnyBot.length > 0) {
        const anyActive = s.requiredAnyBot.some((b) => isBotEnabled(b));
        if (!anyActive) return false;
      }
      if (!rbacVisible(s.key)) return false;
      return true;
    }),
  );

  /// Tuiles visibles regroupees par domaine, dans l'ordre de `GROUP_ORDER`.
  /// Les groupes vides (aucune tuile visible) sont omis.
  const groups = computed<DashboardGroup[]>(() =>
    GROUP_ORDER.map((g) => ({
      prefix: g.prefix,
      label: g.label,
      sections: sections.value.filter((s) => s.key.split(".")[0] === g.prefix),
    })).filter((g) => g.sections.length > 0),
  );

  return { sections, groups };
}
