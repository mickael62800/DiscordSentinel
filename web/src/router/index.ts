import type { RouteRecordRaw } from "vue-router";

// Pages eagerly chargees : critiques au boot (login flow, callback OAuth).
// Tout le reste est lazy-loaded -> chaque page = un chunk separe, charge
// uniquement a la 1ere navigation. Reduit ~70-80% le bundle initial.
import LoginPage from "../components/pages/LoginPage.vue";
import AuthCallbackPage from "../components/pages/AuthCallbackPage.vue";
import DashboardPage from "../components/pages/DashboardPage.vue";

export const routes: RouteRecordRaw[] = [
  // ── Public / boot critique (eager) ──
  { path: "/login", name: "login", component: LoginPage, meta: { public: true } },
  { path: "/auth/callback", name: "auth-callback", component: AuthCallbackPage, meta: { public: true } },

  // Accueil PUBLIC du site communautaire : visible sans connexion, rendu hors
  // de MainLayout. Le back-office demarre desormais a /dashboard.
  {
    path: "/",
    name: "public-home",
    component: () => import("@/components/pages/PublicHomePage.vue"),
    meta: { public: true },
  },

  // Dashboard reste eager : c'est la 1ere page apres login, autant l'avoir
  // dans le bundle initial pour eviter un flash de loader. Le nom de route
  // reste "dashboard" : toutes les redirections internes continuent de
  // fonctionner malgre le changement de chemin.
  { path: "/dashboard", name: "dashboard", component: DashboardPage },

  // ── Stats / Audit ──
  // Statistiques : serveur + modération réunies en onglets (StatsHubPage).
  { path: "/stats", name: "stats", component: () => import("../components/pages/StatsHubPage.vue") },
  { path: "/modstats", redirect: "/stats" },
  // Observabilité : journaux métier + système + audit réunis en onglets
  // (ObservabilityHubPage). Les trois chemins pointent le même hub, l'onglet
  // actif est dérivé de l'URL (chemins bookmarkables).
  { path: "/logs", name: "logs", component: () => import("../components/pages/ObservabilityHubPage.vue") },
  { path: "/system-logs", name: "system-logs", component: () => import("../components/pages/ObservabilityHubPage.vue") },
  { path: "/audit", name: "audit", component: () => import("../components/pages/ObservabilityHubPage.vue") },
  { path: "/name-history", name: "name-history", component: () => import("../components/pages/NameHistoryPage.vue") },

  // ── Modération ──
  { path: "/moderation", name: "moderation", component: () => import("../components/pages/ModerationHubPage.vue") },
  { path: "/rules", name: "rules", component: () => import("../components/pages/RulesPage.vue") },
  // Onglets embarques dans le hub Moderation (/moderation) : plus lies nulle
  // part directement. Redirection pour ne pas casser d'eventuels vieux liens.
  { path: "/strikes", redirect: "/moderation" },
  { path: "/notes", redirect: "/moderation" },
  { path: "/reminders", redirect: "/moderation" },
  { path: "/evidence", redirect: "/moderation" },
  { path: "/review", redirect: "/moderation" },
  { path: "/automod", name: "automod", component: () => import("../components/pages/AutomodPage.vue") },

  // ── Sécurité ──
  { path: "/security", name: "security", component: () => import("../components/pages/SecurityPage.vue") },

  // ── Communauté ──
  { path: "/welcome", name: "welcome", component: () => import("../components/pages/WelcomePage.vue") },
  { path: "/rotation-dashboard", name: "rotation-dashboard", component: () => import("../components/pages/RotationDashboardPage.vue") },
  { path: "/announcements", name: "announcements", component: () => import("../components/pages/AnnouncementsPage.vue") },
  { path: "/confessions", name: "confessions", component: () => import("../components/pages/ConfessionsPage.vue") },
  { path: "/tickets", name: "tickets", component: () => import("../components/pages/TicketsPage.vue") },
  // Vocaux : salons + thèmes réunis en onglets (VoiceHubPage).
  { path: "/voice-channels", name: "voice-channels", component: () => import("../components/pages/VoiceHubPage.vue") },
  { path: "/voice-themes", redirect: "/voice-channels" },
  // Rôles : panneaux + rôles Discord réunis en onglets (RolesHubPage). Les deux
  // chemins pointent le même hub, qui choisit l'onglet selon l'URL (le lien
  // croisé "Voir tous les rôles" reste fonctionnel).
  { path: "/role-panels", name: "role-panels", component: () => import("../components/pages/RolesHubPage.vue") },
  { path: "/role-panels/new", name: "role-panel-new", component: () => import("../components/pages/RolePanelEditPage.vue") },
  { path: "/discord-roles", name: "discord-roles", component: () => import("../components/pages/RolesHubPage.vue") },
  // Niveaux : classement + configuration réunis en onglets (LevelsHubPage).
  { path: "/levels", name: "levels", component: () => import("../components/pages/LevelsHubPage.vue") },
  { path: "/levels-config", redirect: "/levels" },
  { path: "/sponsorships", name: "sponsorships", component: () => import("../components/pages/SponsorshipsPage.vue") },
  { path: "/temp-roles", name: "temp-roles", component: () => import("../components/pages/TempRolesPage.vue") },
  { path: "/members", name: "members", component: () => import("../components/pages/MembersPage.vue") },
  { path: "/watched-users", redirect: "/members" },

  // Un journal par nature d'evenement (cf. EVENT_CATEGORIES). Tous servis par
  // le meme composant, qui lit la categorie dans le chemin.
  {
    path: "/journal",
    name: "journal",
    component: () => import("@/components/pages/EventLogPage.vue"),
  },
  {
    path: "/journal/:category",
    name: "journal-category",
    component: () => import("@/components/pages/EventLogPage.vue"),
  },

  // ── Jeux ──
  // Univers Nexus : backend distinct (nexus-api) derriere la passerelle
  // /nexus-api/. L'acces est garde cote serveur par le gate `nexus.access`.
  {
    path: "/nexus/servers",
    name: "nexus-servers",
    component: () => import("@/components/pages/NexusServersPage.vue"),
  },
  {
    path: "/nexus/economie",
    name: "nexus-economy",
    component: () => import("@/components/pages/NexusEconomyPage.vue"),
  },
  {
    path: "/nexus/coude",
    name: "nexus-coude",
    component: () => import("@/components/pages/NexusCoudePage.vue"),
  },
  {
    path: "/nexus/config",
    name: "nexus-config",
    component: () => import("@/components/pages/NexusConfigPage.vue"),
  },

  // ── Configuration / Admin ──
  { path: "/component-config", name: "component-config", component: () => import("../components/pages/ComponentConfigPage.vue") },
  { path: "/rbac", name: "rbac", component: () => import("../components/pages/RbacPage.vue") },
  { path: "/system/operations", name: "system-ops", component: () => import("../components/pages/SystemOpsPage.vue") },
  { path: "/server-health", name: "server-health", component: () => import("../components/pages/ServerHealthPage.vue") },
  { path: "/alert-rules", name: "alert-rules", component: () => import("../components/pages/AlertRulesPage.vue") },
  { path: "/server-security", name: "server-security", component: () => import("../components/pages/ServerSecurityPage.vue") },
  { path: "/guild-backup", name: "guild-backup", component: () => import("../components/pages/GuildBackupPage.vue") },
  { path: "/ai-dataset", name: "ai-dataset", component: () => import("../components/pages/AiDatasetPage.vue") },
];
